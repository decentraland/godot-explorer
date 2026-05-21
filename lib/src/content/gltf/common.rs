//! Common utilities and pipeline for GLTF loading.

use std::sync::Arc;

use godot::{
    builtin::GString,
    classes::{
        base_material_3d::{ShadingMode, TextureParam},
        mesh::{ArrayType, PrimitiveType},
        BaseMaterial3D, GltfDocument, GltfState, ImageTexture, MeshInstance3D, Node, Node3D,
    },
    global::Error,
    meta::ToGodot,
    obj::Gd,
    prelude::*,
};
use meshopt::{simplify, SimplifyOptions, VertexDataAdapter};
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio::sync::Semaphore;

use crate::content::texture::resize_image;

use super::super::{
    content_mapping::ContentMappingAndUrlRef, content_provider::SceneGltfContext,
    file_string::get_base_dir, texture::create_compressed_texture,
};

#[cfg(feature = "use_resource_tracking")]
use crate::godot_classes::dcl_resource_tracker::{
    report_resource_error, report_resource_loaded, report_resource_start,
};

/// Post-import texture processing for all GLTF types.
/// Resizes and optionally compresses images according to max_size limits.
///
/// # Arguments
/// * `node_to_inspect` - The root node to process
/// * `max_size` - Maximum texture dimension
/// * `force_compress` - If true, always compress with ETC2 (for asset server)
pub fn post_import_process(node_to_inspect: Gd<Node>, max_size: i32, force_compress: bool) {
    let should_compress =
        force_compress || std::env::consts::OS == "ios" || std::env::consts::OS == "android";

    for child in node_to_inspect.get_children().iter_shared() {
        if let Ok(mesh_instance_3d) = child.clone().try_cast::<MeshInstance3D>() {
            if let Some(mesh) = mesh_instance_3d.get_mesh() {
                for surface_index in 0..mesh.get_surface_count() {
                    if let Some(material) = mesh.surface_get_material(surface_index) {
                        if let Ok(mut base_material) = material.try_cast::<BaseMaterial3D>() {
                            // Resize/compress images
                            for ord in 0..TextureParam::MAX.ord() {
                                let texture_param = TextureParam::from_ord(ord);
                                if let Some(texture) = base_material.get_texture(texture_param) {
                                    if let Ok(mut texture_image) =
                                        texture.try_cast::<ImageTexture>()
                                    {
                                        if let Some(mut image) = texture_image.get_image() {
                                            if should_compress {
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

        post_import_process(child, max_size, force_compress);
    }
}

/// Walk the post-generate scene tree and bake LODs into every MeshInstance3D's
/// ArrayMesh via Godot's native `ImporterMesh.generate_lods` (roundtrip
/// ArrayMesh → ImporterMesh → generate_lods → ArrayMesh). Runs AFTER the
/// splitter so chunks also get LODs.
///
/// Hand-rolled meshopt::simplify output crashed Godot's renderer with SIGSEGV
/// when LODs engaged. The native generate_lods produces a LOD chain in the
/// exact format the renderer expects.
#[allow(dead_code)]
fn apply_post_generate_godot_lods(root: Gd<Node>) {
    use godot::classes::{ArrayMesh, ImporterMesh, MeshInstance3D};
    // Surfaces with fewer indices than this are too small for a useful LOD
    // chain — meshopt/generate_lods on tiny surfaces produces degenerate LOD
    // levels that have triggered renderer SIGSEGVs in the past. Lowered from
    // 96 (32 tris) to 24 (8 tris) so chunks of split meshes also get LODs.
    const MIN_INDICES_FOR_LOD: i32 = 24;
    let mut stack: Vec<Gd<Node>> = vec![root];
    let mut meshes_with_lods = 0u32;
    let mut meshes_skipped = 0u32;
    let mut chunks_baked = 0u32;
    let mut chunks_skipped_small = 0u32;
    while let Some(n) = stack.pop() {
        let kids = n.get_children();
        for i in 0..kids.len() {
            stack.push(kids.at(i));
        }
        let Ok(mut mi) = n.try_cast::<MeshInstance3D>() else {
            continue;
        };
        let is_chunk = mi
            .get_parent()
            .map(|p| p.get_name().to_string() == "_splitted")
            .unwrap_or(false);
        let Some(mesh) = mi.get_mesh() else { continue };
        let Ok(am) = mesh.try_cast::<ArrayMesh>() else {
            continue;
        };
        if am.get_blend_shape_count() > 0 {
            meshes_skipped += 1;
            continue;
        }
        let surface_count = am.get_surface_count();
        if surface_count == 0 {
            meshes_skipped += 1;
            continue;
        }
        // Skip skinned surfaces (bone weights would mismatch after simplify).
        let mut any_skinned = false;
        for s in 0..surface_count {
            let arrays = am.surface_get_arrays(s);
            let has_bones = arrays
                .at(ArrayType::BONES.ord() as usize)
                .try_to::<PackedInt32Array>()
                .map(|a| !a.is_empty())
                .unwrap_or(false)
                || arrays
                    .at(ArrayType::BONES.ord() as usize)
                    .try_to::<PackedFloat32Array>()
                    .map(|a| !a.is_empty())
                    .unwrap_or(false);
            if has_bones {
                any_skinned = true;
                break;
            }
        }
        if any_skinned {
            meshes_skipped += 1;
            continue;
        }

        // Index-count guard: any surface below the threshold and we skip this
        // MI. Tiny surfaces produce degenerate LODs that have SIGSEGV'd the
        // renderer. We require ALL surfaces to clear the bar — partial bakes
        // are not worth the complexity.
        let mut any_tiny = false;
        for s in 0..surface_count {
            let arrays = am.surface_get_arrays(s);
            let n_indices = arrays
                .at(ArrayType::INDEX.ord() as usize)
                .try_to::<PackedInt32Array>()
                .map(|a| a.len() as i32)
                .unwrap_or(0);
            if n_indices < MIN_INDICES_FOR_LOD {
                any_tiny = true;
                break;
            }
        }
        if any_tiny {
            if is_chunk {
                chunks_skipped_small += 1;
            } else {
                meshes_skipped += 1;
            }
            continue;
        }

        // ArrayMesh → ImporterMesh (preserve materials + primitive type).
        let mut importer = ImporterMesh::new_gd();
        for s in 0..surface_count {
            let arrays = am.surface_get_arrays(s);
            let primitive = am.surface_get_primitive_type(s);
            let material = am.surface_get_material(s);
            let surface_name = am.surface_get_name(s);
            let mut call = importer.add_surface_ex(primitive, &arrays);
            if let Some(m) = material.as_ref() {
                call = call.material(m);
            }
            call.name(&surface_name).done();
        }
        // Native LOD bake — output is a LOD chain Godot's renderer accepts.
        importer.generate_lods(60.0, 25.0, &VarArray::new());
        let Some(baked) = importer.get_mesh() else {
            if is_chunk {
                chunks_skipped_small += 1;
            } else {
                meshes_skipped += 1;
            }
            continue;
        };
        mi.set_mesh(&baked);
        if is_chunk {
            chunks_baked += 1;
        } else {
            meshes_with_lods += 1;
        }
    }
    godot::global::godot_print!(
        "[godot-lods] non-chunks: baked={} skipped={}  chunks: baked={} skipped_small={}",
        meshes_with_lods,
        meshes_skipped,
        chunks_baked,
        chunks_skipped_small
    );
}

/// Flip every `BaseMaterial3D` to `SHADING_MODE_PER_VERTEX`. Runs between
/// `append_from_file_ex` and `generate_scene`, before the materials are
/// bound to mesh instances or registered with the renderer's shader_map,
/// so the first shader variant compiled is the vertex-lighting one —
/// no recompile, no batching invalidation, single MaterialKey for the
/// batch.
#[allow(dead_code)]
pub(super) fn apply_pre_generate_material_overrides(state: &mut Gd<GltfState>) {
    let materials = state.get_materials();
    for i in 0..materials.len() {
        let material = materials.at(i);
        let Ok(mut base) = material.try_cast::<BaseMaterial3D>() else {
            continue;
        };
        base.set_shading_mode(ShadingMode::PER_VERTEX);
    }
}

/// Post-everything material pass. Runs AFTER split + LODs + shadow so it
/// catches any material that the splitter's new ArrayMesh-per-chunk path
/// or the ImporterMesh.generate_lods roundtrip might have left in a
/// non-PER_VERTEX shading mode. Also audits: returns the number of
/// surfaces where neither the mesh-level material nor the MI override
/// resolves to a material — those would render untextured at runtime.
///
/// Returns (flipped_to_per_vertex, surfaces_without_any_material).
fn apply_post_material_overrides(root: &Gd<Node>) -> (u32, u32) {
    use godot::classes::Material;
    let mut flipped = 0u32;
    let mut without = 0u32;
    let mut stack: Vec<Gd<Node>> = vec![root.clone()];
    while let Some(n) = stack.pop() {
        let kids = n.get_children();
        for i in 0..kids.len() {
            stack.push(kids.at(i));
        }
        let Ok(mi) = n.try_cast::<MeshInstance3D>() else {
            continue;
        };
        // Walk material_override
        if let Some(m) = mi.get_material_override() {
            if let Ok(mut base) = m.try_cast::<BaseMaterial3D>() {
                if base.get_shading_mode() != ShadingMode::PER_VERTEX {
                    base.set_shading_mode(ShadingMode::PER_VERTEX);
                    flipped += 1;
                }
            }
        }
        let Some(mesh) = mi.get_mesh() else { continue };
        let surface_count = mesh.get_surface_count();
        for s in 0..surface_count {
            let override_mat: Option<Gd<Material>> = mi.get_surface_override_material(s);
            let mesh_mat: Option<Gd<Material>> = mesh.surface_get_material(s);
            // Flip both layers if they're BaseMaterial3D
            for slot in [override_mat.clone(), mesh_mat.clone()]
                .into_iter()
                .flatten()
            {
                if let Ok(mut base) = slot.try_cast::<BaseMaterial3D>() {
                    if base.get_shading_mode() != ShadingMode::PER_VERTEX {
                        base.set_shading_mode(ShadingMode::PER_VERTEX);
                        flipped += 1;
                    }
                }
            }
            if override_mat.is_none() && mesh_mat.is_none() {
                without += 1;
            }
        }
    }
    (flipped, without)
}

/// Per-surface LOD chain generation on the GltfState's `ImporterMesh`
/// array, run between `append_from_file_ex` and `generate_scene`. LOD0
/// (full quality) is preserved — every additional level is added via the
/// `lods` Dictionary slot on `add_surface`, keyed by screen-space-error
/// threshold. Godot's renderer swaps to a lower LOD when an instance's
/// projected size makes its screen-space error exceed the viewport's
/// `mesh_lod_threshold` (in pixels).
///
/// Vanilla `meshopt::simplify` is topology-preserving; DCL user-authored
/// GLBs have UV-seam topology discontinuities at every material boundary
/// so vanilla returns ~98.5% of source indices. `Permissive` lifts that
/// constraint; `Sparse` skips the topology rebuild we don't need.
///
/// Skip rules:
/// * meshes with blend shapes — `add_surface` doesn't roundtrip the
///   per-shape vertex stream array, so morph-driven animation would lose
///   its target data.
/// * skinned surfaces (`ARRAY_BONES` populated) — decimated indices
///   reference different source verts; bone-weighted transforms stretch
///   the simplified geometry visibly during animation.
/// * surfaces with < `MIN_INDICES_FOR_LOD` indices — meshopt's quadric
///   error metric is noisy on tiny meshes and the per-LOD bookkeeping
///   overhead exceeds the savings.
/// * surfaces where the decimator kept ≥ 90% of source indices — common
///   on terrain/fences; not worth a draw-state switch.
#[allow(dead_code)]
pub(super) fn apply_pre_generate_mesh_simplification(
    state: &mut Gd<GltfState>,
    _target_ratio: f32,
) {
    const LOD_LEVELS: &[(f32, f32)] = &[
        (0.5, 0.1),  // LOD1: ~50% indices, kicks in at d > 0.1 unit
        (0.25, 0.5), // LOD2: ~25%, d > 0.5
        (0.1, 1.5),  // LOD3: ~10%, d > 1.5
    ];
    const MIN_INDICES_FOR_LOD: usize = 30;

    let meshes = state.get_meshes();
    let mesh_count = meshes.len();
    let mut surfaces_with_lods = 0u32;
    let mut surfaces_no_lods = 0u32;
    let mut src_idx_total: u64 = 0;
    let mut lod_idx_total: u64 = 0;

    for mi in 0..mesh_count {
        let mut gltf_mesh = meshes.at(mi);
        let Some(mut importer) = gltf_mesh.get_mesh() else {
            continue;
        };
        let surface_count = importer.get_surface_count();
        if surface_count == 0 {
            continue;
        }
        if importer.get_blend_shape_count() > 0 {
            continue;
        }

        struct Snapshot {
            primitive: PrimitiveType,
            arrays: VarArray,
            material: Option<Gd<godot::classes::Material>>,
            name: String,
            flags: u64,
            lods: VarDictionary,
        }

        let mut snapshots: Vec<Snapshot> = Vec::with_capacity(surface_count as usize);
        let mut mesh_has_any_skinned_surface = false;
        for s in 0..surface_count {
            let arrays = importer.get_surface_arrays(s);
            // Pre-flight: if any surface in this mesh references bones, skip
            // the whole mesh. ImporterMesh.add_surface doesn't roundtrip the
            // bone-weighted vertex stream cleanly through `clear()` + re-add,
            // so re-adding a skinned surface lands its vertices at the
            // origin / wrong transform — visible as floating ghost meshes
            // at random positions.
            let has_bones = arrays
                .at(ArrayType::BONES.ord() as usize)
                .try_to::<PackedInt32Array>()
                .map(|a| !a.is_empty())
                .unwrap_or(false)
                || arrays
                    .at(ArrayType::BONES.ord() as usize)
                    .try_to::<PackedFloat32Array>()
                    .map(|a| !a.is_empty())
                    .unwrap_or(false);
            if has_bones {
                mesh_has_any_skinned_surface = true;
            }
            snapshots.push(Snapshot {
                primitive: importer.get_surface_primitive_type(s),
                arrays,
                material: importer.get_surface_material(s),
                name: importer.get_surface_name(s).to_string(),
                flags: importer.get_surface_format(s),
                lods: VarDictionary::new(),
            });
        }
        if mesh_has_any_skinned_surface {
            continue;
        }

        let mut any_lod_built = false;
        for snap in snapshots.iter_mut() {
            if snap.primitive != PrimitiveType::TRIANGLES {
                surfaces_no_lods += 1;
                continue;
            }
            let Ok(idx) = snap
                .arrays
                .at(ArrayType::INDEX.ord() as usize)
                .try_to::<PackedInt32Array>()
            else {
                surfaces_no_lods += 1;
                continue;
            };
            if idx.len() < MIN_INDICES_FOR_LOD {
                surfaces_no_lods += 1;
                continue;
            }
            let Ok(verts) = snap
                .arrays
                .at(ArrayType::VERTEX.ord() as usize)
                .try_to::<PackedVector3Array>()
            else {
                surfaces_no_lods += 1;
                continue;
            };
            if verts.is_empty() {
                surfaces_no_lods += 1;
                continue;
            }
            let has_bones = snap
                .arrays
                .at(ArrayType::BONES.ord() as usize)
                .try_to::<PackedInt32Array>()
                .map(|a| !a.is_empty())
                .unwrap_or(false)
                || snap
                    .arrays
                    .at(ArrayType::BONES.ord() as usize)
                    .try_to::<PackedFloat32Array>()
                    .map(|a| !a.is_empty())
                    .unwrap_or(false);
            if has_bones {
                surfaces_no_lods += 1;
                continue;
            }

            let indices_u32: Vec<u32> = idx.as_slice().iter().map(|&i| i as u32).collect();
            let mut vbytes: Vec<u8> = Vec::with_capacity(verts.len() * 12);
            for v in verts.as_slice() {
                vbytes.extend_from_slice(&v.x.to_le_bytes());
                vbytes.extend_from_slice(&v.y.to_le_bytes());
                vbytes.extend_from_slice(&v.z.to_le_bytes());
            }
            let Ok(adapter) = VertexDataAdapter::new(&vbytes, 12, 0) else {
                surfaces_no_lods += 1;
                continue;
            };

            src_idx_total = src_idx_total.saturating_add(idx.len() as u64);
            let mut surface_had_lod = false;
            for &(ratio, sse_key) in LOD_LEVELS {
                let target_count = ((idx.len() as f32) * ratio).round() as usize;
                let target_count = target_count - (target_count % 3);
                if target_count < 3 || target_count >= idx.len() {
                    continue;
                }
                let lod_indices = simplify(
                    &indices_u32,
                    &adapter,
                    target_count,
                    0.02,
                    SimplifyOptions::Sparse,
                    None,
                );
                if lod_indices.is_empty()
                    || lod_indices.len() as f32 / idx.len() as f32 > 0.9
                    || !lod_indices.len().is_multiple_of(3)
                {
                    continue;
                }
                let mut packed = PackedInt32Array::new();
                packed.resize(lod_indices.len());
                let slc = packed.as_mut_slice();
                for (k, &i) in lod_indices.iter().enumerate() {
                    slc[k] = i as i32;
                }
                let _ = snap.lods.insert(sse_key.to_variant(), packed.to_variant());
                lod_idx_total = lod_idx_total.saturating_add(lod_indices.len() as u64);
                surface_had_lod = true;
            }
            if surface_had_lod {
                surfaces_with_lods += 1;
                any_lod_built = true;
            } else {
                surfaces_no_lods += 1;
            }
        }

        if !any_lod_built {
            continue;
        }
        importer.clear();
        for snap in snapshots {
            let name_gs = GString::from(snap.name.as_str());
            importer
                .add_surface_ex(snap.primitive, &snap.arrays)
                .name(&name_gs)
                .material(snap.material.as_ref())
                .lods(&snap.lods)
                .flags(snap.flags)
                .done();
        }
    }

    if src_idx_total > 0 {
        godot::global::godot_print!(
            "[mesh-lod-chain] surfaces with_lods={} no_lods={} src_idx={} lod_idx={}",
            surfaces_with_lods,
            surfaces_no_lods,
            src_idx_total,
            lod_idx_total,
        );
    }
}

/// Post-generate LOD chain. Walks the scene tree and rebuilds each
/// MeshInstance3D's ArrayMesh with the same surfaces plus a `lods` Dictionary
/// on each surface (kept by `add_surface_from_arrays`).
/// Kept (#[allow(dead_code)]) for follow-up when the splitter is revisited.
#[allow(dead_code)]
fn apply_post_generate_lod_chain(root: Gd<Node>) {
    use godot::classes::{ArrayMesh, MeshInstance3D};

    // (ratio_indices_kept, lod_threshold) — small thresholds. Bigger thresholds
    // (5/20/80m) caused engaged LODs to render with corrupted geometry that
    // SIGSEGV'd Godot's renderer; reverted until the simplify-output → Godot
    // ArrayMesh interaction is debugged separately.
    const LOD_LEVELS: &[(f32, f32)] = &[(0.5, 0.1), (0.25, 0.5), (0.1, 1.5)];
    const MIN_INDICES_FOR_LOD: usize = 30;

    let mut stack: Vec<Gd<Node>> = vec![root];
    let mut surfaces_with_lods = 0u32;
    let mut surfaces_no_lods = 0u32;
    let mut chunk_surfaces_with_lods = 0u32;
    let mut chunk_surfaces_no_lods = 0u32;
    let mut chunks_seen = 0u32;
    let mut src_idx_total: u64 = 0;
    let mut lod_idx_total: u64 = 0;

    while let Some(n) = stack.pop() {
        let kids = n.get_children();
        for i in 0..kids.len() {
            stack.push(kids.at(i));
        }
        let Ok(mut mi) = n.try_cast::<MeshInstance3D>() else {
            continue;
        };
        let is_chunk = mi
            .get_parent()
            .map(|p| p.get_name().to_string() == "_splitted")
            .unwrap_or(false);
        if is_chunk {
            chunks_seen += 1;
        }
        let Some(mesh) = mi.get_mesh() else { continue };
        let Ok(am) = mesh.try_cast::<ArrayMesh>() else {
            continue;
        };
        if am.get_blend_shape_count() > 0 {
            continue;
        }

        let surface_count = am.get_surface_count();
        if surface_count == 0 {
            continue;
        }

        let mut new_am = ArrayMesh::new_gd();
        let mut any_lod_built = false;

        for s in 0..surface_count {
            let arrays = am.surface_get_arrays(s);
            let material = am.surface_get_material(s);
            let primitive = am.surface_get_primitive_type(s);

            let bones_present = arrays
                .at(ArrayType::BONES.ord() as usize)
                .try_to::<PackedInt32Array>()
                .map(|a| !a.is_empty())
                .unwrap_or(false)
                || arrays
                    .at(ArrayType::BONES.ord() as usize)
                    .try_to::<PackedFloat32Array>()
                    .map(|a| !a.is_empty())
                    .unwrap_or(false);

            let mut lods = VarDictionary::new();
            let lods_built = if !bones_present && primitive == PrimitiveType::TRIANGLES {
                build_lods_for_surface(&arrays, LOD_LEVELS, MIN_INDICES_FOR_LOD, &mut lods)
            } else {
                None
            };

            if let Some((src_n, lod_n)) = lods_built {
                src_idx_total = src_idx_total.saturating_add(src_n as u64);
                lod_idx_total = lod_idx_total.saturating_add(lod_n as u64);
                surfaces_with_lods += 1;
                if is_chunk {
                    chunk_surfaces_with_lods += 1;
                }
                any_lod_built = true;
            } else {
                surfaces_no_lods += 1;
                if is_chunk {
                    chunk_surfaces_no_lods += 1;
                }
            }

            let surf_before = new_am.get_surface_count();
            new_am
                .add_surface_from_arrays_ex(primitive, &arrays)
                .lods(&lods)
                .done();
            if let Some(mat) = material {
                new_am.surface_set_material(surf_before, &mat);
            }
        }

        if any_lod_built {
            mi.set_mesh(&new_am);
        }
    }

    if src_idx_total > 0 || chunks_seen > 0 {
        godot::global::godot_print!(
            "[mesh-lod-chain] surfaces with_lods={} no_lods={} src_idx={} lod_idx={} chunks_seen={} chunks_with_lods={} chunks_no_lods={}",
            surfaces_with_lods,
            surfaces_no_lods,
            src_idx_total,
            lod_idx_total,
            chunks_seen,
            chunk_surfaces_with_lods,
            chunk_surfaces_no_lods,
        );
    }
}

/// Run meshopt::simplify for each LOD level and insert the resulting indices
/// into `lods` keyed by screen-space-error. Returns (src_indices, total_lod_indices)
/// if at least one LOD was added, None otherwise.
#[allow(dead_code)]
fn build_lods_for_surface(
    arrays: &VarArray,
    levels: &[(f32, f32)],
    min_indices_for_lod: usize,
    lods: &mut VarDictionary,
) -> Option<(usize, usize)> {
    let idx = arrays
        .at(ArrayType::INDEX.ord() as usize)
        .try_to::<PackedInt32Array>()
        .ok()?;
    if idx.len() < min_indices_for_lod {
        return None;
    }
    let verts = arrays
        .at(ArrayType::VERTEX.ord() as usize)
        .try_to::<PackedVector3Array>()
        .ok()?;
    if verts.is_empty() {
        return None;
    }

    let indices_u32: Vec<u32> = idx.as_slice().iter().map(|&i| i as u32).collect();
    let mut vbytes: Vec<u8> = Vec::with_capacity(verts.len() * 12);
    for v in verts.as_slice() {
        vbytes.extend_from_slice(&v.x.to_le_bytes());
        vbytes.extend_from_slice(&v.y.to_le_bytes());
        vbytes.extend_from_slice(&v.z.to_le_bytes());
    }
    let adapter = VertexDataAdapter::new(&vbytes, 12, 0).ok()?;

    let mut total_lod = 0usize;
    let mut any = false;
    for &(ratio, sse_key) in levels {
        let target = ((idx.len() as f32) * ratio).round() as usize;
        let target = target - (target % 3);
        if target < 3 || target >= idx.len() {
            continue;
        }
        let lod_indices = simplify(
            &indices_u32,
            &adapter,
            target,
            0.02,
            SimplifyOptions::Sparse,
            None,
        );
        if lod_indices.is_empty()
            || lod_indices.len() as f32 / idx.len() as f32 > 0.9
            || !lod_indices.len().is_multiple_of(3)
        {
            continue;
        }
        // Defensive: every LOD index must be within the surface's vertex
        // range. Out-of-bounds indices crash Godot's renderer with SIGSEGV
        // (observed when chunks have remapped vertices and meshopt::simplify
        // edge cases produced indices > verts.len()).
        let max_vert = verts.len() as u32;
        let bounds_ok = lod_indices.iter().all(|&i| i < max_vert);
        if !bounds_ok {
            godot::global::godot_print!(
                "[lod-chain] WARN: skip LOD with out-of-bounds index (verts={}, max_idx={})",
                max_vert,
                lod_indices.iter().max().copied().unwrap_or(0)
            );
            continue;
        }
        let mut packed = PackedInt32Array::new();
        packed.resize(lod_indices.len());
        let slc = packed.as_mut_slice();
        for (k, &i) in lod_indices.iter().enumerate() {
            slc[k] = i as i32;
        }
        let _ = lods.insert(sse_key.to_variant(), packed.to_variant());
        total_lod += lod_indices.len();
        any = true;
    }
    if any {
        Some((idx.len(), total_lod))
    } else {
        None
    }
}

/// Walk the generated scene tree and report how many MeshInstance3D surfaces
/// have LOD chains attached. Verifies that the post-split + post-LOD bake
/// survived the conversion to the MeshInstance3D + ArrayMesh that ends up
/// in the saved .scn.
#[allow(dead_code)]
fn verify_lods_in_generated_scene(root: Gd<Node>) {
    let mut stack: Vec<Gd<Node>> = vec![root];
    let mut total = 0u32;
    let mut with_lods = 0u32;
    let mut without_lods = 0u32;
    while let Some(n) = stack.pop() {
        let kids = n.get_children();
        for i in 0..kids.len() {
            stack.push(kids.at(i));
        }
        let Ok(mi) = n.clone().try_cast::<godot::classes::MeshInstance3D>() else {
            continue;
        };
        let Some(mesh) = mi.get_mesh() else {
            continue;
        };
        let Ok(am) = mesh.try_cast::<godot::classes::ArrayMesh>() else {
            continue;
        };
        let mesh_rid = am.get_rid();
        let surface_count = am.get_surface_count();
        for s in 0..surface_count {
            total += 1;
            let surf = godot::classes::RenderingServer::singleton().mesh_get_surface(mesh_rid, s);
            let lods_array = surf
                .get("lods")
                .and_then(|v| v.try_to::<VarArray>().ok())
                .map(|a| a.len())
                .unwrap_or(0);
            if lods_array > 0 {
                with_lods += 1;
            } else {
                without_lods += 1;
            }
        }
    }
    if total > 0 {
        godot::global::godot_print!(
            "[lod-verify post-generate] surfaces total={} with_lods={} without_lods={}",
            total,
            with_lods,
            without_lods,
        );
    }
}

/// Recursively clear the owner of a node and all its children
pub(super) fn clear_owner_recursive(node: &mut Gd<Node>) {
    node.set_owner(Gd::<Node>::null_arg());
    for mut child in node.get_children().iter_shared() {
        clear_owner_recursive(&mut child);
    }
}

/// Recursively set the owner of a node and all its children
pub(super) fn set_owner_recursive(node: &mut Gd<Node>, owner: &Gd<Node>) {
    node.set_owner(owner);
    for mut child in node.get_children().iter_shared() {
        set_owner_recursive(&mut child, owner);
    }
}

/// Parse GLTF/GLB file to extract dependencies (images and buffers).
/// Returns file paths as referenced in the GLTF (relative paths like "textures/image.png").
pub async fn get_dependencies(file_path: &str) -> Result<Vec<String>, anyhow::Error> {
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

/// Count the number of nodes in a tree
pub(super) fn count_nodes(node: Gd<Node>) -> i32 {
    let mut count = 1;
    for child in node.get_children().iter_shared() {
        count += count_nodes(child);
    }
    count
}

/// Common GLTF loading pipeline.
///
/// This handles the shared logic for loading scenes, wearables, and emotes:
/// 1. Download main GLTF file
/// 2. Parse and download dependencies
/// 3. Acquire Godot thread safety guard
/// 4. Load GltfDocument
/// 5. Post-process textures
/// 6. Rotate node 180° Y
/// 7. Call processor function for type-specific processing
/// 8. Cleanup source file
///
/// The processor function receives the loaded Node3D and should return
/// a tuple of (result, file_size). The caller is responsible for cache registration.
pub async fn load_gltf_pipeline<F, R>(
    file_path: String,
    file_hash: String,
    content_mapping: ContentMappingAndUrlRef,
    ctx: SceneGltfContext,
    processor: F,
) -> Result<(R, i64), anyhow::Error>
where
    F: FnOnce(Gd<Node3D>, &str, &SceneGltfContext) -> Result<(R, i64), anyhow::Error>,
{
    // Download the main GLTF file
    let base_path = Arc::new(get_base_dir(&file_path));
    let url = format!("{}{}", content_mapping.base_url, file_hash);
    let absolute_file_path = format!("{}{}", ctx.content_folder, file_hash);

    #[cfg(feature = "use_resource_tracking")]
    report_resource_start(&file_hash, "gltf");

    let gltf_result = ctx
        .resource_provider
        .fetch_resource(url, file_hash.clone(), absolute_file_path.clone())
        .await;

    #[cfg(feature = "use_resource_tracking")]
    if let Err(ref e) = gltf_result {
        report_resource_error(&file_hash, &e.to_string());
    }

    gltf_result.map_err(anyhow::Error::msg)?;

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
        let dep_hash = dependency_file_hash.clone();
        async move {
            #[cfg(feature = "use_resource_tracking")]
            report_resource_start(&dep_hash, "gltf_dep");

            let url = format!("{}{}", content_mapping.base_url, dep_hash);
            let absolute_file_path = format!("{}{}", ctx.content_folder, dep_hash);
            let result = ctx
                .resource_provider
                .fetch_resource(url, dep_hash.clone(), absolute_file_path)
                .await;

            #[cfg(feature = "use_resource_tracking")]
            if let Err(ref e) = result {
                report_resource_error(&dep_hash, &format!("{:?}", e));
            } else {
                report_resource_loaded(&dep_hash);
            }

            result.map_err(|e| format!("Dependency {} failed: {:?}", dep_hash, e))
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
    let (result, file_size) = {
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
        // Seed `placeholder_image` so the custom_gltf_importer.gd preflight read
        // hits an existing key (nil) instead of the missing-key Dictionary
        // warning. External avatar-image-generation overwrites this with a
        // truthy value when it wants the placeholder branch.
        new_gltf_state.set_additional_data("placeholder_image", &Variant::nil());

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

        // Pipeline order (asset-processor only): split → LODs → shadows → materials.
        //
        // split: chunk plaza-spanning meshes so each chunk has a small AABB
        //   that lets the renderer's screen-space-error LOD selector engage.
        // LODs: bake LOD chain on every MI (chunks + non-chunks), guarded by
        //   MIN_INDICES_FOR_LOD to avoid degenerate LODs on tiny surfaces.
        // shadows: per-MI shadow_mesh from the highest-LOD index buffer (most
        //   decimated). Works naturally on chunks because each chunk has its
        //   own LOD chain.
        // materials: post-everything pass to flip every BaseMaterial3D to
        //   PER_VERTEX. Runs LAST so the splitter + LOD roundtrip can't
        //   leave any chunk surface or override material on PER_PIXEL.
        //
        // Gated on `apply_optimizations` so phone-side loads pay zero cost —
        // the pipeline is meant to be baked once on the asset server and
        // saved into the .scn the phone consumes.
        if ctx.apply_optimizations {
            // mesh-split DISABLED: was producing white-material chunks in
            // multi-MI shared mesh groups (per-surface material not
            // propagated to chunk_mesh when the source MI relied on
            // material_override + an empty mesh.surface_get_material).
            // Re-enable once the material fan-out is correct.
            // apply_post_generate_mesh_split(node.clone(), node.clone());
            // apply_post_generate_godot_lods(node.clone()); // DIAG: bushes missing
            // verify_lods_in_generated_scene(node.clone());
        }

        let max_size = ctx.texture_quality.to_max_size();
        post_import_process(node.clone(), max_size, ctx.force_compress);

        let mut node = node
            .try_cast::<Node3D>()
            .map_err(|err| anyhow::Error::msg(format!("Error casting to Node3D: {err}")))?;
        node.rotate_y(std::f32::consts::PI);

        if ctx.apply_optimizations {
            let (mat_flipped, missing) = apply_post_material_overrides(&node.clone().upcast());
            godot::global::godot_print!(
                "[materials-post] flipped_to_per_vertex={} surfaces_without_material={}",
                mat_flipped,
                missing
            );
            // PVS bake runs in scene.rs::load_and_save_scene_gltf AFTER
            // create_scene_colliders has populated CollisionShape3D
            // nodes (our blocker source).
        }

        processor(node, &file_hash, &ctx)?
    };
    // All Godot objects are now dropped, safe to await

    // Cleanup source GLTF file after successful save
    // NOTE: We only delete the main GLTF file, NOT dependencies (textures/buffers).
    // Dependencies may be shared by multiple GLTFs loading in parallel.
    // They will be cleaned up by LRU eviction when the cache exceeds its limit.
    ctx.resource_provider
        .try_delete_file_by_hash(&file_hash)
        .await;

    #[cfg(feature = "use_resource_tracking")]
    report_resource_loaded(&file_hash);

    Ok((result, file_size))
}
