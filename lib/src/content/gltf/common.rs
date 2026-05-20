//! Common utilities and pipeline for GLTF loading.

use std::sync::Arc;

use godot::{
    builtin::GString,
    classes::{
        base_material_3d::{ShadingMode, TextureParam},
        mesh::{ArrayType, PrimitiveType},
        BaseMaterial3D, GltfDocument, GltfState, ImageTexture, ImporterMesh, MeshInstance3D, Node,
        Node3D,
    },
    global::Error,
    meta::ToGodot,
    obj::Gd,
    prelude::*,
};
use meshopt::{simplify, SimplifyOptions, VertexDataAdapter};

use crate::godot_classes::dcl_global::DclGlobal;
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

/// Per-surface LOD chain generation on the GLTFState's ImporterMesh
/// array, run between `append_from_file_ex` and `generate_scene`. LOD0
/// (full quality) is preserved — every additional level is added via the
/// `lods` Dictionary slot on `add_surface`, keyed by screen-space error
/// threshold. Godot's renderer picks a lower LOD when an instance's
/// screen-space projection error exceeds the threshold (controlled by
/// the viewport's `mesh_lod_threshold` in pixels).
///
/// LOD generation uses `meshopt::simplify` with `Permissive | Sparse`:
/// vanilla simplify bails on DCL's user-authored GLBs (attribute
/// discontinuities at every UV seam → nothing to collapse) and would
/// return nearly the source. Permissive lifts that restriction.
fn apply_pre_generate_mesh_simplification(state: &mut Gd<GltfState>, _target_ratio: f32) {
    /// Ratios for additional LOD levels (LOD0 stays at full quality).
    /// Screen-space error keys are picked roughly proportional to the
    /// inverse of `1.0 / ratio`; the exact values matter less than their
    /// ordering — Godot picks the highest LOD whose error <= viewport
    /// `mesh_lod_threshold` at the instance's screen size.
    const LOD_LEVELS: &[(f32, f32)] = &[
        (0.5, 0.5),  // LOD1: half the indices, mid distance
        (0.25, 1.5), // LOD2: quarter, far distance
        (0.1, 3.0),  // LOD3: tenth, very far
    ];

    let mut meshes = state.get_meshes();
    let n = meshes.len();
    let mut surfaces_with_lods = 0u32;
    let mut surfaces_no_lods = 0u32;
    let mut src_idx_total: u64 = 0;
    let mut lod_idx_total: u64 = 0;
    for mi in 0..n {
        let mut gltf_mesh = meshes.at(mi);
        let Some(mut importer) = gltf_mesh.get_mesh() else {
            continue;
        };
        let surface_count = importer.get_surface_count();
        if surface_count == 0 {
            continue;
        }
        // Skip meshes with blend shapes (morph targets) — `add_surface`'s
        // `blend_shapes` parameter takes the full per-shape vertex stream
        // array, which we don't snapshot. Tweaking these meshes would
        // drop the morph target data and break any animation that drives
        // them (facial expressions, plant-sway micro-animations).
        if importer.get_blend_shape_count() > 0 {
            continue;
        }
        struct Snapshot {
            primitive: PrimitiveType,
            arrays: VarArray,
            material: Option<Gd<godot::classes::Material>>,
            name: String,
            flags: u64,
            // Empty when no LODs can/should be generated for this surface;
            // Godot then renders LOD0 (the unmodified arrays) at all
            // distances, identical to the pre-cheap-pbr behavior.
            lods: VarDictionary,
        }
        let mut snapshots: Vec<Snapshot> = Vec::with_capacity(surface_count as usize);
        for s in 0..surface_count {
            snapshots.push(Snapshot {
                primitive: importer.get_surface_primitive_type(s),
                arrays: importer.get_surface_arrays(s),
                material: importer.get_surface_material(s),
                name: importer.get_surface_name(s).to_string(),
                flags: importer.get_surface_format(s),
                lods: VarDictionary::new(),
            });
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
            // Below ~33 triangles (100 indices) the per-LOD overhead is
            // bigger than the rendering win, and meshopt's quadric error
            // metric becomes noisy on tiny meshes (sign of decoration
            // props that should stay full-res).
            if idx.len() < 100 {
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
            // Skinned surfaces (ARRAY_BONES non-null) deform under animation
            // — a decimated LOD with collapsed vertices doesn't follow the
            // bone-weighted transforms cleanly and visibly stretches or
            // collapses during animation. Leave skinned meshes at LOD0.
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
                    SimplifyOptions::Permissive | SimplifyOptions::Sparse,
                    None,
                );
                // Skip if meshopt couldn't get useful reduction at this
                // level — too close to source, not worth a draw-state
                // switch.
                if lod_indices.is_empty()
                    || lod_indices.len() as f32 / idx.len() as f32 > 0.9
                    || lod_indices.len() % 3 != 0
                {
                    continue;
                }
                let mut packed = PackedInt32Array::new();
                packed.resize(lod_indices.len());
                let slc = packed.as_mut_slice();
                for (k, &i) in lod_indices.iter().enumerate() {
                    slc[k] = i as i32;
                }
                snap.lods.insert(sse_key.to_variant(), packed.to_variant());
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
        // Clear and re-add. `add_surface` rebuilds the internal vertex
        // buffer and bookkeeping for each surface; LOD0 stays the source
        // arrays, additional LODs come in via `lods()`.
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

/// Mutate materials in `GLTFState` after `append_from_file_ex` but before
/// `generate_scene`. At this point the BaseMaterial3D resources are parsed
/// from the GLTF file but not yet bound to any scene node or rendered, so
/// changing MaterialKey-affecting properties (shading_mode, diffuse_mode)
/// does not trigger a shader variant recompile — the very first variant
/// compiled by the renderer will be the one we want.
fn apply_pre_generate_material_overrides(state: &mut Gd<GltfState>) {
    let materials = state.get_materials();
    for i in 0..materials.len() {
        let mat = materials.at(i);
        let Ok(mut base) = mat.try_cast::<BaseMaterial3D>() else {
            continue;
        };
        // Per-vertex lighting: cuts fragment ALU drastically on Mali, the
        // visual hit on opaque flat surfaces (DCL plaza walls) is small.
        // Setting it here keeps every material on a SINGLE MaterialKey
        // entry — they're all PER_VERTEX from the start, no fragmentation.
        base.set_shading_mode(ShadingMode::PER_VERTEX);
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

        // Pre-generate hook: GLTFState now contains the BaseMaterial3D
        // resources parsed from the file but the scene tree has NOT been
        // generated yet. Materials are not bound to any MeshInstance3D,
        // not registered with the renderer's shader_map. This is the
        // right place to mutate the material's MaterialKey-affecting
        // properties (shading_mode, diffuse_mode, etc) so that the FIRST
        // shader variant compiled is the desired one — no recompile, no
        // batching invalidation, no Mali driver lockup.
        if DclGlobal::try_singleton()
            .map(|g| g.bind().cli.bind().cheap_pbr_enabled)
            .unwrap_or(false)
        {
            apply_pre_generate_material_overrides(&mut new_gltf_state);
            apply_pre_generate_mesh_simplification(&mut new_gltf_state, 0.5);
        }

        let node = new_gltf
            .generate_scene(&new_gltf_state)
            .ok_or(anyhow::Error::msg("Error generating scene from gltf"))?;

        // Post-process textures (compress if on mobile or force_compress is set)
        let max_size = ctx.texture_quality.to_max_size();
        post_import_process(node.clone(), max_size, ctx.force_compress);

        // Cast to Node3D and rotate
        let mut node = node
            .try_cast::<Node3D>()
            .map_err(|err| anyhow::Error::msg(format!("Error casting to Node3D: {err}")))?;
        node.rotate_y(std::f32::consts::PI);

        // Call the type-specific processor
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
