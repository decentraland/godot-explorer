use std::{collections::HashMap, sync::Arc};

use godot::{
    builtin::{GString, VarArray, Variant},
    classes::{
        animation::TrackType, base_material_3d::TextureParam, node::ProcessMode, AnimatableBody3D,
        Animation, AnimationLibrary, AnimationPlayer, BaseMaterial3D, CollisionShape3D,
        ConcavePolygonShape3D, GltfDocument, GltfState, ImageTexture, MeshInstance3D, Node, Node3D,
        StaticBody3D,
    },
    global::Error,
    meta::ToGodot,
    obj::{EngineEnum, Gd, NewAlloc},
    prelude::*,
};
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio::sync::Semaphore;

use crate::{content::texture::resize_image, godot_classes::resource_locker::ResourceLocker};

#[cfg(feature = "use_resource_tracking")]
use crate::godot_classes::dcl_resource_tracker::report_resource_loading;

use super::{
    content_mapping::ContentMappingAndUrlRef,
    content_provider::{ContentProviderContext, SceneGltfContext},
    file_string::get_base_dir,
    scene_saver::{
        get_emote_path_for_hash, get_scene_path_for_hash, get_wearable_path_for_hash,
        save_node_as_scene,
    },
    texture::create_compressed_texture,
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
    ctx.resource_provider
        .fetch_resource(url, file_hash.clone(), absolute_file_path.clone())
        .await
        .map_err(anyhow::Error::msg)?;

    #[cfg(feature = "use_resource_tracking")]
    report_resource_loading(file_hash, &"0%".to_string(), &"gltf started".to_string());

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
            ctx.resource_provider
                .fetch_resource(url, dependency_file_hash.clone(), absolute_file_path)
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

    let mut new_gltf = GltfDocument::new_gd();
    let mut new_gltf_state = GltfState::new_gd();

    let mappings = VarDictionary::from_iter(
        dependencies_hash
            .iter()
            .map(|(file_path, hash)| (file_path.to_variant(), hash.to_variant())),
    );

    new_gltf_state.set_additional_data("base_path", &"some".to_variant());
    new_gltf_state.set_additional_data("mappings", &mappings.to_variant());

    let file_path = GString::from(absolute_file_path.as_str());
    let base_path = GString::from(ctx.content_folder.as_str());
    let err = new_gltf
        .append_from_file_ex(&file_path, &new_gltf_state.clone())
        .base_path(&base_path)
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
        .generate_scene(&new_gltf_state)
        .ok_or(anyhow::Error::msg(
            "Error loading gltf when generating scene".to_string(),
        ))?;

    // Attach a ResourceLocker to the Node to control the lifecycle
    ResourceLocker::attach_to(node.clone());

    let max_size = ctx.texture_quality.to_max_size();
    post_import_process(node.clone(), max_size);

    let mut node = node.try_cast::<Node3D>().map_err(|err| {
        anyhow::Error::msg(format!("Error loading gltf when casting to Node3D: {err}"))
    })?;

    node.rotate_y(std::f32::consts::PI);

    Ok((node, thread_safe_check))
}

pub fn post_import_process(node_to_inspect: Gd<Node>, max_size: i32) {
    for child in node_to_inspect.get_children().iter_shared() {
        if let Ok(mesh_instance_3d) = child.clone().try_cast::<MeshInstance3D>() {
            if let Some(mesh) = mesh_instance_3d.get_mesh() {
                for surface_index in 0..mesh.get_surface_count() {
                    if let Some(material) = mesh.surface_get_material(surface_index) {
                        if let Ok(mut base_material) = material.try_cast::<BaseMaterial3D>() {
                            // Resize images
                            for ord in 0..TextureParam::MAX.ord() {
                                let texture_param = TextureParam::from_ord(ord);
                                if let Some(texture) = base_material.get_texture(texture_param) {
                                    if let Ok(mut texture_image) =
                                        texture.try_cast::<ImageTexture>()
                                    {
                                        if let Some(mut image) = texture_image.get_image() {
                                            if std::env::consts::OS == "ios" {
                                                let texture =
                                                    create_compressed_texture(&mut image, max_size);
                                                base_material.set_texture(texture_param, &texture);
                                            } else if resize_image(&mut image, max_size) {
                                                texture_image.set_image(&image);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        post_import_process(child, max_size);
    }
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

    let result = process_emote_animations(&file_hash, &gltf_node).map(
        |(mut armature_prop, default_animation, prop_animation)| {
            // Remove armature_prop from gltf_node before freeing (so it survives)
            if let Some(ref mut prop) = armature_prop {
                if let Some(mut parent) = prop.get_parent() {
                    parent.remove_child(&prop.clone().upcast::<Node>());
                }
            }
            build_dcl_emote_gltf(armature_prop, default_animation, prop_animation).to_variant()
        },
    );
    gltf_node.free();
    Ok(result)
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
            animation_player.get_animation_library(&animation_library_name.clone())
        else {
            tracing::error!("animation library not found");
            continue;
        };

        let mut new_animations = HashMap::new();
        let animations = animation_library.get_animation_list();
        for animation_name in animations.iter_shared() {
            let Some(animation) = animation_player.get_animation(&animation_name.clone()) else {
                continue;
            };

            let Some(dup_animation) = animation.duplicate_ex().deep(true).done() else {
                tracing::error!("Error duplicating animation {:?}", animation_name);
                continue;
            };
            let _ = new_animations.insert(animation_name, dup_animation);
        }

        let mut new_animation_library = AnimationLibrary::new_gd();
        for new_animation in new_animations {
            let this_animation = new_animation.1.cast::<Animation>();
            new_animation_library.add_animation(&new_animation.0, &this_animation);
        }
        new_animation_libraries.insert(animation_library_name, new_animation_library);
    }

    // remove current animation library
    for animation_library_name in animation_libraries.iter_shared() {
        animation_player.remove_animation_library(&animation_library_name);
    }

    // add new animation library
    for new_animation_library in new_animation_libraries {
        animation_player.add_animation_library(&new_animation_library.0, &new_animation_library.1);
    }
}

/// Strip Blender duplicate suffixes from bone names
///
/// Blender adds `_001`, `_002`, etc. suffixes when objects have duplicate names.
/// These need to be stripped so animation tracks can target the actual skeleton bones.
///
/// Pattern: `_0XX` where XX are digits (e.g., `_001`, `_012`, `_099`)
/// This does NOT strip valid bone suffixes like `Index1`, `Thumb2` (no underscore before digit)
///
/// Examples:
/// - `Avatar_LeftLeg_001` → `Avatar_LeftLeg`
/// - `Avatar_RightFoot_012` → `Avatar_RightFoot`
/// - `Avatar_LeftHandIndex1` → `Avatar_LeftHandIndex1` (unchanged - valid bone name)
fn strip_blender_suffix(name: &str) -> String {
    // Pattern: ends with underscore + 0 + two more digits
    // e.g., _001, _002, _012, _099
    if name.len() >= 4 {
        let bytes = name.as_bytes();
        let len = bytes.len();

        // Check if ends with _0XX pattern
        if bytes[len - 4] == b'_'
            && bytes[len - 3] == b'0'
            && bytes[len - 2].is_ascii_digit()
            && bytes[len - 1].is_ascii_digit()
        {
            return name[..len - 4].to_string();
        }
    }
    name.to_string()
}

/// Get the last 16 alphanumeric characters from a hash (used for animation naming)
pub fn get_last_16_alphanumeric(input: &str) -> String {
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

/// Process emote animations and return components for embedding or DclEmoteGltf creation
///
/// Returns (armature_prop, default_animation, prop_animation)
/// This is used by load_and_save_emote_gltf to extract and embed animations in the background thread
#[allow(clippy::type_complexity)]
pub fn process_emote_animations(
    file_hash: &str,
    gltf_node: &Gd<Node3D>,
) -> Option<(
    Option<Gd<Node3D>>,
    Option<Gd<Animation>>,
    Option<Gd<Animation>>,
)> {
    let anim_sufix_from_hash = get_last_16_alphanumeric(file_hash);
    let armature_prop_node = gltf_node.find_child("Armature_Prop");

    let anim_player = gltf_node.try_get_node_as::<AnimationPlayer>("AnimationPlayer")?;

    let armature_prefix = format!("Armature_Prop_{}/Skeleton3D:", anim_sufix_from_hash);

    let armature_prop = armature_prop_node
        .and_then(|v| v.clone().try_cast::<Node3D>().ok())
        .map(|mut node| {
            node.set_name(&format!("Armature_Prop_{}", anim_sufix_from_hash));
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

    tracing::debug!(
        "Emote '{}': Found {} animations: {:?}, is_single_animation={}",
        file_hash,
        anim_list.len(),
        anim_list,
        is_single_animation
    );

    for animation_key in anim_list.iter() {
        // Strip Blender suffixes before checking for _avatar or _prop endings
        let key_lower = animation_key.to_lowercase();
        let key_stripped = strip_blender_suffix(&key_lower);

        if is_single_animation || key_stripped.ends_with("_avatar") {
            default_anim_key = Some(animation_key.clone());
        } else if key_stripped.ends_with("_prop")
            || key_stripped.ends_with("action")
            || key_lower.contains("prop")
        {
            // Match prop animations: ending with _prop, or "action" (common Blender naming),
            // or containing "prop" anywhere in the name
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

    tracing::debug!(
        "Emote '{}': Animation assignments - default_anim_key={:?}, prop_anim_key={:?}",
        file_hash,
        default_anim_key,
        prop_anim_key
    );

    let mut play_emote_audio_args = VarArray::new();
    play_emote_audio_args.push(&file_hash.to_variant());
    let play_emote_audio_call = VarDictionary::from_iter([
        ("method", "_play_emote_audio".to_variant()),
        ("args", play_emote_audio_args.to_variant()),
    ]);

    let mut audio_added = false;

    for animation_key in anim_list.iter() {
        let Some(mut anim) = anim_player.get_animation(animation_key) else {
            continue;
        };

        tracing::debug!(
            "Processing emote animation: hash='{}', key='{}', track_count={}, is_default={}, is_prop={}",
            file_hash,
            animation_key,
            anim.get_track_count(),
            default_anim_key.as_ref() == Some(animation_key),
            prop_anim_key.as_ref() == Some(animation_key)
        );

        // Log all tracks before processing
        for track_idx in 0..anim.get_track_count() {
            let track_path = anim.track_get_path(track_idx).to_string();
            tracing::debug!("  [BEFORE] Track[{}]: '{}'", track_idx, track_path);
        }

        if default_anim_key.as_ref() == Some(animation_key) {
            default_animation = Some(anim.clone());
            anim.set_name(&anim_sufix_from_hash.to_godot())
        } else if prop_anim_key.as_ref() == Some(animation_key) {
            prop_animation = Some(anim.clone());
            anim.set_name(&format!("{anim_sufix_from_hash}_prop"))
        }

        // First pass: identify tracks that reference orphan Blender duplicate bones
        // These tracks have animation data authored for a wrong bone hierarchy
        // and will cause incorrect animations if applied to the real skeleton.
        //
        // We identify two types of problematic tracks:
        // 1. Direct orphan tracks: "Armature/Avatar_LeftLeg_001" (direct child with suffix)
        // 2. Descendants of orphans: "Armature/Avatar_LeftLeg_001/Avatar_LeftFoot"
        //    (these have animation data in the wrong coordinate space)
        let mut orphan_tracks: Vec<i32> = Vec::new();

        for track_idx in 0..anim.get_track_count() {
            let track_path = anim.track_get_path(track_idx).to_string();

            // Skip tracks that already have Skeleton3D or are prop tracks
            if track_path.contains("Skeleton3D") || track_path.contains("Armature_Prop") {
                continue;
            }

            let parts: Vec<&str> = track_path.split('/').collect();

            // Check if ANY part of the path (except the last bone) has a Blender suffix
            // This means the track is either an orphan or a child of an orphan
            let mut has_orphan_ancestor = false;
            for (i, part) in parts.iter().enumerate() {
                // Skip "Armature" prefix and the last part (which is the target bone)
                if i == 0 || i == parts.len() - 1 {
                    continue;
                }
                // Check if this intermediate path part has a Blender suffix
                let stripped = strip_blender_suffix(part);
                if stripped != *part {
                    has_orphan_ancestor = true;
                    break;
                }
            }

            // Also check for direct orphan: "Armature/BoneName_0XX" (2 parts)
            if parts.len() == 2 && parts[0] == "Armature" {
                let bone_name = parts[1];
                let stripped = strip_blender_suffix(bone_name);
                if stripped != bone_name {
                    has_orphan_ancestor = true;
                }
            }

            if has_orphan_ancestor {
                tracing::debug!(
                    "Removing orphan hierarchy track [{}]: '{}' (has Blender duplicate ancestor)",
                    track_idx,
                    track_path
                );
                orphan_tracks.push(track_idx);
            }
        }

        // Remove orphan tracks (in reverse order to maintain indices)
        orphan_tracks.sort();
        orphan_tracks.reverse();
        for track_idx in &orphan_tracks {
            tracing::debug!("Removing orphan track {}", track_idx);
            anim.remove_track(*track_idx);
        }

        // Second pass: remap remaining tracks
        for track_idx in 0..anim.get_track_count() {
            let track_path = anim.track_get_path(track_idx).to_string();

            // Handle root motion "Armature" tracks specially
            // These animate the entire Armature node and need coordinate system transformation
            // because the avatar's Skeleton3D has a 180° Y rotation built into its transform
            if track_path == "Armature" {
                let track_type = anim.track_get_type(track_idx);
                match track_type {
                    TrackType::POSITION_3D => {
                        // Don't transform position - let it pass through as-is
                        // The sway motion should work without position changes
                        tracing::debug!(
                            "  Keeping position track 'Armature' unchanged ({} keys)",
                            anim.track_get_key_count(track_idx)
                        );
                    }
                    TrackType::ROTATION_3D => {
                        // Transform rotation values: invert X only
                        // This compensates for the 180° Y rotation in the skeleton
                        let key_count = anim.track_get_key_count(track_idx);
                        for key_idx in 0..key_count {
                            let value = anim.track_get_key_value(track_idx, key_idx);
                            if let Ok(rot) = value.try_to::<Quaternion>() {
                                // Invert X and Z (fixes direction), then apply +90° X rotation (fixes forward tilt)
                                // The avatar's Skeleton3D has a 180° Y rotation, requiring this compensation
                                let sin_45 = std::f32::consts::FRAC_PI_4.sin(); // sin(45°) = cos(45°) ≈ 0.7071
                                let x_pos90 = Quaternion::new(sin_45, 0.0, 0.0, sin_45);
                                let inverted = Quaternion::new(-rot.x, rot.y, -rot.z, rot.w);
                                let transformed = x_pos90 * inverted;
                                anim.track_set_key_value(
                                    track_idx,
                                    key_idx,
                                    &transformed.to_variant(),
                                );
                            }
                        }
                        tracing::debug!(
                            "  Transformed {} rotation keys for root motion track 'Armature' (inverted X)",
                            key_count
                        );
                    }
                    _ => {
                        tracing::debug!(
                            "  Keeping root motion track 'Armature' unchanged (type: {:?})",
                            track_type
                        );
                    }
                }
                continue;
            }

            if !track_path.contains("Skeleton3D") {
                let last_track_name = track_path.split('/').next_back().unwrap_or_default();

                // Strip Blender duplicate suffixes from the BONE NAME only
                let bone_name = strip_blender_suffix(last_track_name);

                // Check if this is a prop track (Armature_Prop/...) or avatar track (Armature/...)
                if track_path.starts_with("Armature_Prop") {
                    // Check if it's a root prop track (just "Armature_Prop")
                    if track_path == "Armature_Prop" {
                        // Rename to Armature_Prop_{hash} for root motion
                        let new_track_path = format!("Armature_Prop_{}", anim_sufix_from_hash);
                        tracing::debug!(
                            "  Remapping prop root motion '{}' -> '{}'",
                            track_path,
                            new_track_path
                        );
                        anim.track_set_path(track_idx, &NodePath::from(&new_track_path));
                    } else {
                        // Prop bone track: remap to Armature_Prop_{hash}/Skeleton3D:{bone}
                        let new_track_path = format!("{}{}", armature_prefix, bone_name);
                        tracing::debug!(
                            "  Remapping prop track '{}' -> '{}'",
                            track_path,
                            new_track_path
                        );
                        anim.track_set_path(track_idx, &NodePath::from(&new_track_path));
                    }
                } else {
                    // Avatar track: remap to Armature/Skeleton3D:{bone}
                    let new_track_path = format!("Armature/Skeleton3D:{}", bone_name);
                    anim.track_set_path(track_idx, &NodePath::from(&new_track_path));
                }
            } else if track_path.contains("Armature_Prop/Skeleton3D")
                || track_path.contains("Armature_Prop:")
            {
                // Already has Skeleton3D, just rename the Armature_Prop to include hash
                let track_subname = track_path.split(':').next_back().unwrap_or_default();
                let new_track_path = format!("{}{}", armature_prefix, track_subname);
                tracing::debug!(
                    "  Remapping Skeleton3D prop track '{}' -> '{}'",
                    track_path,
                    new_track_path
                );
                anim.track_set_path(track_idx, &NodePath::from(&new_track_path));
            }
        }

        if armature_prop.is_some() {
            let new_track_prop = anim.add_track(TrackType::VALUE);
            anim.track_set_path(
                new_track_prop,
                &format!("Armature_Prop_{}:visible", anim_sufix_from_hash),
            );
            anim.track_insert_key(new_track_prop, 0.0, &true.to_variant());
        }

        if !audio_added {
            audio_added = true;
            let new_track_audio = anim.add_track(TrackType::METHOD);
            anim.track_set_path(new_track_audio, ".");
            anim.track_insert_key(
                new_track_audio,
                0.0,
                &play_emote_audio_call.clone().to_variant(),
            );
        }

        // Log all tracks after processing
        tracing::debug!(
            "  [AFTER] Animation '{}' now has {} tracks",
            animation_key,
            anim.get_track_count()
        );
        for track_idx in 0..anim.get_track_count() {
            let track_path = anim.track_get_path(track_idx).to_string();
            tracing::debug!("  [AFTER] Track[{}]: '{}'", track_idx, track_path);
        }
    }

    Some((armature_prop, default_animation, prop_animation))
}

/// Build DclEmoteGltf from processed components
/// Used when loading from cache to create the DclEmoteGltf from stored data
pub fn build_dcl_emote_gltf(
    armature_prop: Option<Gd<Node3D>>,
    default_animation: Option<Gd<Animation>>,
    prop_animation: Option<Gd<Animation>>,
) -> Gd<DclEmoteGltf> {
    Gd::from_init_fn(|_base| DclEmoteGltf {
        armature_prop,
        default_animation,
        prop_animation,
    })
}

#[derive(GodotClass)]
#[class(init, base=RefCounted)]
pub struct DclEmoteGltf {
    #[var]
    armature_prop: Option<Gd<Node3D>>,
    #[var]
    default_animation: Option<Gd<Animation>>,
    #[var]
    prop_animation: Option<Gd<Animation>>,
}

// ============================================================================
// Scene GLTF Loading (for ContentProvider scene loading)
// ============================================================================

/// Thread safety guard for Godot API access
pub struct GodotThreadSafetyGuard {
    _guard: tokio::sync::OwnedSemaphorePermit,
}

impl GodotThreadSafetyGuard {
    pub async fn acquire(godot_single_thread: &Arc<Semaphore>) -> Option<Self> {
        let guard = godot_single_thread.clone().acquire_owned().await.ok()?;
        set_thread_safety_checks_enabled(false);
        Some(Self { _guard: guard })
    }
}

impl Drop for GodotThreadSafetyGuard {
    fn drop(&mut self) {
        set_thread_safety_checks_enabled(true);
    }
}

fn set_thread_safety_checks_enabled(enabled: bool) {
    let mut temp_script =
        godot::tools::load::<godot::classes::Script>("res://src/logic/thread_safety.gd");
    temp_script.call("set_thread_safety_checks_enabled", &[enabled.to_variant()]);
}

/// Load and save a scene GLTF to disk
///
/// This function:
/// 1. Downloads the GLTF and its dependencies
/// 2. Loads it into Godot
/// 3. Processes textures
/// 4. Creates colliders (with mask=0 - caller sets masks after instantiating)
/// 5. Saves the processed scene to disk
///
/// Returns the path to the saved scene file on success
pub async fn load_and_save_scene_gltf(
    file_path: String,
    file_hash: String,
    content_mapping: ContentMappingAndUrlRef,
    ctx: SceneGltfContext,
) -> Result<String, anyhow::Error> {
    // Download the main GLTF file
    let base_path = Arc::new(get_base_dir(&file_path));
    let url = format!("{}{}", content_mapping.base_url, file_hash);
    let absolute_file_path = format!("{}{}", ctx.content_folder, file_hash);

    ctx.resource_provider
        .fetch_resource(url, file_hash.clone(), absolute_file_path.clone())
        .await
        .map_err(anyhow::Error::msg)?;

    // Get dependencies from the GLTF file
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

    // Check all dependencies are available
    if dependencies.iter().any(|(_, hash)| hash.is_none()) {
        return Err(anyhow::Error::msg(
            "There are some missing dependencies in the gltf",
        ));
    }

    let dependencies_hash: Vec<(String, String)> = dependencies
        .into_iter()
        .map(|(file_path, hash)| (file_path, hash.unwrap()))
        .collect();

    // Download all dependencies in parallel
    let futures = dependencies_hash.iter().map(|(_, dependency_file_hash)| {
        let ctx = ctx.clone();
        let content_mapping = content_mapping.clone();
        async move {
            let url = format!("{}{}", content_mapping.base_url, dependency_file_hash);
            let absolute_file_path = format!("{}{}", ctx.content_folder, dependency_file_hash);
            ctx.resource_provider
                .fetch_resource(url, dependency_file_hash.clone(), absolute_file_path)
                .await
                .map_err(|e| format!("Dependency {} failed: {:?}", dependency_file_hash, e))
        }
    });

    let result = futures_util::future::join_all(futures).await;
    if result.iter().any(|res| res.is_err()) {
        let errors: Vec<String> = result.into_iter().filter_map(|res| res.err()).collect();
        return Err(anyhow::Error::msg(format!(
            "Error downloading gltf dependencies: {}",
            errors.join("\n")
        )));
    }

    // Acquire thread safety guard for Godot API access
    let _thread_guard = GodotThreadSafetyGuard::acquire(&ctx.godot_single_thread)
        .await
        .ok_or(anyhow::Error::msg("Failed to acquire thread safety guard"))?;

    // Process GLTF using Godot (all Godot objects are scoped here to drop before await)
    let (scene_path, file_size) = {
        // Load the GLTF using Godot
        let mut new_gltf = GltfDocument::new_gd();
        let mut new_gltf_state = GltfState::new_gd();

        let mappings = VarDictionary::from_iter(
            dependencies_hash
                .iter()
                .map(|(file_path, hash)| (file_path.to_variant(), hash.to_variant())),
        );

        new_gltf_state.set_additional_data("base_path", &"some".to_variant());
        new_gltf_state.set_additional_data("mappings", &mappings.to_variant());

        let file_path_gstr = GString::from(absolute_file_path.as_str());
        let base_path_gstr = GString::from(ctx.content_folder.as_str());
        let err = new_gltf
            .append_from_file_ex(&file_path_gstr, &new_gltf_state.clone())
            .base_path(&base_path_gstr)
            .flags(0)
            .done();

        if err != Error::OK {
            return Err(anyhow::Error::msg(format!("Error loading gltf: {:?}", err)));
        }

        let node = new_gltf
            .generate_scene(&new_gltf_state)
            .ok_or(anyhow::Error::msg("Error generating scene from gltf"))?;

        // Post-process textures
        let max_size = ctx.texture_quality.to_max_size();
        post_import_process(node.clone(), max_size);

        // Cast to Node3D and rotate
        let mut node = node
            .try_cast::<Node3D>()
            .map_err(|err| anyhow::Error::msg(format!("Error casting to Node3D: {err}")))?;
        node.rotate_y(std::f32::consts::PI);

        // Create colliders (with mask=0 initially - will be set by gltf_container.gd after loading)
        let root_node = node.clone();
        create_scene_colliders(node.clone().upcast(), root_node.clone());

        // Save the processed scene to disk (in the same cache folder as other content)
        let scene_path = get_scene_path_for_hash(&ctx.content_folder, &file_hash);
        save_node_as_scene(node.clone(), &scene_path).map_err(anyhow::Error::msg)?;

        // Get file size synchronously (std::fs is fine here, it's just a stat call)
        let file_size = std::fs::metadata(&scene_path)
            .map(|m| m.len() as i64)
            .unwrap_or(0);

        // Count nodes before freeing
        let node_count = count_nodes(node.clone().upcast());
        tracing::info!(
            "GLTF processed: {} with {} nodes, saved to {} ({} bytes)",
            file_hash,
            node_count,
            scene_path,
            file_size
        );

        // Free the node since we've saved it to disk
        // IMPORTANT: Use free() instead of queue_free() for orphan nodes processed on background threads
        node.free();

        (scene_path, file_size)
    };
    // All Godot objects are now dropped, safe to await

    // Register the saved scene in resource_provider for cache management
    ctx.resource_provider
        .register_local_file(&scene_path, file_size)
        .await;

    // Cleanup source GLTF file after successful save
    // NOTE: We only delete the main GLTF file, NOT dependencies (textures/buffers).
    // Dependencies may be shared by multiple GLTFs loading in parallel.
    // They will be cleaned up by LRU eviction when the cache exceeds its limit.
    ctx.resource_provider
        .try_delete_file_by_hash(&file_hash)
        .await;

    Ok(scene_path)
}

/// Count the number of nodes in a tree
fn count_nodes(node: Gd<Node>) -> i32 {
    let mut count = 1;
    for child in node.get_children().iter_shared() {
        count += count_nodes(child);
    }
    count
}

/// Get the StaticBody3D collider from a MeshInstance3D (created by create_trimesh_collision)
fn get_static_body_collider(mesh_instance: &Gd<MeshInstance3D>) -> Option<Gd<StaticBody3D>> {
    for maybe_static_body in mesh_instance.get_children().iter_shared() {
        if let Ok(static_body_3d) = maybe_static_body.try_cast::<StaticBody3D>() {
            return Some(static_body_3d);
        }
    }
    None
}

/// Create colliders for all mesh instances in a scene GLTF
/// Note: Colliders are created with mask=0 (disabled) and no scene_id/entity_id.
/// The masks and metadata should be set by the caller after instantiating the scene.
fn create_scene_colliders(node_to_inspect: Gd<Node>, root_node: Gd<Node3D>) {
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

            // First check if there's already a StaticBody3D (created by create_trimesh_collision)
            let mut static_body_3d = get_static_body_collider(&mesh_instance_3d);
            if static_body_3d.is_none() {
                mesh_instance_3d.create_trimesh_collision();
                static_body_3d = get_static_body_collider(&mesh_instance_3d);
            }

            if let Some(mut static_body_3d) = static_body_3d {
                // Create AnimatableBody3D to replace StaticBody3D
                let mut animatable_body = AnimatableBody3D::new_alloc();
                animatable_body.set_sync_to_physics(false);
                animatable_body.set_process_mode(ProcessMode::DISABLED);
                animatable_body.set_meta("dcl_col", &0.to_variant());
                animatable_body.set_meta("invisible_mesh", &invisible_mesh.to_variant());
                animatable_body.set_collision_layer(0);
                animatable_body.set_collision_mask(0);
                let colgen_name = format!("{}_colgen", mesh_instance_3d.get_name());
                animatable_body.set_name(&colgen_name);

                // Get the parent to add the new body
                if let Some(mut parent) = static_body_3d.get_parent() {
                    parent.add_child(&animatable_body.clone().upcast::<Node>());

                    // Move collision shapes from StaticBody3D to AnimatableBody3D
                    for mut body_child in static_body_3d
                        .get_children_ex()
                        .include_internal(true)
                        .done()
                        .iter_shared()
                    {
                        static_body_3d.remove_child(&body_child.clone());
                        body_child.call("set_owner", &[godot::builtin::Variant::nil()]);
                        animatable_body.add_child(&body_child.clone());

                        // Enable backface collision for concave shapes
                        if let Ok(collision_shape_3d) =
                            body_child.clone().try_cast::<CollisionShape3D>()
                        {
                            if let Some(shape) = collision_shape_3d.get_shape() {
                                if let Ok(mut concave_polygon_shape_3d) =
                                    shape.try_cast::<ConcavePolygonShape3D>()
                                {
                                    concave_polygon_shape_3d.set_backface_collision_enabled(true);
                                }
                            }
                        }

                        // Set owner to root so it gets saved with PackedScene
                        body_child.set_owner(&root_node.clone().upcast::<Node>());
                    }

                    // Remove the old StaticBody3D and free it immediately
                    parent.remove_child(&static_body_3d.clone().upcast::<Node>());
                    static_body_3d.free();

                    // Set owner for AnimatableBody3D
                    animatable_body.set_owner(&root_node.clone().upcast::<Node>());
                }
            }
        }

        create_scene_colliders(child, root_node.clone());
    }
}

// ============================================================================
// Wearable GLTF Loading (for ContentProvider wearable loading)
// ============================================================================

/// Load and save a wearable GLTF to disk
///
/// This function:
/// 1. Downloads the GLTF and its dependencies
/// 2. Loads it into Godot
/// 3. Processes textures
/// 4. Saves the processed scene to disk (NO colliders - wearables don't need them)
///
/// Returns the path to the saved scene file on success
pub async fn load_and_save_wearable_gltf(
    file_path: String,
    file_hash: String,
    content_mapping: ContentMappingAndUrlRef,
    ctx: SceneGltfContext,
) -> Result<String, anyhow::Error> {
    // Download the main GLTF file
    let base_path = Arc::new(get_base_dir(&file_path));
    let url = format!("{}{}", content_mapping.base_url, file_hash);
    let absolute_file_path = format!("{}{}", ctx.content_folder, file_hash);

    ctx.resource_provider
        .fetch_resource(url, file_hash.clone(), absolute_file_path.clone())
        .await
        .map_err(anyhow::Error::msg)?;

    // Get dependencies from the GLTF file
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

    // Check all dependencies are available
    if dependencies.iter().any(|(_, hash)| hash.is_none()) {
        return Err(anyhow::Error::msg(
            "There are some missing dependencies in the gltf",
        ));
    }

    let dependencies_hash: Vec<(String, String)> = dependencies
        .into_iter()
        .map(|(file_path, hash)| (file_path, hash.unwrap()))
        .collect();

    // Download all dependencies in parallel
    let futures = dependencies_hash.iter().map(|(_, dependency_file_hash)| {
        let ctx = ctx.clone();
        let content_mapping = content_mapping.clone();
        async move {
            let url = format!("{}{}", content_mapping.base_url, dependency_file_hash);
            let absolute_file_path = format!("{}{}", ctx.content_folder, dependency_file_hash);
            ctx.resource_provider
                .fetch_resource(url, dependency_file_hash.clone(), absolute_file_path)
                .await
                .map_err(|e| format!("Dependency {} failed: {:?}", dependency_file_hash, e))
        }
    });

    let result = futures_util::future::join_all(futures).await;
    if result.iter().any(|res| res.is_err()) {
        let errors: Vec<String> = result.into_iter().filter_map(|res| res.err()).collect();
        return Err(anyhow::Error::msg(format!(
            "Error downloading gltf dependencies: {}",
            errors.join("\n")
        )));
    }

    // Acquire thread safety guard for Godot API access
    let _thread_guard = GodotThreadSafetyGuard::acquire(&ctx.godot_single_thread)
        .await
        .ok_or(anyhow::Error::msg("Failed to acquire thread safety guard"))?;

    // Process GLTF using Godot (all Godot objects are scoped here to drop before await)
    let (scene_path, file_size) = {
        // Load the GLTF using Godot
        let mut new_gltf = GltfDocument::new_gd();
        let mut new_gltf_state = GltfState::new_gd();

        let mappings = VarDictionary::from_iter(
            dependencies_hash
                .iter()
                .map(|(file_path, hash)| (file_path.to_variant(), hash.to_variant())),
        );

        new_gltf_state.set_additional_data("base_path", &"some".to_variant());
        new_gltf_state.set_additional_data("mappings", &mappings.to_variant());

        let file_path_gstr = GString::from(absolute_file_path.as_str());
        let base_path_gstr = GString::from(ctx.content_folder.as_str());
        let err = new_gltf
            .append_from_file_ex(&file_path_gstr, &new_gltf_state.clone())
            .base_path(&base_path_gstr)
            .flags(0)
            .done();

        if err != Error::OK {
            return Err(anyhow::Error::msg(format!(
                "Error loading wearable gltf: {:?}",
                err
            )));
        }

        let node = new_gltf
            .generate_scene(&new_gltf_state)
            .ok_or(anyhow::Error::msg("Error generating scene from gltf"))?;

        // Post-process textures
        let max_size = ctx.texture_quality.to_max_size();
        post_import_process(node.clone(), max_size);

        // Attach ResourceLocker to track the asset's lifecycle
        ResourceLocker::attach_to(node.clone());

        // Cast to Node3D and rotate
        let mut node = node
            .try_cast::<Node3D>()
            .map_err(|err| anyhow::Error::msg(format!("Error casting to Node3D: {err}")))?;
        node.rotate_y(std::f32::consts::PI);

        // NOTE: No colliders for wearables - they don't need collision shapes

        // Save the processed scene to disk
        let scene_path = get_wearable_path_for_hash(&ctx.content_folder, &file_hash);
        save_node_as_scene(node.clone(), &scene_path).map_err(anyhow::Error::msg)?;

        // Get file size synchronously
        let file_size = std::fs::metadata(&scene_path)
            .map(|m| m.len() as i64)
            .unwrap_or(0);

        // Count nodes before freeing
        let node_count = count_nodes(node.clone().upcast());
        tracing::info!(
            "Wearable GLTF processed: {} with {} nodes, saved to {} ({} bytes)",
            file_hash,
            node_count,
            scene_path,
            file_size
        );

        // Free the node since we've saved it to disk
        node.free();

        (scene_path, file_size)
    };

    // Register the saved scene in resource_provider for cache management
    ctx.resource_provider
        .register_local_file(&scene_path, file_size)
        .await;

    // Cleanup source GLTF file after successful save
    // NOTE: We only delete the main GLTF file, NOT dependencies (textures/buffers).
    // Dependencies may be shared by multiple GLTFs loading in parallel.
    // They will be cleaned up by LRU eviction when the cache exceeds its limit.
    ctx.resource_provider
        .try_delete_file_by_hash(&file_hash)
        .await;

    Ok(scene_path)
}

// ============================================================================
// Emote GLTF Loading (for ContentProvider emote loading)
// ============================================================================

/// Load and save an emote GLTF to disk
///
/// This function:
/// 1. Downloads the GLTF and its dependencies
/// 2. Loads it into Godot
/// 3. Processes textures
/// 4. Saves the processed scene to disk as plain Node3D
///    (Animation extraction happens when loading from cache via load_cached_emote)
///
/// Returns the path to the saved scene file on success
pub async fn load_and_save_emote_gltf(
    file_path: String,
    file_hash: String,
    content_mapping: ContentMappingAndUrlRef,
    ctx: SceneGltfContext,
) -> Result<String, anyhow::Error> {
    // Download the main GLTF file
    let base_path = Arc::new(get_base_dir(&file_path));
    let url = format!("{}{}", content_mapping.base_url, file_hash);
    let absolute_file_path = format!("{}{}", ctx.content_folder, file_hash);

    ctx.resource_provider
        .fetch_resource(url, file_hash.clone(), absolute_file_path.clone())
        .await
        .map_err(anyhow::Error::msg)?;

    // Get dependencies from the GLTF file
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

    // Check all dependencies are available
    if dependencies.iter().any(|(_, hash)| hash.is_none()) {
        return Err(anyhow::Error::msg(
            "There are some missing dependencies in the gltf",
        ));
    }

    let dependencies_hash: Vec<(String, String)> = dependencies
        .into_iter()
        .map(|(file_path, hash)| (file_path, hash.unwrap()))
        .collect();

    // Download all dependencies in parallel
    let futures = dependencies_hash.iter().map(|(_, dependency_file_hash)| {
        let ctx = ctx.clone();
        let content_mapping = content_mapping.clone();
        async move {
            let url = format!("{}{}", content_mapping.base_url, dependency_file_hash);
            let absolute_file_path = format!("{}{}", ctx.content_folder, dependency_file_hash);
            ctx.resource_provider
                .fetch_resource(url, dependency_file_hash.clone(), absolute_file_path)
                .await
                .map_err(|e| format!("Dependency {} failed: {:?}", dependency_file_hash, e))
        }
    });

    let result = futures_util::future::join_all(futures).await;
    if result.iter().any(|res| res.is_err()) {
        let errors: Vec<String> = result.into_iter().filter_map(|res| res.err()).collect();
        return Err(anyhow::Error::msg(format!(
            "Error downloading gltf dependencies: {}",
            errors.join("\n")
        )));
    }

    // Acquire thread safety guard for Godot API access
    let _thread_guard = GodotThreadSafetyGuard::acquire(&ctx.godot_single_thread)
        .await
        .ok_or(anyhow::Error::msg("Failed to acquire thread safety guard"))?;

    // Process GLTF using Godot (all Godot objects are scoped here to drop before await)
    let (scene_path, file_size) = {
        // Load the GLTF using Godot
        let mut new_gltf = GltfDocument::new_gd();
        let mut new_gltf_state = GltfState::new_gd();

        let mappings = VarDictionary::from_iter(
            dependencies_hash
                .iter()
                .map(|(file_path, hash)| (file_path.to_variant(), hash.to_variant())),
        );

        new_gltf_state.set_additional_data("base_path", &"some".to_variant());
        new_gltf_state.set_additional_data("mappings", &mappings.to_variant());

        let file_path_gstr = GString::from(absolute_file_path.as_str());
        let base_path_gstr = GString::from(ctx.content_folder.as_str());
        let err = new_gltf
            .append_from_file_ex(&file_path_gstr, &new_gltf_state.clone())
            .base_path(&base_path_gstr)
            .flags(0)
            .done();

        if err != Error::OK {
            return Err(anyhow::Error::msg(format!(
                "Error loading emote gltf: {:?}",
                err
            )));
        }

        let node = new_gltf
            .generate_scene(&new_gltf_state)
            .ok_or(anyhow::Error::msg("Error generating scene from gltf"))?;

        // Post-process textures
        let max_size = ctx.texture_quality.to_max_size();
        post_import_process(node.clone(), max_size);

        // Cast to Node3D and rotate (same as internal_load_gltf does)
        let mut node = node
            .try_cast::<Node3D>()
            .map_err(|err| anyhow::Error::msg(format!("Error casting to Node3D: {err}")))?;

        // Apply 180° rotation - same as internal_load_gltf
        node.rotate_y(std::f32::consts::PI);

        // Extract animations in background thread
        let (armature_prop, default_animation, prop_animation) =
            process_emote_animations(&file_hash, &node)
                .ok_or(anyhow::Error::msg("Failed to extract emote animations"))?;

        // Create EmoteRoot node to save with embedded animations
        let mut root = Node3D::new_alloc();
        root.set_name(&StringName::from("EmoteRoot"));

        // Add armature_prop as child if present
        if let Some(mut prop) = armature_prop {
            // Remove from original parent if any
            if let Some(mut parent) = prop.get_parent() {
                parent.remove_child(&prop.clone().upcast::<Node>());
            }
            root.add_child(&prop.clone().upcast::<Node>());
            prop.set_owner(&root.clone().upcast::<Node>());
        }

        // Create AnimationPlayer with processed animations
        let mut anim_player = AnimationPlayer::new_alloc();
        anim_player.set_name(&StringName::from("EmoteAnimations"));

        let mut anim_library = AnimationLibrary::new_gd();
        if let Some(ref anim) = default_animation {
            anim_library.add_animation(&StringName::from(&anim.get_name()), anim);
        }
        if let Some(ref anim) = prop_animation {
            anim_library.add_animation(&StringName::from(&anim.get_name()), anim);
        }
        anim_player.add_animation_library(&StringName::from(""), &anim_library);

        root.add_child(&anim_player.clone().upcast::<Node>());
        anim_player.set_owner(&root.clone().upcast::<Node>());

        // Save the EmoteRoot scene to disk
        let scene_path = get_emote_path_for_hash(&ctx.content_folder, &file_hash);
        save_node_as_scene(root.clone(), &scene_path).map_err(anyhow::Error::msg)?;

        // Get file size synchronously
        let file_size = std::fs::metadata(&scene_path)
            .map(|m| m.len() as i64)
            .unwrap_or(0);

        // Count nodes before freeing
        let node_count = count_nodes(root.clone().upcast());
        tracing::info!(
            "Emote GLTF processed with embedded animations: {} with {} nodes, saved to {} ({} bytes)",
            file_hash,
            node_count,
            scene_path,
            file_size
        );

        // Free both nodes since we've saved to disk
        node.free();
        root.free();

        (scene_path, file_size)
    };

    // Register the saved scene in resource_provider for cache management
    ctx.resource_provider
        .register_local_file(&scene_path, file_size)
        .await;

    // Cleanup source GLTF file after successful save
    // NOTE: We only delete the main GLTF file, NOT dependencies (textures/buffers).
    // Dependencies may be shared by multiple GLTFs loading in parallel.
    // They will be cleaned up by LRU eviction when the cache exceeds its limit.
    ctx.resource_provider
        .try_delete_file_by_hash(&file_hash)
        .await;

    Ok(scene_path)
}
