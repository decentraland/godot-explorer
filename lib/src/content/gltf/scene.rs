//! Scene GLTF loading (for ContentProvider scene loading).

use godot::{
    classes::{
        geometry_instance_3d::ShadowCastingSetting, node::ProcessMode, ArrayMesh, CollisionShape3D,
        ConcavePolygonShape3D, Mesh, MeshInstance3D, Node, Node3D, StaticBody3D,
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
use crate::godot_classes::dcl_global::DclGlobal;
use crate::scene_runner::components::mesh_lod::lod_baker::bake_shadow_mesh;

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

            if shadow_mesh_enabled() {
                let (paired, fallback) = apply_shadow_mesh(&root_node);
                tracing::info!(
                    "[shadow-mesh] {}: paired={} fallback={}",
                    hash,
                    paired,
                    fallback
                );
            }

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

                // Enable backface collision only for volumetric meshes.
                // Unity's PhysX MeshCollider is always double-sided for physics, so volumetric
                // meshes (streets, ramps, buildings) need backface collision to prevent falling
                // through. Thin/planar meshes (one-way colliders) are left single-sided so they
                // only block from the front face direction.
                let is_planar = is_mesh_planar(&mesh_instance_3d);
                if !is_planar {
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
                                if let Ok(mut concave) = shape.try_cast::<ConcavePolygonShape3D>() {
                                    concave.set_backface_collision_enabled(true);
                                }
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

fn shadow_mesh_enabled() -> bool {
    DclGlobal::try_singleton()
        .map(|g| g.bind().cli.bind().shadow_mesh_enabled)
        .unwrap_or(false)
}

/// Per `Node3D` parent in a scene GLTF, pair each visible `MeshInstance3D`
/// with its sibling `*collider*` MI's mesh resource as the visible mesh's
/// `ArrayMesh.shadow_mesh`. Falls back to a stride-decimated bake when a
/// visible MI has no collider sibling.
///
/// The renderer rasterizes `shadow_mesh` (a cheaper substitute) into the
/// directional shadow map during the shadow pass while keeping the
/// full-detail source for the visible pass. Collider MIs themselves stay
/// `visible=false` for physics-only use.
///
/// Returns `(paired_count, fallback_baked_count)` for logging.
///
/// Skipped:
/// - meshes with blend shapes (morph-target shadows need the full mesh)
/// - collider MIs that have no visible sibling (e.g. trigger-only GLBs)
pub(super) fn apply_shadow_mesh(root_node: &Gd<Node3D>) -> (u32, u32) {
    let mut paired = 0u32;
    let mut fallback = 0u32;
    apply_shadow_mesh_recursive(&root_node.clone().upcast(), &mut paired, &mut fallback);
    (paired, fallback)
}

fn apply_shadow_mesh_recursive(node: &Gd<Node>, paired: &mut u32, fallback: &mut u32) {
    let mut visible_mis: Vec<Gd<MeshInstance3D>> = Vec::new();
    let mut collider_mesh: Option<Gd<Mesh>> = None;
    for child in node.get_children().iter_shared() {
        if let Ok(mi) = child.clone().try_cast::<MeshInstance3D>() {
            let is_collider = mi
                .get_name()
                .to_string()
                .to_lowercase()
                .contains("collider");
            if is_collider {
                if collider_mesh.is_none() {
                    collider_mesh = mi.get_mesh();
                }
            } else {
                visible_mis.push(mi);
            }
        }
    }

    for mut mi in visible_mis {
        let Some(mesh) = mi.get_mesh() else { continue };
        let Ok(am) = mesh.try_cast::<ArrayMesh>() else {
            continue;
        };
        if am.get_blend_shape_count() > 0 {
            continue;
        }
        let mut am_mut = am.clone();

        if let Some(coll_mesh) = collider_mesh.clone() {
            if let Ok(coll_am) = coll_mesh.try_cast::<ArrayMesh>() {
                am_mut.set_shadow_mesh(&coll_am);
                // Defensive: a previous legacy proxy pass may have flipped
                // this off. apply_shadow_mesh always casts shadow from the
                // visible MI (via the shadow_mesh slot).
                mi.set_cast_shadows_setting(ShadowCastingSetting::ON);
                *paired += 1;
                continue;
            }
        }
        if let Some(result) = bake_shadow_mesh(&am) {
            am_mut.set_shadow_mesh(&result.mesh);
            mi.set_cast_shadows_setting(ShadowCastingSetting::ON);
            *fallback += 1;
        }
    }

    for child in node.get_children().iter_shared() {
        apply_shadow_mesh_recursive(&child, paired, fallback);
    }
}

/// Minimum thickness (in any axis) below which a mesh is considered planar/one-way.
const PLANAR_THICKNESS_THRESHOLD: f32 = 0.01;

/// Check if a mesh is essentially planar (very thin in at least one axis).
/// Planar meshes are used as one-way colliders and should NOT have backface collision.
fn is_mesh_planar(mesh_instance: &Gd<MeshInstance3D>) -> bool {
    if let Some(mesh) = mesh_instance.get_mesh() {
        let aabb = mesh.get_aabb();
        let size = aabb.size;
        size.x < PLANAR_THICKNESS_THRESHOLD
            || size.y < PLANAR_THICKNESS_THRESHOLD
            || size.z < PLANAR_THICKNESS_THRESHOLD
    } else {
        false
    }
}
