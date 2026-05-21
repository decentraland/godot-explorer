//! Scene GLTF loading (for ContentProvider scene loading).

use std::collections::HashMap;

use godot::{
    classes::{
        base_material_3d::{CullMode, ShadingMode, Transparency},
        geometry_instance_3d::ShadowCastingSetting,
        mesh::{ArrayType, PrimitiveType},
        multi_mesh::TransformFormat,
        node::ProcessMode,
        AnimationPlayer, ArrayMesh, BaseMaterial3D, CollisionShape3D, ConcavePolygonShape3D,
        Material, Mesh, MeshInstance3D, MultiMesh, MultiMeshInstance3D, Node, Node3D, Skeleton3D,
        StandardMaterial3D, StaticBody3D,
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

/// Recursively walk the scene tree spawning mesh-shaped `OccluderInstance3D`
/// siblings on every visible MeshInstance3D that passes the size + opacity
/// filters in `mesh_occluder::try_spawn_for`. Counts how many were attached
/// so we can log the per-asset total.
fn spawn_mesh_occluders(node: &Gd<Node>, occluder_count: &mut u32) {
    if let Ok(mut mi) = node.clone().try_cast::<MeshInstance3D>() {
        if let Some(mesh) = mi.get_mesh() {
            if let Ok(am) = mesh.try_cast::<ArrayMesh>() {
                if mesh_occluder::try_spawn_for(&mut mi, &am) {
                    *occluder_count += 1;
                }
            }
        }
    }
    for child in node.get_children().iter_shared() {
        spawn_mesh_occluders(&child, occluder_count);
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

            // Merge static MIs — DISABLED. Import-time merge is unsound:
            // SDK7 can mutate any entity at runtime (GltfNodeModifier,
            // Tween, Animator, Material) and we can't predict that from
            // the .glb alone. Kept compiled but gated off; revisit when
            // SDK adds a `Static` declaration.
            if false
                && DclGlobal::try_singleton()
                    .map(|g| g.bind().cli.bind().cheap_pbr_enabled)
                    .unwrap_or(false)
            {
                let stats = merge_static_meshes(&root_node);
                tracing::info!(
                    "[mesh-merge] {}: eligible={} groups={} sources={} skipped_animated={}",
                    hash,
                    stats.eligible_mis,
                    stats.merged_groups,
                    stats.merged_sources,
                    stats.skipped_tree_animated
                );
            }

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
                let mut occluders_added = 0u32;
                spawn_mesh_occluders(&root_node.clone().upcast(), &mut occluders_added);
                if occluders_added > 0 {
                    godot::global::godot_print!(
                        "[occluder-gen] {}: occluders_added={}",
                        hash,
                        occluders_added
                    );
                }
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

/// Walk a subtree and report whether any MeshInstance3D in it has
/// "collider" in its name. We use this to decide whether the shadow-proxy
/// swap is safe for a given GLTF: scenes WITHOUT author-named collider
/// meshes would lose all shadows otherwise (visible MIs flipped to
/// cast_shadow=OFF with no proxy to replace them).
fn tree_has_named_collider(node: &Gd<Node>) -> bool {
    for child in node.get_children().iter_shared() {
        if let Ok(mi) = child.clone().try_cast::<MeshInstance3D>() {
            if mi
                .get_name()
                .to_string()
                .to_lowercase()
                .contains("collider")
            {
                return true;
            }
        }
        if tree_has_named_collider(&child) {
            return true;
        }
    }
    false
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

/// Walk a scene tree and, per Node3D parent, pair visible MIs with their
/// sibling `*collider*` MI. The collider's mesh becomes the visible
/// mesh's `shadow_mesh` — the renderer rasterizes the (already simpler)
/// collider geometry into the directional shadow map. Visible meshes
/// without a paired collider get a stride-decimated bake as fallback.
///
/// Returns (paired_count, fallback_baked_count) for logging.
///
/// Why this beats the old shadow_proxy approach:
/// - One MI per pair instead of two (visible + SHADOWS_ONLY proxy)
/// - Cleaner renderer state: visible MI casts shadow normally with the
///   `shadow_mesh` slot doing the substitution at shadow-pass time
/// - Collider MI stays in the tree purely for physics (its
///   StaticBody3D child), set_visible(false) — never rendered
///
/// Skipped:
/// - blend-shape meshes (morph-target shadows need the full mesh)
// apply_shadow_mesh + apply_shadow_mesh_recursive live in common.rs now —
// the per-GLB pipeline runs them at GLTF import time. This scene-level wrapper
// keeps the existing call site working by importing the new entry point.

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

// ============================================================================
// Import-time mesh merging
// ============================================================================
//
// Bake N MeshInstance3Ds that share the same Material instance into one
// combined ArrayMesh. World-space transforms (relative to the scene root)
// are folded into vertex positions so the merged MI sits at the root with
// IDENTITY transform.
//
// Runs BEFORE `create_scene_colliders` so trimesh colliders track the
// merged geometry. Persists into the .scn cache so the merge cost is paid
// once per content hash.
//
// Whole-tree skip when any `Skeleton3D` or `AnimationPlayer` exists:
// animations target node paths, and collapsing animated nodes would
// silently break their motion. DCL Genesis Plaza scene chunks are static
// and pass this check; user scenes with animated rigs are left untouched.

#[derive(Default, Debug)]
pub struct MergeStats {
    pub eligible_mis: u32,
    pub merged_groups: u32,
    pub merged_sources: u32,
    pub skipped_tree_animated: u32,
}

fn tree_has_skeleton_or_animation(node: &Gd<Node>) -> bool {
    for child in node.get_children().iter_shared() {
        if child.clone().try_cast::<AnimationPlayer>().is_ok() {
            return true;
        }
        if child.clone().try_cast::<Skeleton3D>().is_ok() {
            return true;
        }
        if tree_has_skeleton_or_animation(&child) {
            return true;
        }
    }
    false
}

struct MergeCandidate {
    mi: Gd<MeshInstance3D>,
    material_id: i64,
    world_xform: Transform3D,
}

/// Per-MI eligibility check. Returns `Some(material_instance_id)` when the
/// MI is safe to merge. Conservative on purpose — partial vertex formats
/// or non-triangle primitives are rejected outright rather than coerced.
fn mi_eligible_for_merge(mi: &Gd<MeshInstance3D>) -> Option<i64> {
    if mi
        .get_name()
        .to_string()
        .to_lowercase()
        .contains("collider")
    {
        return None;
    }
    let mesh = mi.get_mesh()?;
    if mesh.get_surface_count() != 1 {
        return None;
    }
    // `surface_get_primitive_type` is ArrayMesh-only; cast to access it.
    // Non-ArrayMesh source meshes are skipped — GLTF imports always
    // produce ArrayMesh, so this rejects nothing in the common path.
    let array_mesh = mesh.clone().try_cast::<ArrayMesh>().ok()?;
    if array_mesh.get_blend_shape_count() > 0 {
        return None;
    }
    if array_mesh.surface_get_primitive_type(0) != PrimitiveType::TRIANGLES {
        return None;
    }
    let arrays = array_mesh.surface_get_arrays(0);
    let bones_v = arrays.at(ArrayType::BONES.ord() as usize);
    let has_int_bones = bones_v
        .try_to::<PackedInt32Array>()
        .map(|a| !a.is_empty())
        .unwrap_or(false);
    let has_float_bones = bones_v
        .try_to::<PackedFloat32Array>()
        .map(|a| !a.is_empty())
        .unwrap_or(false);
    if has_int_bones || has_float_bones {
        return None;
    }
    let verts = arrays
        .at(ArrayType::VERTEX.ord() as usize)
        .try_to::<PackedVector3Array>()
        .ok()?;
    if verts.is_empty() {
        return None;
    }
    let material: Gd<Material> = mi.get_active_material(0)?;
    Some(material.instance_id().to_i64())
}

/// Accumulate Node3D local transforms walking from `mi` up to `root`.
/// For an orphan tree (the scene hasn't been added to the SceneTree yet)
/// `get_global_transform` isn't valid, so we compute it ourselves.
fn compute_relative_transform(mi: &Gd<MeshInstance3D>, root: &Gd<Node3D>) -> Transform3D {
    let mut xform = Transform3D::IDENTITY;
    let mut current: Gd<Node3D> = mi.clone().upcast();
    loop {
        if current.instance_id() == root.instance_id() {
            break;
        }
        xform = current.get_transform() * xform;
        let Some(parent) = current.get_parent() else {
            break;
        };
        match parent.try_cast::<Node3D>() {
            Ok(p) => current = p,
            Err(_) => break,
        }
    }
    xform
}

fn collect_merge_candidates(node: Gd<Node>, root: &Gd<Node3D>, out: &mut Vec<MergeCandidate>) {
    let mut descend = true;
    if let Ok(mi) = node.clone().try_cast::<MeshInstance3D>() {
        if let Some(material_id) = mi_eligible_for_merge(&mi) {
            let world_xform = compute_relative_transform(&mi, root);
            out.push(MergeCandidate {
                mi: mi.clone(),
                material_id,
                world_xform,
            });
            // MI children are typically StaticBody3D colgens that get
            // recreated later — but here we're running BEFORE collider gen,
            // so MIs are leaves. Still cheap to descend defensively.
            descend = true;
        }
    }
    if descend {
        for child in node.get_children().iter_shared() {
            collect_merge_candidates(child, root, out);
        }
    }
}

fn build_merged_mesh(parts: &[&MergeCandidate]) -> Option<(Gd<ArrayMesh>, Gd<Material>)> {
    if parts.is_empty() {
        return None;
    }

    let mut all_verts: Vec<Vector3> = Vec::new();
    let mut all_normals: Vec<Vector3> = Vec::new();
    let mut all_tangents: Vec<f32> = Vec::new();
    let mut all_uvs: Vec<Vector2> = Vec::new();
    let mut all_uv2s: Vec<Vector2> = Vec::new();
    let mut all_colors: Vec<Color> = Vec::new();
    let mut all_indices: Vec<i32> = Vec::new();

    // Vertex-attribute presence is decided by the FIRST surface; subsequent
    // surfaces missing the attribute disqualify it for the whole group
    // (partial channel = renderer-side mismatch).
    let mut first = true;
    let mut have_normals = false;
    let mut have_tangents = false;
    let mut have_uvs = false;
    let mut have_uv2s = false;
    let mut have_colors = false;
    let mut material: Option<Gd<Material>> = None;

    for part in parts {
        let Some(mesh) = part.mi.get_mesh() else {
            continue;
        };
        if mesh.get_surface_count() < 1 {
            continue;
        }
        if material.is_none() {
            material = part.mi.get_active_material(0);
        }

        let arrays = mesh.surface_get_arrays(0);
        let verts = match arrays
            .at(ArrayType::VERTEX.ord() as usize)
            .try_to::<PackedVector3Array>()
        {
            Ok(v) if !v.is_empty() => v,
            _ => continue,
        };

        let normals = arrays
            .at(ArrayType::NORMAL.ord() as usize)
            .try_to::<PackedVector3Array>()
            .ok()
            .filter(|a| a.len() == verts.len());
        let tangents = arrays
            .at(ArrayType::TANGENT.ord() as usize)
            .try_to::<PackedFloat32Array>()
            .ok()
            .filter(|a| a.len() == verts.len() * 4);
        let uvs = arrays
            .at(ArrayType::TEX_UV.ord() as usize)
            .try_to::<PackedVector2Array>()
            .ok()
            .filter(|a| a.len() == verts.len());
        let uv2s = arrays
            .at(ArrayType::TEX_UV2.ord() as usize)
            .try_to::<PackedVector2Array>()
            .ok()
            .filter(|a| a.len() == verts.len());
        let colors = arrays
            .at(ArrayType::COLOR.ord() as usize)
            .try_to::<PackedColorArray>()
            .ok()
            .filter(|a| a.len() == verts.len());

        if first {
            have_normals = normals.is_some();
            have_tangents = tangents.is_some();
            have_uvs = uvs.is_some();
            have_uv2s = uv2s.is_some();
            have_colors = colors.is_some();
            first = false;
        } else {
            have_normals = have_normals && normals.is_some();
            have_tangents = have_tangents && tangents.is_some();
            have_uvs = have_uvs && uvs.is_some();
            have_uv2s = have_uv2s && uv2s.is_some();
            have_colors = have_colors && colors.is_some();
        }

        let xform = part.world_xform;
        let basis_it = xform.basis.inverse().transposed();
        let base_vertex = all_verts.len() as i32;

        for i in 0..verts.len() {
            all_verts.push(xform * verts.get(i).unwrap_or(Vector3::ZERO));
        }
        if have_normals {
            if let Some(n) = normals {
                for i in 0..n.len() {
                    all_normals.push((basis_it * n.get(i).unwrap_or(Vector3::UP)).normalized());
                }
            }
        }
        if have_tangents {
            if let Some(t) = tangents {
                let count = t.len() / 4;
                for i in 0..count {
                    let tx = t.get(i * 4).unwrap_or(0.0);
                    let ty = t.get(i * 4 + 1).unwrap_or(0.0);
                    let tz = t.get(i * 4 + 2).unwrap_or(0.0);
                    let tw = t.get(i * 4 + 3).unwrap_or(1.0);
                    let tv = (xform.basis * Vector3::new(tx, ty, tz)).normalized();
                    all_tangents.push(tv.x);
                    all_tangents.push(tv.y);
                    all_tangents.push(tv.z);
                    all_tangents.push(tw);
                }
            }
        }
        if have_uvs {
            if let Some(u) = uvs {
                for i in 0..u.len() {
                    all_uvs.push(u.get(i).unwrap_or(Vector2::ZERO));
                }
            }
        }
        if have_uv2s {
            if let Some(u) = uv2s {
                for i in 0..u.len() {
                    all_uv2s.push(u.get(i).unwrap_or(Vector2::ZERO));
                }
            }
        }
        if have_colors {
            if let Some(c) = colors {
                for i in 0..c.len() {
                    all_colors.push(c.get(i).unwrap_or(Color::from_rgba(1.0, 1.0, 1.0, 1.0)));
                }
            }
        }

        let idx = arrays
            .at(ArrayType::INDEX.ord() as usize)
            .try_to::<PackedInt32Array>()
            .unwrap_or_else(|_| PackedInt32Array::new());
        if idx.is_empty() {
            for i in 0..(verts.len() as i32) {
                all_indices.push(base_vertex + i);
            }
        } else {
            for i in 0..idx.len() {
                all_indices.push(base_vertex + idx.get(i).unwrap_or(0));
            }
        }
    }

    if all_verts.is_empty() {
        return None;
    }
    let material = material?;

    let mut arrays = VarArray::new();
    arrays.resize(ArrayType::MAX.ord() as usize, &Variant::nil());
    arrays.set(
        ArrayType::VERTEX.ord() as usize,
        &packed_vector3(&all_verts).to_variant(),
    );
    if have_normals && all_normals.len() == all_verts.len() {
        arrays.set(
            ArrayType::NORMAL.ord() as usize,
            &packed_vector3(&all_normals).to_variant(),
        );
    }
    if have_tangents && all_tangents.len() == all_verts.len() * 4 {
        arrays.set(
            ArrayType::TANGENT.ord() as usize,
            &packed_float32(&all_tangents).to_variant(),
        );
    }
    if have_uvs && all_uvs.len() == all_verts.len() {
        arrays.set(
            ArrayType::TEX_UV.ord() as usize,
            &packed_vector2(&all_uvs).to_variant(),
        );
    }
    if have_uv2s && all_uv2s.len() == all_verts.len() {
        arrays.set(
            ArrayType::TEX_UV2.ord() as usize,
            &packed_vector2(&all_uv2s).to_variant(),
        );
    }
    if have_colors && all_colors.len() == all_verts.len() {
        arrays.set(
            ArrayType::COLOR.ord() as usize,
            &packed_color(&all_colors).to_variant(),
        );
    }
    arrays.set(
        ArrayType::INDEX.ord() as usize,
        &packed_int32(&all_indices).to_variant(),
    );

    let mut mesh = ArrayMesh::new_gd();
    mesh.add_surface_from_arrays(PrimitiveType::TRIANGLES, &arrays);
    Some((mesh, material))
}

/// Merge static MIs sharing material identity into one MI per material.
/// Returns counts for logging; no metrics infrastructure needed.
pub fn merge_static_meshes(root_node: &Gd<Node3D>) -> MergeStats {
    let mut stats = MergeStats::default();
    if tree_has_skeleton_or_animation(&root_node.clone().upcast()) {
        stats.skipped_tree_animated = 1;
        return stats;
    }

    let mut candidates: Vec<MergeCandidate> = Vec::new();
    collect_merge_candidates(root_node.clone().upcast(), root_node, &mut candidates);
    stats.eligible_mis = candidates.len() as u32;

    let mut groups: HashMap<i64, Vec<MergeCandidate>> = HashMap::new();
    for c in candidates {
        groups.entry(c.material_id).or_default().push(c);
    }

    let mut root_as_node: Gd<Node> = root_node.clone().upcast();

    for (key, group) in groups {
        if group.len() < 2 {
            continue;
        }
        let refs: Vec<&MergeCandidate> = group.iter().collect();
        let Some((merged_mesh, material)) = build_merged_mesh(&refs) else {
            continue;
        };

        let mut new_mi = MeshInstance3D::new_alloc();
        let name = format!("MergedMesh_{}_x{}", key, group.len());
        new_mi.set_name(&name);
        new_mi.set_mesh(&merged_mesh.upcast::<godot::classes::Mesh>());
        new_mi.set_surface_override_material(0, &material);
        root_as_node.add_child(&new_mi.upcast::<Node>());

        for c in group {
            // Orphan tree → free() not queue_free() (per the comment in
            // load_and_save_scene_gltf about background-thread node handling).
            c.mi.clone().free();
            stats.merged_sources += 1;
        }
        stats.merged_groups += 1;
    }
    stats
}

fn packed_vector3(src: &[Vector3]) -> PackedVector3Array {
    let mut a = PackedVector3Array::new();
    a.resize(src.len());
    a.as_mut_slice().copy_from_slice(src);
    a
}

fn packed_vector2(src: &[Vector2]) -> PackedVector2Array {
    let mut a = PackedVector2Array::new();
    a.resize(src.len());
    a.as_mut_slice().copy_from_slice(src);
    a
}

fn packed_int32(src: &[i32]) -> PackedInt32Array {
    let mut a = PackedInt32Array::new();
    a.resize(src.len());
    a.as_mut_slice().copy_from_slice(src);
    a
}

fn packed_color(src: &[Color]) -> PackedColorArray {
    let mut a = PackedColorArray::new();
    a.resize(src.len());
    a.as_mut_slice().copy_from_slice(src);
    a
}

fn packed_float32(src: &[f32]) -> PackedFloat32Array {
    let mut a = PackedFloat32Array::new();
    a.resize(src.len());
    a.as_mut_slice().copy_from_slice(src);
    a
}

// ============================================================================
// Import-time MultiMesh batching
// ============================================================================
//
// Group MIs that share BOTH `Mesh` resource identity AND `Material`
// instance identity into one `MultiMeshInstance3D`. Each instance keeps
// its own AABB (derived from base mesh AABB × instance transform), so
// per-instance frustum culling and per-instance LOD selection both still
// work — unlike geometry merging, which collapses everything into one
// giant AABB.
//
// This is the right shape for DCL Genesis Plaza-style content: many
// repeated props (lampposts, benches, trees) referencing the same mesh
// + material end up as one draw call, while still respecting our LOD
// chain and visibility grid.
//
// Eligibility mirrors `merge_static_meshes` (single-surface, no blend
// shapes, no skinning, not a named collider) plus the additional
// requirement of mesh-resource sharing. Whole-tree skip on
// `Skeleton3D` / `AnimationPlayer` ancestors is the same conservative
// guard.

#[derive(Default, Debug)]
pub struct MultiMeshStats {
    pub eligible_mis: u32,
    pub batches_created: u32,
    pub instances_batched: u32,
    pub skipped_tree_animated: u32,
}

/// Minimum group size to convert to MultiMesh. Below this the per-draw
/// savings don't pay for the MultiMeshInstance3D overhead.
const MULTIMESH_MIN_INSTANCES: usize = 3;

struct MultiMeshCandidate {
    mi: Gd<MeshInstance3D>,
    mesh: Gd<Mesh>,
    material: Gd<Material>,
    mesh_id: i64,
    material_id: i64,
    world_xform: Transform3D,
}

fn mi_eligible_for_multimesh(
    mi: &Gd<MeshInstance3D>,
) -> Option<(Gd<Mesh>, Gd<Material>, i64, i64)> {
    if mi
        .get_name()
        .to_string()
        .to_lowercase()
        .contains("collider")
    {
        return None;
    }
    let mesh = mi.get_mesh()?;
    if mesh.get_surface_count() != 1 {
        return None;
    }
    let array_mesh = mesh.clone().try_cast::<ArrayMesh>().ok()?;
    if array_mesh.get_blend_shape_count() > 0 {
        return None;
    }
    if array_mesh.surface_get_primitive_type(0) != PrimitiveType::TRIANGLES {
        return None;
    }
    let arrays = array_mesh.surface_get_arrays(0);
    let bones_v = arrays.at(ArrayType::BONES.ord() as usize);
    let has_int_bones = bones_v
        .try_to::<PackedInt32Array>()
        .map(|a| !a.is_empty())
        .unwrap_or(false);
    let has_float_bones = bones_v
        .try_to::<PackedFloat32Array>()
        .map(|a| !a.is_empty())
        .unwrap_or(false);
    if has_int_bones || has_float_bones {
        return None;
    }
    let material: Gd<Material> = mi.get_active_material(0)?;
    let mesh_id = mesh.instance_id().to_i64();
    let material_id = material.instance_id().to_i64();
    Some((mesh, material, mesh_id, material_id))
}

fn collect_multimesh_candidates(
    node: Gd<Node>,
    root: &Gd<Node3D>,
    out: &mut Vec<MultiMeshCandidate>,
) {
    if let Ok(mi) = node.clone().try_cast::<MeshInstance3D>() {
        if let Some((mesh, material, mesh_id, material_id)) = mi_eligible_for_multimesh(&mi) {
            let world_xform = compute_relative_transform(&mi, root);
            out.push(MultiMeshCandidate {
                mi: mi.clone(),
                mesh,
                material,
                mesh_id,
                material_id,
                world_xform,
            });
        }
    }
    for child in node.get_children().iter_shared() {
        collect_multimesh_candidates(child, root, out);
    }
}

/// Convert groups of identical-mesh+material MIs into `MultiMeshInstance3D`
/// nodes parented at the scene root. Returns counts for logging.
pub fn multimesh_batch_static(root_node: &Gd<Node3D>) -> MultiMeshStats {
    let mut stats = MultiMeshStats::default();
    if tree_has_skeleton_or_animation(&root_node.clone().upcast()) {
        stats.skipped_tree_animated = 1;
        return stats;
    }

    let mut candidates: Vec<MultiMeshCandidate> = Vec::new();
    collect_multimesh_candidates(root_node.clone().upcast(), root_node, &mut candidates);
    stats.eligible_mis = candidates.len() as u32;

    // Key by both mesh and material identity. Different materials means
    // different shader bindings → can't share a MultiMesh draw.
    let mut groups: HashMap<(i64, i64), Vec<MultiMeshCandidate>> = HashMap::new();
    for c in candidates {
        groups
            .entry((c.mesh_id, c.material_id))
            .or_default()
            .push(c);
    }

    let mut root_as_node: Gd<Node> = root_node.clone().upcast();

    for ((mesh_id, _mat_id), group) in groups {
        if group.len() < MULTIMESH_MIN_INSTANCES {
            continue;
        }

        // Pick representative mesh/material from the first member (all
        // identical by construction).
        let mesh = group[0].mesh.clone();
        let material = group[0].material.clone();

        let mut mm = MultiMesh::new_gd();
        mm.set_transform_format(TransformFormat::TRANSFORM_3D);
        mm.set_use_colors(false);
        mm.set_use_custom_data(false);
        mm.set_mesh(&mesh);
        mm.set_instance_count(group.len() as i32);
        for (i, c) in group.iter().enumerate() {
            mm.set_instance_transform(i as i32, c.world_xform);
        }

        let mut mmi = MultiMeshInstance3D::new_alloc();
        let name = format!("BatchedMM_{}_x{}", mesh_id, group.len());
        mmi.set_name(&name);
        mmi.set_multimesh(&mm);
        // Apply material as override so per-instance LOD selection still
        // uses the source mesh's LOD chain (set on the Mesh, not the
        // material).
        mmi.set_material_override(&material);
        root_as_node.add_child(&mmi.upcast::<Node>());

        for c in group {
            c.mi.clone().free();
            stats.instances_batched += 1;
        }
        stats.batches_created += 1;
    }
    stats
}
