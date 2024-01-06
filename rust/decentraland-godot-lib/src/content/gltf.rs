use std::sync::Arc;

use godot::{
    builtin::{meta::ToGodot, Dictionary, GString},
    engine::{
        file_access::ModeFlags, global::Error, node::ProcessMode, AnimatableBody3D,
        CollisionShape3D, ConcavePolygonShape3D, FileAccess, GdScript, GltfDocument, GltfState,
        MeshInstance3D, Node, Node3D, StaticBody3D,
    },
    obj::Gd,
};

use crate::{
    godot_classes::promise::Promise,
    http_request::request_response::{RequestOption, ResponseType},
};

use super::{content_mapping::ContentMappingAndUrlRef, content_provider::ContentProviderContext};

fn reject_promise(get_promise: impl Fn() -> Option<Gd<Promise>>, reason: String) {
    if let Some(mut promise) = get_promise() {
        promise.call_deferred("reject".into(), &[reason.to_variant()]);
    }
}

fn get_dependencies(file_path: &String) -> Vec<String> {
    let mut dependencies = Vec::new();
    let Some(mut p_file) = FileAccess::open(GString::from(&file_path), ModeFlags::READ) else {
        return dependencies;
    };

    p_file.seek(0);

    let magic = p_file.get_32();
    let maybe_json: Result<serde_json::Value, serde_json::Error> = if magic == 0x46546C67 {
        p_file.get_32(); // version
        p_file.get_32(); // length

        let chunk_length = p_file.get_32();
        p_file.get_32(); // chunk_type

        let json_data = p_file.get_buffer(chunk_length as i64);
        serde_json::de::from_slice(json_data.as_slice())
    } else {
        p_file.seek(0);
        let json_data = p_file.get_buffer(p_file.get_length() as i64);
        serde_json::de::from_slice(json_data.as_slice())
    };

    if maybe_json.is_err() {
        return dependencies;
    }

    let json = maybe_json.unwrap();

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

    dependencies
}

pub fn get_base_dir(file_path: &String) -> String {
    let last_slash = file_path.rfind("/");
    if let Some(last_slash) = last_slash {
        return file_path[0..last_slash].to_string();
    }
    return "".to_string();
}

pub async fn load_gltf(
    file_path: String,
    content_mapping: ContentMappingAndUrlRef,
    get_promise: impl Fn() -> Option<Gd<Promise>>,
    ctx: ContentProviderContext,
) {
    let base_path = Arc::new(get_base_dir(&file_path));

    let Some(file_hash) = content_mapping.content.get(&file_path) else {
        reject_promise(
            get_promise,
            "File not found in the content mappings".to_string(),
        );
        return;
    };

    let absolute_file_path = format!("{}{}", ctx.content_folder, file_hash);
    if !FileAccess::file_exists(GString::from(&absolute_file_path)) {
        let request = RequestOption::new(
            0,
            format!("{}{}", content_mapping.base_url, file_hash),
            http::Method::GET,
            ResponseType::ToFile(absolute_file_path.clone()),
            None,
            None,
        );

        match ctx.http_queue_requester.request(request, 0).await {
            Ok(_response) => {}
            Err(_err) => {
                reject_promise(get_promise, "Error downloading gltf".to_string());
                return;
            }
        }
    }

    let dependencies = get_dependencies(&absolute_file_path)
        .into_iter()
        .map(|dep| {
            let full_path = if base_path.is_empty() {
                dep.clone()
            } else {
                format!("{}/{}", base_path, dep)
            }
            .to_lowercase();

            let item = content_mapping
                .content
                .get(&full_path)
                .map(|item| item.clone());
            (dep, item)
        })
        .collect::<Vec<(String, Option<String>)>>();

    if dependencies.iter().any(|(_, hash)| hash.is_none()) {
        reject_promise(
            get_promise,
            "There are some missing dependencies in the gltf".to_string(),
        );
        return;
    }

    let dependencies_hash = dependencies
        .into_iter()
        .map(|(file_path, hash)| (file_path, hash.unwrap()))
        .collect::<Vec<(String, String)>>();

    let futures = dependencies_hash.iter().map(|(_, file_hash)| {
        let ctx = ctx.clone();
        let absolute_file_path = format!("{}{}", ctx.content_folder, file_hash);
        let content_mapping = content_mapping.clone();
        async move {
            if !FileAccess::file_exists(GString::from(&absolute_file_path)) {
                let request = RequestOption::new(
                    0,
                    format!("{}{}", content_mapping.base_url, file_hash),
                    http::Method::GET,
                    ResponseType::ToFile(absolute_file_path.clone()),
                    None,
                    None,
                );

                match ctx.http_queue_requester.request(request, 0).await {
                    Ok(_response) => Ok(()),
                    Err(_err) => Err(()),
                }
            } else {
                Ok(())
            }
        }
    });

    let result = futures_util::future::join_all(futures).await;
    if result.iter().any(|res| res.is_err()) {
        reject_promise(
            get_promise,
            "Error downloading gltf dependencies".to_string(),
        );
        return;
    }

    let Some(mut promise) = get_promise() else {
        return;
    };

    set_thread_safety_checks_enabled(false);

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
        reject_promise(
            get_promise,
            format!("Error loading gltf after appending from file"),
        );
        return;
    }

    let Some(node) = new_gltf.generate_scene(new_gltf_state) else {
        reject_promise(
            get_promise,
            "Error loading gltf when generating scene".to_string(),
        );
        return;
    };

    let Ok(mut node) = node.try_cast::<Node3D>() else {
        reject_promise(
            get_promise,
            "Error loading gltf when casting to Node3D".to_string(),
        );
        return;
    };

    node.rotate_y(std::f32::consts::PI);

    if let Some(mut promise) = get_promise() {
        promise.call_deferred("resolve_with_data".into(), &[node.to_variant()]);
    }

    create_colliders(node.upcast());
    set_thread_safety_checks_enabled(true);
}

pub fn set_thread_safety_checks_enabled(enabled: bool) {
    let mut temp_script = godot::engine::load::<GdScript>("res://src/logic/thread_safety.gd");
    temp_script.call(
        "set_thread_safety_checks_enabled".into(),
        &[enabled.to_variant()],
    );
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
                .find("_collider")
                .is_some();

            let mut static_body_3d = get_collider(&mesh_instance_3d);
            if static_body_3d.is_none() {
                mesh_instance_3d.create_trimesh_collision();
                static_body_3d = get_collider(&mesh_instance_3d);
                if static_body_3d.is_none() {
                    create_colliders(child);
                    continue;
                }
            }

            if let Some(mut static_body_3d) = static_body_3d {
                let Some(mut parent) = static_body_3d.get_parent() else {
                    create_colliders(child);
                    continue;
                };
                static_body_3d.set_name(GString::from(format!(
                    "{}_colgen",
                    mesh_instance_3d.get_name()
                )));

                let mut new_animatable = AnimatableBody3D::new_alloc();
                new_animatable.set_sync_to_physics(false);
                new_animatable.set_process_mode(ProcessMode::PROCESS_MODE_DISABLED);
                new_animatable.set_meta("dcl_col".into(), 0.to_variant());
                new_animatable.set_meta("invisible_mesh".into(), invisible_mesh.to_variant());
                new_animatable.set_collision_layer(0);
                new_animatable.set_collision_mask(0);

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

            if invisible_mesh {
                mesh_instance_3d.set_visible(false);
            }
            create_colliders(child);
        } else {
            create_colliders(child);
        }
    }
}
