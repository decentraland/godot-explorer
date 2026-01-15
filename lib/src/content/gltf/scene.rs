//! Scene GLTF loading (for ContentProvider scene loading).

use godot::{
    classes::{
        node::ProcessMode, CollisionShape3D, ConcavePolygonShape3D, MeshInstance3D, Node, Node3D,
        StaticBody3D,
    },
    meta::ToGodot,
    obj::Gd,
};

use super::super::{
    content_mapping::ContentMappingAndUrlRef,
    content_provider::SceneGltfContext,
    scene_saver::{get_scene_path_for_hash, save_node_as_scene},
};
use super::common::{count_nodes, load_gltf_pipeline};

/// Load and save a scene GLTF to disk.
///
/// This function:
/// 1. Downloads the GLTF and its dependencies
/// 2. Loads it into Godot
/// 3. Processes textures
/// 4. Creates colliders (with mask=0 - caller sets masks after instantiating)
/// 5. Saves the processed scene to disk
///
/// Returns the path to the saved scene file on success.
pub async fn load_and_save_scene_gltf(
    file_path: String,
    file_hash: String,
    content_mapping: ContentMappingAndUrlRef,
    ctx: SceneGltfContext,
) -> Result<String, anyhow::Error> {
    let ctx_clone = ctx.clone();

    let (scene_path, file_size) = load_gltf_pipeline(
        file_path,
        file_hash.clone(),
        content_mapping,
        ctx,
        |node, hash, ctx| {
            // Create colliders (with mask=0 initially - will be set by gltf_container.gd after loading)
            let root_node = node.clone();
            create_scene_colliders(node.clone().upcast(), root_node.clone());

            // Save the processed scene to disk (in the same cache folder as other content)
            let scene_path = get_scene_path_for_hash(&ctx.content_folder, hash);
            save_node_as_scene(node.clone(), &scene_path).map_err(anyhow::Error::msg)?;

            // Get file size synchronously (std::fs is fine here, it's just a stat call)
            let file_size = std::fs::metadata(&scene_path)
                .map(|m| m.len() as i64)
                .unwrap_or(0);

            // Count nodes before freeing
            let node_count = count_nodes(node.clone().upcast());
            tracing::debug!(
                "GLTF processed: {} with {} nodes, saved to {} ({} bytes)",
                hash,
                node_count,
                scene_path,
                file_size
            );

            // Free the node since we've saved it to disk
            // IMPORTANT: Use free() instead of queue_free() for orphan nodes processed on background threads
            node.free();

            Ok((scene_path, file_size))
        },
    )
    .await?;

    // Register the saved scene in resource_provider for cache management
    ctx_clone
        .resource_provider
        .register_local_file(&scene_path, file_size)
        .await;

    Ok(scene_path)
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

/// Create colliders for all mesh instances in a scene GLTF.
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
                // Keep StaticBody3D - we'll use PhysicsServer3D to switch to KINEMATIC when needed
                // This is simpler than replacing with AnimatableBody3D
                static_body_3d.set_process_mode(ProcessMode::DISABLED);
                static_body_3d.set_meta("dcl_col", &0.to_variant());
                static_body_3d.set_meta("invisible_mesh", &invisible_mesh.to_variant());
                static_body_3d.set_collision_layer(0);
                static_body_3d.set_collision_mask(0);
                let colgen_name = format!("{}_colgen", mesh_instance_3d.get_name());
                static_body_3d.set_name(&colgen_name);

                // Enable backface collision for concave shapes
                for body_child in static_body_3d
                    .get_children_ex()
                    .include_internal(true)
                    .done()
                    .iter_shared()
                {
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
                }

                // Set owner so it gets saved with PackedScene
                static_body_3d.set_owner(&root_node.clone().upcast::<Node>());
            }
        }

        create_scene_colliders(child, root_node.clone());
    }
}
