use std::{collections::HashMap, sync::Arc};

use godot::{
    bind::GodotClass,
    builtin::{meta::ToGodot, Dictionary, GString, Variant, VariantArray},
    engine::{
        animation::TrackType, global::Error, node::ProcessMode, AnimatableBody3D, Animation,
        AnimationLibrary, AnimationPlayer, CollisionShape3D, ConcavePolygonShape3D, GltfDocument,
        GltfState, MeshInstance3D, Node, Node3D, NodeExt, StaticBody3D,
    },
    obj::{Gd, InstanceId},
};
use tokio::io::{AsyncReadExt, AsyncSeekExt};

use super::{
    content_mapping::ContentMappingAndUrlRef, content_provider::ContentProviderContext,
    download::fetch_resource_or_wait, file_string::get_base_dir,
    thread_safety::GodotSingleThreadSafety,
};

pub async fn internal_load_gltf(
    file_path: String,
    content_mapping: ContentMappingAndUrlRef,
    ctx: ContentProviderContext,
) -> Result<(Gd<Node3D>, GodotSingleThreadSafety), anyhow::Error> {
    let base_path = Arc::new(get_base_dir(&file_path));

    let file_hash = content_mapping
        .get_hash(file_path.as_str())
        .ok_or(anyhow::Error::msg("File not found in the content mappings"))?;

    let url = format!("{}{}", content_mapping.base_url, file_hash);
    let absolute_file_path = format!("{}{}", ctx.content_folder, file_hash);
    fetch_resource_or_wait(&url, file_hash, &absolute_file_path, ctx.clone())
        .await
        .map_err(anyhow::Error::msg)?;

    let dependencies = get_dependencies(&absolute_file_path)
        .await?
        .into_iter()
        .map(|dep| {
            let full_path = if base_path.is_empty() {
                dep.clone()
            } else {
                format!("{}/{}", base_path, dep)
            };

            let item = content_mapping.get_hash(full_path.as_str()).cloned();
            (dep, item)
        })
        .collect::<Vec<(String, Option<String>)>>();

    if dependencies.iter().any(|(_, hash)| hash.is_none()) {
        return Err(anyhow::Error::msg(
            "There are some missing dependencies in the gltf".to_string(),
        ));
    }

    let dependencies_hash = dependencies
        .into_iter()
        .map(|(file_path, hash)| (file_path, hash.unwrap()))
        .collect::<Vec<(String, String)>>();

    let futures = dependencies_hash.iter().map(|(_, dependency_file_hash)| {
        let ctx = ctx.clone();
        let content_mapping = content_mapping.clone();
        async move {
            let url = format!("{}{}", content_mapping.base_url, dependency_file_hash);
            let absolute_file_path = format!("{}{}", ctx.content_folder, dependency_file_hash);
            fetch_resource_or_wait(&url, dependency_file_hash, &absolute_file_path, ctx.clone())
                .await
                .map_err(|e| {
                    format!(
                        "Dependency {} failed to fetch: {:?}",
                        dependency_file_hash, e
                    )
                })
        }
    });

    let result = futures_util::future::join_all(futures).await;
    if result.iter().any(|res| res.is_err()) {
        // collect errors
        let errors = result
            .into_iter()
            .filter_map(|res| res.err())
            .map(|err| err.to_string())
            .collect::<Vec<String>>()
            .join("\n");

        return Err(anyhow::Error::msg(format!(
            "Error downloading gltf dependencies: {errors}"
        )));
    }

    let thread_safe_check = GodotSingleThreadSafety::acquire_owned(&ctx)
        .await
        .ok_or(anyhow::Error::msg("Failed trying to get thread-safe check"))?;

    let mut new_gltf = GltfDocument::new();
    let mut new_gltf_state = GltfState::new();

    let mappings = Dictionary::from_iter(
        dependencies_hash
            .iter()
            .map(|(file_path, hash)| (file_path.to_variant(), hash.to_variant())),
    );

    new_gltf_state.set_additional_data("base_path".into(), "some".to_variant());
    new_gltf_state.set_additional_data("mappings".into(), mappings.to_variant());

    let err = new_gltf
        .append_from_file_ex(
            GString::from(absolute_file_path.as_str()),
            new_gltf_state.clone(),
        )
        .base_path(GString::from(ctx.content_folder.as_str()))
        .flags(0)
        .done();

    if err != Error::OK {
        let err = err.to_variant().to::<i32>();
        return Err(anyhow::Error::msg(format!(
            "Error loading gltf after appending from file {}",
            err
        )));
    }

    let node = new_gltf
        .generate_scene(new_gltf_state)
        .ok_or(anyhow::Error::msg(
            "Error loading gltf when generating scene".to_string(),
        ))?;

    let mut node = node.try_cast::<Node3D>().map_err(|err| {
        anyhow::Error::msg(format!("Error loading gltf when casting to Node3D: {err}"))
    })?;

    node.rotate_y(std::f32::consts::PI);

    Ok((node, thread_safe_check))
}

pub async fn load_gltf_scene_content(
    file_path: String,
    content_mapping: ContentMappingAndUrlRef,
    ctx: ContentProviderContext,
) -> Result<Option<Variant>, anyhow::Error> {
    let (node, _thread_safe_check) = internal_load_gltf(file_path, content_mapping, ctx).await?;
    create_colliders(node.clone().upcast());
    Ok(Some(node.to_variant()))
}

pub async fn load_gltf_wearable(
    file_path: String,
    content_mapping: ContentMappingAndUrlRef,
    ctx: ContentProviderContext,
) -> Result<Option<Variant>, anyhow::Error> {
    let (node, _thread_safe_check) = internal_load_gltf(file_path, content_mapping, ctx).await?;
    Ok(Some(node.to_variant()))
}

pub async fn load_gltf_emote(
    file_path: String,
    content_mapping: ContentMappingAndUrlRef,
    ctx: ContentProviderContext,
) -> Result<Option<Variant>, anyhow::Error> {
    let file_hash = content_mapping
        .clone()
        .get_hash(file_path.as_str())
        .ok_or(anyhow::Error::msg("File not found in the content mappings"))?
        .clone();

    let (gltf_node, _thread_safe_check) =
        internal_load_gltf(file_path, content_mapping, ctx).await?;
    Ok(add_animation_from_obj(&file_hash, gltf_node).map(|emote| emote.to_variant()))
}

pub async fn apply_update_set_mask_colliders(
    gltf_node_instance_id: InstanceId,
    dcl_visible_cmask: i32,
    dcl_invisible_cmask: i32,
    dcl_scene_id: i32,
    dcl_entity_id: i32,
    ctx: ContentProviderContext,
) -> Result<Option<Variant>, anyhow::Error> {
    let _thread_safe_check = GodotSingleThreadSafety::acquire_owned(&ctx)
        .await
        .ok_or(anyhow::Error::msg("Failed trying to get thread-safe check"))?;

    let mut to_remove_nodes = Vec::new();
    let gltf_node: Gd<Node> = Gd::from_instance_id(gltf_node_instance_id);
    let gltf_node = gltf_node
        .duplicate_ex()
        .flags(8)
        .done()
        .ok_or(anyhow::Error::msg("unable to duplicate gltf node"))?;

    update_set_mask_colliders(
        gltf_node.clone(),
        dcl_visible_cmask,
        dcl_invisible_cmask,
        dcl_scene_id,
        dcl_entity_id,
        &mut to_remove_nodes,
    );

    // TODO: Check if remove this logic is needed
    //      animation duplication is no longer needed for MultipleAnimationController.
    // duplicate_animation_resources(gltf_node.clone());

    //  The duplication was done because the loop property could be modified
    //      in the MultipleAnimationController the looping is handled by replaying every time
    //      the animation emits the finished signal

    for mut node in to_remove_nodes {
        node.queue_free();
    }

    Ok(Some(gltf_node.to_variant()))
}

async fn get_dependencies(file_path: &String) -> Result<Vec<String>, anyhow::Error> {
    let mut dependencies = Vec::new();
    let mut file = tokio::fs::File::open(file_path).await?;

    let magic = file.read_i32_le().await?;
    let json: serde_json::Value = if magic == 0x46546C67 {
        let _version = file.read_i32_le().await?;
        let _length = file.read_i32_le().await?;
        let chunk_length = file.read_i32_le().await?;
        let _chunk_type = file.read_i32_le().await?;

        let mut json_data = vec![0u8; chunk_length as usize];
        let _ = file.read_exact(&mut json_data).await?;
        serde_json::de::from_slice(json_data.as_slice())
    } else {
        let mut json_data = Vec::new();
        let _ = file.seek(std::io::SeekFrom::Start(0)).await?;
        let _ = file.read_to_end(&mut json_data).await?;
        serde_json::de::from_slice(json_data.as_slice())
    }?;

    if let Some(images) = json.get("images") {
        if let Some(images) = images.as_array() {
            for image in images {
                if let Some(uri) = image.get("uri") {
                    if let Some(uri) = uri.as_str() {
                        if !uri.is_empty() && !uri.starts_with("data:") {
                            dependencies.push(uri.to_string());
                        }
                    }
                }
            }
        }
    }

    if let Some(images) = json.get("buffers") {
        if let Some(images) = images.as_array() {
            for image in images {
                if let Some(uri) = image.get("uri") {
                    if let Some(uri) = uri.as_str() {
                        if !uri.is_empty() && !uri.starts_with("data:") {
                            dependencies.push(uri.to_string());
                        }
                    }
                }
            }
        }
    }

    Ok(dependencies)
}

fn get_collider(mesh_instance: &Gd<MeshInstance3D>) -> Option<Gd<StaticBody3D>> {
    for maybe_static_body in mesh_instance.get_children().iter_shared() {
        if let Ok(static_body_3d) = maybe_static_body.try_cast::<StaticBody3D>() {
            return Some(static_body_3d);
        }
    }
    None
}

fn create_colliders(node_to_inspect: Gd<Node>) {
    for child in node_to_inspect.get_children().iter_shared() {
        if let Ok(mut mesh_instance_3d) = child.clone().try_cast::<MeshInstance3D>() {
            let invisible_mesh = mesh_instance_3d
                .get_name()
                .to_string()
                .to_lowercase()
                .contains("collider");

            if invisible_mesh {
                mesh_instance_3d.set_visible(false);
            }

            let mut static_body_3d = get_collider(&mesh_instance_3d);
            if static_body_3d.is_none() {
                mesh_instance_3d.create_trimesh_collision();
                static_body_3d = get_collider(&mesh_instance_3d);
            }

            if let Some(mut static_body_3d) = static_body_3d {
                if let Some(mut parent) = static_body_3d.get_parent() {
                    let mut new_animatable = AnimatableBody3D::new_alloc();
                    new_animatable.set_sync_to_physics(false);
                    new_animatable.set_process_mode(ProcessMode::PROCESS_MODE_DISABLED);
                    new_animatable.set_meta("dcl_col".into(), 0.to_variant());
                    new_animatable.set_meta("invisible_mesh".into(), invisible_mesh.to_variant());
                    new_animatable.set_collision_layer(0);
                    new_animatable.set_collision_mask(0);
                    new_animatable.set_name(GString::from(format!(
                        "{}_colgen",
                        mesh_instance_3d.get_name()
                    )));

                    parent.add_child(new_animatable.clone().upcast());
                    parent.remove_child(static_body_3d.clone().upcast());

                    for body_child in static_body_3d
                        .get_children_ex()
                        .include_internal(true)
                        .done()
                        .iter_shared()
                    {
                        static_body_3d.remove_child(body_child.clone());
                        new_animatable.add_child(body_child.clone());
                        if let Ok(collision_shape_3d) = body_child.try_cast::<CollisionShape3D>() {
                            if let Some(shape) = collision_shape_3d.get_shape() {
                                if let Ok(mut concave_polygon_shape_3d) =
                                    shape.try_cast::<ConcavePolygonShape3D>()
                                {
                                    concave_polygon_shape_3d.set_backface_collision_enabled(true);
                                }
                            }
                        }
                    }
                }
            }
        }

        create_colliders(child);
    }
}

fn update_set_mask_colliders(
    mut node_to_inspect: Gd<Node>,
    dcl_visible_cmask: i32,
    dcl_invisible_cmask: i32,
    dcl_scene_id: i32,
    dcl_entity_id: i32,
    to_remove_nodes: &mut Vec<Gd<Node>>,
) {
    for child in node_to_inspect.get_children().iter_shared() {
        if let Ok(mut node) = child.clone().try_cast::<AnimatableBody3D>() {
            let invisible_mesh = node.has_meta("invisible_mesh".into())
                && node
                    .get_meta("invisible_mesh".into())
                    .try_to::<bool>()
                    .unwrap_or_default();

            let mask = if invisible_mesh {
                dcl_invisible_cmask
            } else {
                dcl_visible_cmask
            };

            if !node.has_meta("dcl_scene_id".into()) {
                let Some(mut resolved_node) = node.duplicate_ex().flags(8).done() else {
                    continue;
                };

                resolved_node.set_name(GString::from(format!("{}_instanced", node.get_name())));
                resolved_node.set_meta("dcl_scene_id".into(), dcl_scene_id.to_variant());
                resolved_node.set_meta("dcl_entity_id".into(), dcl_entity_id.to_variant());

                node_to_inspect.add_child(resolved_node.clone().upcast());
                to_remove_nodes.push(node.clone().upcast());

                node = resolved_node.cast();
            }

            node.set_meta("dcl_col".into(), mask.to_variant());
            node.set_collision_layer(mask as u32);
            node.set_collision_mask(0);
            if mask == 0 {
                node.set_process_mode(ProcessMode::PROCESS_MODE_DISABLED);
            } else {
                node.set_process_mode(ProcessMode::PROCESS_MODE_INHERIT);
            }
        }

        update_set_mask_colliders(
            child,
            dcl_visible_cmask,
            dcl_invisible_cmask,
            dcl_scene_id,
            dcl_entity_id,
            to_remove_nodes,
        )
    }
}

// TODO: maybe remove
fn _duplicate_animation_resources(gltf_node: Gd<Node>) {
    let Some(mut animation_player) =
        gltf_node.try_get_node_as::<AnimationPlayer>("AnimationPlayer")
    else {
        return;
    };

    let mut new_animation_libraries = HashMap::new();
    let animation_libraries = animation_player.get_animation_library_list();
    for animation_library_name in animation_libraries.iter_shared() {
        let Some(animation_library) =
            animation_player.get_animation_library(animation_library_name.clone())
        else {
            tracing::error!("animation library not found");
            continue;
        };

        let mut new_animations = HashMap::new();
        let animations = animation_library.get_animation_list();
        for animation_name in animations.iter_shared() {
            let Some(animation) = animation_player.get_animation(animation_name.clone()) else {
                continue;
            };

            let Some(dup_animation) = animation.duplicate_ex().subresources(true).done() else {
                tracing::error!("Error duplicating animation {:?}", animation_name);
                continue;
            };
            let _ = new_animations.insert(animation_name, dup_animation);
        }

        let mut new_animation_library = AnimationLibrary::new();
        for new_animation in new_animations {
            new_animation_library.add_animation(new_animation.0, new_animation.1.cast());
        }
        new_animation_libraries.insert(animation_library_name, new_animation_library);
    }

    // remove current animation library
    for animation_library_name in animation_libraries.iter_shared() {
        animation_player.remove_animation_library(animation_library_name);
    }

    // add new animation library
    for new_animation_library in new_animation_libraries {
        animation_player.add_animation_library(new_animation_library.0, new_animation_library.1);
    }
}

fn get_last_16_alphanumeric(input: &str) -> String {
    let alphanumeric: String = input
        .chars()
        .rev()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(16)
        .collect();

    alphanumeric
        .chars()
        .rev()
        .collect::<String>()
        .to_lowercase()
}

fn add_animation_from_obj(file_hash: &String, gltf_node: Gd<Node3D>) -> Option<Gd<DclEmoteGltf>> {
    let anim_sufix_from_hash = get_last_16_alphanumeric(file_hash.as_str());
    let armature_prop = gltf_node.find_child("Armature_Prop".into());
    let Some(anim_player) = gltf_node.try_get_node_as::<AnimationPlayer>("AnimationPlayer") else {
        return None;
    };
    let armature_prefix = format!("Armature_Prop_{}/Skeleton3D:", anim_sufix_from_hash);

    let armature_prop = armature_prop
        .and_then(|v| v.clone().try_cast::<Node3D>().ok())
        .map(|mut node| {
            node.set_name(format!("Armature_Prop_{}", anim_sufix_from_hash).into());
            node.rotate_y(std::f32::consts::PI);
            node
        });

    let is_single_animation = anim_player.get_animation_list().len() == 1;

    let anim_list: Vec<String> = anim_player
        .get_animation_list()
        .as_slice()
        .iter()
        .map(|v| v.to_string())
        .collect();

    let mut default_animation: Option<Gd<Animation>> = None;
    let mut prop_animation: Option<Gd<Animation>> = None;
    let mut default_anim_key = None;
    let mut prop_anim_key = None;

    for animation_key in anim_list.iter() {
        if is_single_animation || animation_key.to_lowercase().ends_with("_avatar") {
            default_anim_key = Some(animation_key.clone());
        } else if animation_key.to_lowercase().ends_with("_prop") {
            prop_anim_key = Some(animation_key.clone());
        }
    }

    // Corner case, the glb doesn't follow the docs instructions
    if !is_single_animation {
        for animation_key in anim_list.iter() {
            if default_anim_key.is_none() {
                default_anim_key = Some(animation_key.clone());
            } else if prop_anim_key.is_none() {
                prop_anim_key = Some(animation_key.clone());
            }
        }
    }

    let mut play_emote_audio_args = VariantArray::new();
    play_emote_audio_args.push(file_hash.to_variant());
    let play_emote_audio_call = Dictionary::from_iter([
        ("method", "_play_emote_audio".to_variant()),
        ("args", play_emote_audio_args.to_variant()),
    ]);

    let mut audio_added = false;

    for animation_key in anim_list.iter() {
        let Some(mut anim) = anim_player.get_animation(animation_key.into()) else {
            continue;
        };

        if default_anim_key.as_ref() == Some(animation_key) {
            default_animation = Some(anim.clone());
            anim.set_name(anim_sufix_from_hash.to_string().into())
        } else if prop_anim_key.as_ref() == Some(animation_key) {
            prop_animation = Some(anim.clone());
            anim.set_name(format!("{anim_sufix_from_hash}_prop").into())
        }

        for track_idx in 0..anim.get_track_count() {
            let track_path = anim.track_get_path(track_idx).to_string();
            if !track_path.contains("Skeleton3D") {
                let last_track_name = track_path.split('/').last().unwrap_or_default();
                let new_track_path = format!("Armature/Skeleton3D:{}", last_track_name);
                anim.track_set_path(track_idx, new_track_path.into());
            }
            if track_path.contains("Armature_Prop/Skeleton3D") {
                let track_subname = track_path.split(':').last().unwrap_or_default();
                let new_track_path = format!("{armature_prefix}{track_subname}");
                anim.track_set_path(track_idx, new_track_path.into());
            }
        }

        if armature_prop.is_some() {
            let new_track_prop = anim.add_track(TrackType::TYPE_VALUE);
            anim.track_set_path(
                new_track_prop,
                format!("Armature_Prop_{}:visible", anim_sufix_from_hash).into(),
            );
            anim.track_insert_key(new_track_prop, 0.0, true.to_variant());
        }

        if !audio_added {
            audio_added = true;
            let new_track_audio = anim.add_track(TrackType::TYPE_METHOD);
            anim.track_set_path(new_track_audio, ".".into());
            anim.track_insert_key(
                new_track_audio,
                0.0,
                play_emote_audio_call.clone().to_variant(),
            );
        }
    }

    Some(Gd::from_init_fn(|_base| DclEmoteGltf {
        armature_prop,
        default_animation,
        prop_animation,
    }))
}

#[derive(GodotClass)]
#[class(base=RefCounted)]
pub struct DclEmoteGltf {
    #[var]
    armature_prop: Option<Gd<Node3D>>,
    #[var]
    default_animation: Option<Gd<Animation>>,
    #[var]
    prop_animation: Option<Gd<Animation>>,
}
