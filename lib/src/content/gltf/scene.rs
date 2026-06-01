//! Scene GLTF loading (for ContentProvider scene loading).

use godot::{
    classes::{
        base_material_3d::{CullMode, ShadingMode, Transparency},
        geometry_instance_3d::ShadowCastingSetting,
        node::ProcessMode,
        ArrayMesh, BaseMaterial3D, CollisionShape3D, ConcavePolygonShape3D, MeshInstance3D, Node,
        Node3D, StandardMaterial3D, StaticBody3D,
    },
    prelude::*,
};

use crate::godot_classes::dcl_global::DclGlobal;

/// Shared material override for shadow-proxy colliders: cull FRONT faces so
/// only the inner (back) faces rasterize into the shadow map. Because DCL
/// colliders are slightly larger than the visible mesh they wrap, leaving
/// front-face culling on would self-shadow the visible mesh. PER_VERTEX
/// shading keeps the shader path consistent with the importer's cheap-pbr
/// path; transparency is OFF, alpha not used.
fn build_shadow_proxy_material() -> Gd<BaseMaterial3D> {
    let mut mat = StandardMaterial3D::new_gd();
    mat.set_cull_mode(CullMode::FRONT);
    mat.set_shading_mode(ShadingMode::PER_VERTEX);
    mat.set_transparency(Transparency::DISABLED);
    mat.upcast()
}

use super::super::{
    content_mapping::ContentMappingAndUrlRef,
    content_provider::SceneGltfContext,
    scene_saver::{get_scene_path_for_hash, save_node_as_scene},
};
use super::common::{count_nodes, load_gltf_pipeline};

use crate::scene_runner::components::asset_preprocessor::mesh_occluder;

struct AssetServerPreprocCounts {
    occluders: u32,
}

fn apply_asset_server_optimizations(root: &Gd<Node3D>, hash: &str) {
    let mut counts = AssetServerPreprocCounts { occluders: 0 };
    walk_and_preprocess(&root.clone().upcast(), &mut counts);
    if counts.occluders > 0 {
        godot::global::godot_print!(
            "[asset-server-preproc] {}: occluders={}",
            hash,
            counts.occluders
        );
    }
}

fn walk_and_preprocess(node: &Gd<Node>, counts: &mut AssetServerPreprocCounts) {
    if let Ok(mut mi) = node.clone().try_cast::<MeshInstance3D>() {
        if mi.is_visible_in_tree() && mi.get_layer_mask() == 1 {
            if let Some(mesh) = mi.get_mesh() {
                if let Ok(array_mesh) = mesh.try_cast::<ArrayMesh>() {
                    if mesh_occluder::try_spawn_for(&mut mi, &array_mesh) {
                        counts.occluders = counts.occluders.saturating_add(1);
                    }
                }
            }
        }
    }
    for child in node.get_children().iter_shared() {
        walk_and_preprocess(&child, counts);
    }
}

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

            // Auto-attach `OccluderInstance3D` siblings on big opaque
            // meshes so Godot's culler can early-out everything behind
            // them at runtime. Baked into the saved `.scn` so the
            // device pays zero generation cost.
            //
            // Gated on `--asset-server`: this code path also runs when
            // the cliente loads a fresh GLTF (no cache yet), and
            // re-applying it on device duplicates the occluders that
            // are already baked in the optimized `.scn`. Limiting to
            // asset-server mode keeps the device-side path lean.
            let in_asset_server_mode = DclGlobal::try_singleton()
                .map(|g| g.bind().cli.bind().asset_server)
                .unwrap_or(false);
            if in_asset_server_mode {
                apply_asset_server_optimizations(&root_node, hash);
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
    // Shadow_proxy permanently disabled. The old approach (collider
    // SHADOWS_ONLY + visible cast_shadow=OFF) saved shadow-pass cost
    // but disrupted the GP zeppelin animation (cast_shadow=OFF on
    // the visible MI breaks its AnimationPlayer drive). The
    // visible mesh now casts shadow normally; Godot's LOD selector
    // picks LOD3 at distance for the shadow pass, which keeps the
    // cost low without the dual-MI trick.
    let shadow_proxy = false;
    create_scene_colliders_inner(node_to_inspect, root_node, shadow_proxy, &mut None);
}

fn create_scene_colliders_inner(
    node_to_inspect: Gd<Node>,
    root_node: Gd<Node3D>,
    shadow_proxy: bool,
    shadow_proxy_mat: &mut Option<Gd<BaseMaterial3D>>,
) {
    for child in node_to_inspect.get_children().iter_shared() {
        if let Ok(mut mesh_instance_3d) = child.clone().try_cast::<MeshInstance3D>() {
            let invisible_mesh = mesh_instance_3d
                .get_name()
                .to_string()
                .to_lowercase()
                .contains("collider");

            if invisible_mesh {
                if shadow_proxy {
                    // Keep the collider in the visible tree (so the renderer
                    // submits it to the shadow pass) but flag it as
                    // SHADOW_ONLY — the visible pass skips it entirely.
                    mesh_instance_3d.set_cast_shadows_setting(ShadowCastingSetting::SHADOWS_ONLY);
                    // Cull FRONT faces so only the inner back faces write to
                    // the shadow map. DCL colliders are slightly inflated
                    // relative to the visible mesh; without front-cull the
                    // collider's front face would project a shadow onto the
                    // visible mesh's surface (self-shadow acne / Peter
                    // Panning).
                    let mat = shadow_proxy_mat
                        .get_or_insert_with(build_shadow_proxy_material)
                        .clone();
                    mesh_instance_3d
                        .set_material_override(&mat.upcast::<godot::classes::Material>());
                } else {
                    mesh_instance_3d.set_visible(false);
                }
            } else if shadow_proxy {
                // Visible mesh in a scene that has a shadow proxy chain:
                // hand off shadow-casting to the collider sibling.
                mesh_instance_3d.set_cast_shadows_setting(ShadowCastingSetting::OFF);
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

        create_scene_colliders_inner(child, root_node.clone(), shadow_proxy, shadow_proxy_mat);
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
