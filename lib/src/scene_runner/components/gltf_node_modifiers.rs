use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};

use godot::{
    classes::{
        base_material_3d::{
            EmissionOperator, Feature, Flags, ShadingMode, TextureParam, Transparency,
        },
        geometry_instance_3d::ShadowCastingSetting,
        Material, MeshInstance3D, Node, Node3D, StandardMaterial3D, Texture2D,
    },
    global::weakref,
    obj::GdMut,
    prelude::*,
};

use crate::{
    content::content_mapping::DclContentMappingAndUrl,
    dcl::{
        components::{
            material::{DclMaterial, DclSourceTex},
            proto_components::sdk::components::{
                pb_gltf_node_modifiers::GltfNodeModifier, MaterialTransparencyMode, PbMaterial,
            },
            SceneComponentId, SceneEntityId,
        },
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
    },
    godot_classes::{dcl_global::DclGlobal, dcl_gltf_container::DclGltfContainer},
    scene_runner::scene::Scene,
};

/// Tracks a material applied by GltfNodeModifiers that's waiting for textures
pub struct ModifierMaterialItem {
    pub dcl_material: DclMaterial,
    pub weak_ref: Variant,
    pub waiting_textures: bool,
}

/// State tracking for a single entity's GLTF node modifications.
/// Used to restore original state when modifiers are removed.
#[derive(Default)]
pub struct GltfNodeModifierState {
    /// Original materials for each node path (before any modifications)
    /// The Vec contains materials for each surface of the mesh
    pub original_materials: HashMap<String, Vec<Option<Gd<Material>>>>,
    /// Original shadow casting settings for each node path
    pub original_shadows: HashMap<String, ShadowCastingSetting>,
    /// Set of paths that currently have modifiers applied
    pub applied_paths: HashSet<String>,
    /// Materials waiting for textures to load, keyed by node path
    pub pending_materials: HashMap<String, ModifierMaterialItem>,
}

/// Find the GltfContainer node for an entity
fn get_gltf_container(node_3d: &Gd<Node3D>) -> Option<Gd<DclGltfContainer>> {
    node_3d.try_get_node_as::<DclGltfContainer>("GltfContainer")
}

/// Information about a mesh instance in the GLTF hierarchy
pub struct MeshInstanceInfo {
    /// Full node path (e.g., "Screen/PlaneScreen")
    pub node_path: String,
    /// Node name only (e.g., "PlaneScreen")
    pub node_name: String,
    /// Mesh resource name if available (e.g., "Plane.105")
    pub mesh_name: Option<String>,
    /// Full path including mesh name (e.g., "Screen/PlaneScreen/Plane.105")
    pub full_path: String,
    /// The actual MeshInstance3D node
    pub mesh_instance: Gd<MeshInstance3D>,
    /// Number of surfaces (primitives) in this mesh
    pub surface_count: i32,
}

/// Result of matching a modifier path to a mesh instance
#[derive(Clone)]
pub struct ModifierMatch<'a> {
    pub modifier: &'a GltfNodeModifier,
    /// Optional surface index for per-primitive targeting (None = all surfaces)
    pub surface_index: Option<i32>,
}

/// Collect all MeshInstance3D nodes in the GLTF hierarchy with full info
fn collect_all_mesh_instances(root: &Gd<Node3D>) -> Vec<MeshInstanceInfo> {
    let mut result = Vec::new();
    collect_mesh_instances_recursive(root, String::new(), &mut result);
    result
}

fn collect_mesh_instances_recursive(
    node: &Gd<Node3D>,
    current_path: String,
    result: &mut Vec<MeshInstanceInfo>,
) {
    // Check if this node is a MeshInstance3D
    if let Ok(mesh_instance) = node.clone().try_cast::<MeshInstance3D>() {
        let node_name = node.get_name().to_string();
        let mesh_name = mesh_instance
            .get_mesh()
            .map(|m| m.get_name().to_string())
            .filter(|n| !n.is_empty());

        let full_path = if let Some(ref mesh) = mesh_name {
            if current_path.is_empty() {
                mesh.clone()
            } else {
                format!("{}/{}", current_path, mesh)
            }
        } else {
            current_path.clone()
        };

        let surface_count = mesh_instance.get_surface_override_material_count();
        result.push(MeshInstanceInfo {
            node_path: current_path.clone(),
            node_name,
            mesh_name,
            full_path,
            mesh_instance,
            surface_count,
        });
    }

    // Recurse into children
    let child_count = node.get_child_count();
    for i in 0..child_count {
        if let Some(child) = node.get_child(i) {
            if let Ok(child_3d) = child.try_cast::<Node3D>() {
                let child_name = child_3d.get_name().to_string();
                let child_path = if current_path.is_empty() {
                    child_name
                } else {
                    format!("{}/{}", current_path, child_name)
                };
                collect_mesh_instances_recursive(&child_3d, child_path, result);
            }
        }
    }
}

/// Find a node in the GLTF hierarchy by path (e.g., "Parent/Child/Grandchild")
fn find_node_by_path(root: &Gd<Node3D>, path: &str) -> Option<Gd<Node3D>> {
    if path.is_empty() {
        return Some(root.clone());
    }

    let mut current: Gd<Node> = root.clone().upcast();

    for part in path.split('/').filter(|p| !p.is_empty()) {
        let mut found = false;
        let child_count = current.get_child_count();

        for i in 0..child_count {
            if let Some(child) = current.get_child(i) {
                if child.get_name().to_string() == part {
                    current = child;
                    found = true;
                    break;
                }
            }
        }

        if !found {
            return None;
        }
    }

    current.try_cast::<Node3D>().ok()
}

/// Capture original material state for a mesh instance
fn capture_original_materials(mesh: &Gd<MeshInstance3D>) -> Vec<Option<Gd<Material>>> {
    let surface_count = mesh.get_surface_override_material_count() as usize;
    let mut materials = Vec::with_capacity(surface_count);

    for i in 0..surface_count {
        materials.push(mesh.get_surface_override_material(i as i32));
    }

    materials
}

/// Restore original materials to a mesh instance
fn restore_original_materials(mesh: &mut Gd<MeshInstance3D>, materials: &[Option<Gd<Material>>]) {
    for (i, mat) in materials.iter().enumerate() {
        if let Some(material) = mat {
            mesh.set_surface_override_material(i as i32, material);
        } else {
            // Use Variant::nil() to properly clear the override
            mesh.call(
                "set_surface_override_material",
                &[(i as i32).to_variant(), Variant::nil()],
            );
        }
    }
}

/// Apply a material modifier to a mesh instance
/// If surface_index is Some, only applies to that surface; otherwise applies to all surfaces.
/// Returns the DclMaterial and Godot material if textures need loading
fn apply_material_to_mesh(
    mesh: &mut Gd<MeshInstance3D>,
    material: &PbMaterial,
    content_mapping: &crate::content::content_mapping::ContentMappingAndUrlRef,
    surface_index: Option<i32>,
) -> Option<(DclMaterial, Gd<StandardMaterial3D>)> {
    let mat = material.material.as_ref()?;
    let dcl_material = DclMaterial::from_proto(mat, content_mapping);
    let godot_material = create_godot_material_from_dcl(&dcl_material);

    // Request texture fetches
    let mut content_provider = DclGlobal::singleton().bind().get_content_provider();
    for tex in dcl_material.get_textures().into_iter().flatten() {
        if let DclSourceTex::Texture(hash) = &tex.source {
            content_provider.call_deferred(
                "fetch_texture_by_hash",
                &[
                    hash.to_godot().to_variant(),
                    DclContentMappingAndUrl::from_ref(content_mapping.clone()).to_variant(),
                ],
            );
        }
    }

    // Apply to specified surface(s)
    if let Some(idx) = surface_index {
        // Apply to specific surface only
        if idx < mesh.get_surface_override_material_count() {
            mesh.set_surface_override_material(idx, &godot_material.clone().upcast::<Material>());
        }
    } else {
        // Apply to all surfaces
        let surface_count = mesh.get_surface_override_material_count();
        for i in 0..surface_count {
            mesh.set_surface_override_material(i, &godot_material.clone().upcast::<Material>());
        }
    }

    // Check if we need to wait for textures
    let has_textures = match &dcl_material {
        DclMaterial::Unlit(unlit) => unlit.texture.is_some(),
        DclMaterial::Pbr(pbr) => {
            pbr.texture.is_some()
                || pbr.bump_texture.is_some()
                || pbr.alpha_texture.is_some()
                || pbr.emissive_texture.is_some()
        }
    };

    if has_textures {
        Some((dcl_material, godot_material))
    } else {
        None
    }
}

/// Apply shadow casting modifier to a mesh instance
/// If surface_index is Some, this is a per-surface modifier (but shadows apply to whole mesh)
fn apply_shadow_to_mesh(
    mesh: &mut Gd<MeshInstance3D>,
    cast_shadows: bool,
    _surface_index: Option<i32>,
) {
    // Note: Shadow casting is per-mesh, not per-surface, so surface_index is ignored
    let setting = if cast_shadows {
        ShadowCastingSetting::ON
    } else {
        ShadowCastingSetting::OFF
    };
    mesh.set_cast_shadows_setting(setting);
}

/// Create a Godot StandardMaterial3D from a DclMaterial
fn create_godot_material_from_dcl(dcl_material: &DclMaterial) -> Gd<StandardMaterial3D> {
    let mut godot_material = StandardMaterial3D::new_gd();

    match dcl_material {
        DclMaterial::Unlit(unlit) => {
            godot_material.set_metallic(0.0);
            godot_material.set_roughness(0.0);
            godot_material.set_specular(0.0);

            godot_material.set_shading_mode(ShadingMode::UNSHADED);
            godot_material.set_flag(Flags::ALBEDO_TEXTURE_FORCE_SRGB, true);
            godot_material.set_albedo(unlit.diffuse_color.0.to_godot().linear_to_srgb());

            // Apply UV offset/tiling from main texture (only main texture supports this)
            if let Some(texture) = &unlit.texture {
                godot_material.set_uv1_offset(godot::builtin::Vector3::new(
                    texture.offset.0.x,
                    texture.offset.0.y,
                    0.0,
                ));
                godot_material.set_uv1_scale(godot::builtin::Vector3::new(
                    texture.tiling.0.x,
                    texture.tiling.0.y,
                    1.0,
                ));
            }

            // Handle transparency for unlit materials (auto-detect)
            if unlit.diffuse_color.0.a < 1.0 || unlit.texture.is_some() {
                godot_material.set_transparency(Transparency::ALPHA_DEPTH_PRE_PASS);
            } else {
                godot_material.set_transparency(Transparency::DISABLED);
            }
        }
        DclMaterial::Pbr(pbr) => {
            godot_material.set_metallic(pbr.metallic.0);
            godot_material.set_roughness(pbr.roughness.0);
            godot_material.set_specular(pbr.specular_intensity.0);

            godot_material.set_emission(pbr.emissive_color.0.to_godot());
            godot_material.set_emission_energy_multiplier(pbr.emissive_intensity.0);
            godot_material.set_feature(Feature::EMISSION, true);

            // Use MULTIPLY operator when there's an emissive texture
            if pbr.emissive_texture.is_some() {
                godot_material.set_emission_operator(EmissionOperator::MULTIPLY);
            }

            godot_material.set_flag(Flags::ALBEDO_TEXTURE_FORCE_SRGB, true);
            godot_material.set_albedo(pbr.albedo_color.0.to_godot());

            // Apply UV offset/tiling from main texture (only main texture supports this)
            if let Some(texture) = &pbr.texture {
                godot_material.set_uv1_offset(godot::builtin::Vector3::new(
                    texture.offset.0.x,
                    texture.offset.0.y,
                    0.0,
                ));
                godot_material.set_uv1_scale(godot::builtin::Vector3::new(
                    texture.tiling.0.x,
                    texture.tiling.0.y,
                    1.0,
                ));
            }

            // Handle transparency mode
            match pbr.transparency_mode {
                MaterialTransparencyMode::MtmOpaque => {
                    godot_material.set_transparency(Transparency::DISABLED);
                }
                MaterialTransparencyMode::MtmAlphaTest => {
                    godot_material.set_transparency(Transparency::ALPHA_SCISSOR);
                    godot_material.set_alpha_scissor_threshold(pbr.alpha_test.0);
                }
                MaterialTransparencyMode::MtmAlphaBlend => {
                    godot_material.set_transparency(Transparency::ALPHA_DEPTH_PRE_PASS);
                }
                MaterialTransparencyMode::MtmAlphaTestAndAlphaBlend => {
                    godot_material.set_transparency(Transparency::ALPHA_DEPTH_PRE_PASS);
                    godot_material.set_alpha_scissor_threshold(pbr.alpha_test.0);
                }
                MaterialTransparencyMode::MtmAuto => {
                    // Auto-detect: use alpha blend if albedo has transparency
                    if pbr.albedo_color.0.a < 1.0 || pbr.texture.is_some() {
                        godot_material.set_transparency(Transparency::ALPHA_DEPTH_PRE_PASS);
                    } else {
                        godot_material.set_transparency(Transparency::DISABLED);
                    }
                }
            }
        }
    }

    godot_material
}

/// Result of path matching - includes optional surface index for per-primitive targeting
#[derive(Clone, Copy)]
pub struct PathMatchResult {
    /// Whether the path matched
    pub matched: bool,
    /// Optional surface index if targeting a specific primitive (e.g., "Sphere_1" -> surface 1)
    pub surface_index: Option<i32>,
}

impl PathMatchResult {
    fn no_match() -> Self {
        Self {
            matched: false,
            surface_index: None,
        }
    }

    fn matched_all() -> Self {
        Self {
            matched: true,
            surface_index: None,
        }
    }

    fn matched_surface(index: i32) -> Self {
        Self {
            matched: true,
            surface_index: Some(index),
        }
    }
}

/// Try to parse a virtual primitive suffix from a path segment.
/// SDK convention: "NodeName_N" where N is a 0-based primitive index.
/// Returns (base_name, surface_index) if parsing succeeds.
fn parse_virtual_primitive(segment: &str) -> Option<(&str, i32)> {
    // Look for pattern like "Sphere_1" where the number after underscore is the surface index
    if let Some(last_underscore) = segment.rfind('_') {
        let (base, num_part) = segment.split_at(last_underscore);
        // Skip the underscore
        let num_str = &num_part[1..];
        if let Ok(index) = num_str.parse::<i32>() {
            // Only match single digit suffixes (0-9) to avoid false positives
            // Also don't match if the base name is empty
            if !base.is_empty() && num_str.len() <= 2 {
                return Some((base, index));
            }
        }
    }
    None
}

/// Check if a modifier path matches a mesh instance.
/// Supports multiple matching strategies for compatibility with Unity/Bevy:
/// - Empty path: global modifier, matches everything
/// - Full path with mesh: "Screen/PlaneScreen/Plane.105" (exact match)
/// - Node path: "Screen/PlaneScreen" (exact node path match)
/// - Node name only: "PlaneScreen" (matches any node with that name)
/// - Mesh name only: "Plane.105" (matches any mesh with that resource name)
/// - Prefix/subtree: "Screen" matches "Screen/PlaneScreen" and children
/// - Suffix match: partial paths match at the end of full paths
/// - Virtual primitive: "Sphere.001/Sphere_2" targets surface 2 on Sphere.001
fn path_matches(modifier_path: &str, info: &MeshInstanceInfo) -> PathMatchResult {
    if modifier_path.is_empty() {
        return PathMatchResult::matched_all();
    }

    // Check if the last segment is a virtual primitive reference
    let segments: Vec<&str> = modifier_path.split('/').collect();
    let last_segment = segments.last().unwrap_or(&"");

    // Try to parse virtual primitive targeting (e.g., "Parent/Sphere_2")
    if let Some((primitive_base, surface_index)) = parse_virtual_primitive(last_segment) {
        // Build the parent path without the virtual primitive
        let parent_path = if segments.len() > 1 {
            segments[..segments.len() - 1].join("/")
        } else {
            String::new()
        };

        // Check if the parent path matches this mesh instance
        let parent_matches = if parent_path.is_empty() {
            // Just primitive name, match any mesh whose name starts with the base
            info.node_name.starts_with(primitive_base)
                || normalize_segment(&info.node_name)
                    .starts_with(&normalize_segment(primitive_base))
        } else {
            // Check parent path against this node
            path_matches_basic(&parent_path, info)
        };

        if parent_matches && surface_index < info.surface_count {
            return PathMatchResult::matched_surface(surface_index);
        }
    }

    // Standard path matching
    if path_matches_basic(modifier_path, info) {
        return PathMatchResult::matched_all();
    }

    PathMatchResult::no_match()
}

/// Basic path matching without virtual primitive parsing
fn path_matches_basic(modifier_path: &str, info: &MeshInstanceInfo) -> bool {
    if modifier_path.is_empty() {
        return true;
    }

    // 1. Exact full path match (node_path/mesh_name)
    if info.full_path == modifier_path {
        return true;
    }

    // 2. Exact node path match
    if info.node_path == modifier_path {
        return true;
    }

    // 3. Node name only match
    if info.node_name == modifier_path {
        return true;
    }

    // 4. Mesh name only match
    if let Some(ref mesh_name) = info.mesh_name {
        if mesh_name == modifier_path {
            return true;
        }
    }

    // 5. Prefix/subtree match
    if info.node_path.starts_with(&format!("{}/", modifier_path)) {
        return true;
    }

    // 6. Suffix match on full path
    if info.full_path.ends_with(&format!("/{}", modifier_path)) {
        return true;
    }

    // 7. Suffix match on node path
    if info.node_path.ends_with(&format!("/{}", modifier_path)) {
        return true;
    }

    // 8. Fuzzy matching for skeletal meshes and Godot naming conventions
    // Godot may:
    // - Insert "Skeleton3D" nodes for skeletal meshes
    // - Replace "." with "_" in node names (e.g., "Sphere.001" -> "Sphere_001")
    let modifier_segments: Vec<&str> = modifier_path.split('/').collect();
    let node_segments: Vec<&str> = info.node_path.split('/').collect();

    if segments_match_fuzzy(&modifier_segments, &node_segments) {
        return true;
    }

    false
}

/// Normalize a segment name for comparison (handle Godot's "." -> "_" conversion)
fn normalize_segment(s: &str) -> String {
    s.replace('.', "_")
}

/// Check if pattern segments match target segments with fuzzy matching.
/// Allows:
/// - Extra segments in target (like "Skeleton3D" inserted by Godot)
/// - "." replaced with "_" in segment names
fn segments_match_fuzzy(pattern: &[&str], target: &[&str]) -> bool {
    if pattern.is_empty() {
        return true;
    }

    let mut pattern_idx = 0;
    for target_segment in target {
        let pattern_segment = pattern[pattern_idx];

        // Check for exact match or normalized match (. -> _)
        if *target_segment == pattern_segment
            || normalize_segment(target_segment) == normalize_segment(pattern_segment)
        {
            pattern_idx += 1;
            if pattern_idx == pattern.len() {
                return true;
            }
        }
        // Skip segments like "Skeleton3D" that Godot inserts
    }

    false
}

/// Key for resolved modifier mappings - combines node path and optional surface index
#[derive(Clone, Hash, PartialEq, Eq)]
struct ModifierKey {
    node_path: String,
    /// None means all surfaces, Some(i) means specific surface
    surface_index: Option<i32>,
}

impl ModifierKey {
    fn new(node_path: &str, surface_index: Option<i32>) -> Self {
        Self {
            node_path: node_path.to_string(),
            surface_index,
        }
    }

    fn all_surfaces(node_path: &str) -> Self {
        Self::new(node_path, None)
    }
}

/// Resolve which modifier applies to each mesh instance.
/// Specific path modifiers take priority over global modifiers.
/// More specific matches (longer paths) take priority over less specific ones.
/// Returns map from (node_path, surface_index) to modifier match.
fn resolve_modifiers<'a>(
    modifiers: &'a [GltfNodeModifier],
    all_meshes: &[MeshInstanceInfo],
) -> HashMap<ModifierKey, ModifierMatch<'a>> {
    let mut resolved: HashMap<ModifierKey, ModifierMatch> = HashMap::new();

    // First pass: find global modifier (empty path)
    let global_modifier = modifiers.iter().find(|m| m.path.is_empty());

    // If there's a global modifier, apply it to all meshes (all surfaces)
    if let Some(global) = global_modifier {
        for info in all_meshes {
            resolved.insert(
                ModifierKey::all_surfaces(&info.node_path),
                ModifierMatch {
                    modifier: global,
                    surface_index: None,
                },
            );
        }
    }

    // Second pass: specific paths override globals
    // Sort modifiers by path length (shorter first) so more specific paths override
    let mut specific_modifiers: Vec<_> = modifiers.iter().filter(|m| !m.path.is_empty()).collect();
    specific_modifiers.sort_by_key(|m| m.path.len());

    for modifier in &specific_modifiers {
        let mut matched = false;
        for info in all_meshes {
            let match_result = path_matches(&modifier.path, info);
            if match_result.matched {
                let key = ModifierKey::new(&info.node_path, match_result.surface_index);
                resolved.insert(
                    key,
                    ModifierMatch {
                        modifier,
                        surface_index: match_result.surface_index,
                    },
                );
                matched = true;
            }
        }

        // Debug: log when a modifier path doesn't match any mesh
        if !matched {
            let available_paths: Vec<&str> =
                all_meshes.iter().map(|i| i.node_path.as_str()).collect();
            let surface_counts: Vec<(&str, i32)> = all_meshes
                .iter()
                .map(|i| (i.node_path.as_str(), i.surface_count))
                .collect();
            tracing::warn!(
                "GltfNodeModifiers: path '{}' did not match any node. Available paths: {:?}, surface counts: {:?}",
                modifier.path,
                available_paths,
                surface_counts
            );
        }
    }

    resolved
}

pub fn update_gltf_node_modifiers(
    scene: &mut Scene,
    crdt_state: &mut SceneCrdtState,
    ref_time: &Instant,
    end_time_us: i64,
) -> bool {
    // Get dirty entities from CRDT (read-only access to dirty state)
    let gltf_node_modifiers_dirty = scene
        .current_dirty
        .lww_components
        .get(&SceneComponentId::GLTF_NODE_MODIFIERS)
        .cloned();

    // Check if there are pending entities (without draining yet)
    let has_pending = !scene.gltf_node_modifiers_pending.is_empty();
    let has_dirty = gltf_node_modifiers_dirty
        .as_ref()
        .is_some_and(|d| !d.is_empty());

    // Early exit if nothing to do
    if !has_dirty && !has_pending {
        return true;
    }

    tracing::debug!(
        "update_gltf_node_modifiers: has_dirty={}, has_pending={}",
        has_dirty,
        has_pending
    );

    let mut updated_count = 0;
    let mut current_time_us;

    // Now drain pending entities
    let pending_entities: Vec<SceneEntityId> = scene.gltf_node_modifiers_pending.drain().collect();

    let gltf_node_modifiers_component =
        SceneCrdtStateProtoComponents::get_gltf_node_modifiers(crdt_state);

    // Clone content_mapping reference before mutable borrows
    let content_mapping = scene.content_mapping.clone();

    // Combine both sources of entities to process
    let entities_to_process: Vec<SceneEntityId> = {
        let mut entities = pending_entities;
        if let Some(dirty) = &gltf_node_modifiers_dirty {
            for entity in dirty {
                if !entities.contains(entity) {
                    entities.push(*entity);
                }
            }
        }
        entities
    };

    if !entities_to_process.is_empty() {
        for entity in entities_to_process.iter() {
            let new_value = gltf_node_modifiers_component.get(entity);

            // Skip entities that have no GltfNodeModifiers component AND no existing state
            // This avoids processing entities that never had the component
            let has_component = new_value
                .and_then(|v| v.value.as_ref())
                .is_some_and(|v| !v.modifiers.is_empty());
            let has_existing_state = scene.gltf_node_modifier_states.contains_key(entity);

            tracing::debug!(
                "Processing entity {:?}: has_component={}, has_existing_state={}",
                entity,
                has_component,
                has_existing_state
            );

            if !has_component && !has_existing_state {
                tracing::debug!("Skipping entity {:?} - no component and no state", entity);
                updated_count += 1;
                continue;
            }

            let godot_dcl_scene = &mut scene.godot_dcl_scene;
            let (_, node_3d) = godot_dcl_scene.ensure_node_3d(entity);

            // Get the GltfContainer to access the loaded GLTF
            let Some(gltf_container) = get_gltf_container(&node_3d) else {
                // No GltfContainer on this entity, skip
                // Could be still loading - will be processed when GLTF finishes loading
                tracing::debug!("Entity {:?}: No GltfContainer found, skipping", entity);
                updated_count += 1;
                continue;
            };

            let Some(gltf_root) = gltf_container.bind().get_gltf_resource() else {
                // GLTF not loaded yet, skip - will be processed when loading completes
                tracing::debug!(
                    "Entity {:?}: GltfContainer found but get_gltf_resource() returned None, skipping",
                    entity
                );
                updated_count += 1;
                continue;
            };

            tracing::debug!(
                "Entity {:?}: GltfContainer and gltf_root found, processing modifiers",
                entity
            );

            // Get or create state for this entity
            let state = scene.gltf_node_modifier_states.entry(*entity).or_default();

            // Handle component removal or empty modifiers
            let modifiers = new_value
                .and_then(|v| v.value.as_ref())
                .map(|v| v.modifiers.as_slice())
                .unwrap_or(&[]);

            tracing::debug!(
                "Entity {:?}: modifiers.len()={}, original_materials.len()={}",
                entity,
                modifiers.len(),
                state.original_materials.len()
            );

            if modifiers.is_empty() {
                // Component was removed or has no modifiers - restore all original states
                tracing::debug!(
                    "Entity {:?}: modifiers empty, restoring original state",
                    entity
                );
                for (_path, mesh_instances) in collect_paths_with_meshes(&gltf_root) {
                    for (full_path, mut mesh) in mesh_instances {
                        if let Some(original_materials) = state.original_materials.get(&full_path) {
                            restore_original_materials(&mut mesh, original_materials);
                        }
                        if let Some(original_shadow) = state.original_shadows.get(&full_path) {
                            mesh.set_cast_shadows_setting(*original_shadow);
                        }
                    }
                }

                // Clear state
                state.original_materials.clear();
                state.original_shadows.clear();
                state.applied_paths.clear();

                updated_count += 1;
                current_time_us = (std::time::Instant::now() - *ref_time).as_micros() as i64;
                if current_time_us > end_time_us {
                    break;
                }
                continue;
            }

            // Collect all mesh instances in the GLTF
            let all_meshes = collect_all_mesh_instances(&gltf_root);

            // Resolve which modifier applies to each mesh (and optionally which surface)
            let resolved = resolve_modifiers(modifiers, &all_meshes);

            // Track which paths are now being modified (using node_path for simplicity)
            let new_applied_paths: HashSet<String> =
                resolved.keys().map(|k| k.node_path.clone()).collect();

            // Find paths that were previously modified but no longer have modifiers
            let paths_to_restore: Vec<String> = state
                .applied_paths
                .difference(&new_applied_paths)
                .cloned()
                .collect();

            // Restore paths that no longer have modifiers
            for path in paths_to_restore {
                if let Some(node) = find_node_by_path(&gltf_root, &path) {
                    if let Ok(mut mesh) = node.try_cast::<MeshInstance3D>() {
                        if let Some(original_materials) = state.original_materials.get(&path) {
                            restore_original_materials(&mut mesh, original_materials);
                        }
                        if let Some(original_shadow) = state.original_shadows.get(&path) {
                            mesh.set_cast_shadows_setting(*original_shadow);
                        }
                    }
                }
            }

            // Apply modifiers to each mesh
            for info in all_meshes {
                let path = &info.node_path;
                let mut mesh = info.mesh_instance;

                // Capture original state if not already captured (before applying any modifiers)
                if !state.original_materials.contains_key(path) {
                    state
                        .original_materials
                        .insert(path.clone(), capture_original_materials(&mesh));
                    state
                        .original_shadows
                        .insert(path.clone(), mesh.get_cast_shadows_setting());
                }

                // Check for "all surfaces" modifier first
                let all_surfaces_key = ModifierKey::all_surfaces(path);
                if let Some(modifier_match) = resolved.get(&all_surfaces_key) {
                    // Apply shadow casting if specified
                    if let Some(cast_shadows) = modifier_match.modifier.cast_shadows {
                        apply_shadow_to_mesh(&mut mesh, cast_shadows, None);
                    }

                    // Apply material if specified (to all surfaces)
                    if let Some(material) = &modifier_match.modifier.material {
                        if let Some((dcl_material, godot_material)) =
                            apply_material_to_mesh(&mut mesh, material, &content_mapping, None)
                        {
                            // Track material for texture loading
                            state.pending_materials.insert(
                                path.clone(),
                                ModifierMaterialItem {
                                    dcl_material,
                                    weak_ref: weakref(&godot_material.to_variant()),
                                    waiting_textures: true,
                                },
                            );
                        }
                    }
                }

                // Check for per-surface modifiers (can override "all surfaces" for specific surfaces)
                for surface_idx in 0..info.surface_count {
                    let surface_key = ModifierKey::new(path, Some(surface_idx));
                    if let Some(modifier_match) = resolved.get(&surface_key) {
                        // Per-surface shadow is applied to whole mesh (shadows are mesh-level)
                        if let Some(cast_shadows) = modifier_match.modifier.cast_shadows {
                            apply_shadow_to_mesh(&mut mesh, cast_shadows, Some(surface_idx));
                        }

                        // Apply material to this specific surface
                        if let Some(material) = &modifier_match.modifier.material {
                            if let Some((dcl_material, godot_material)) = apply_material_to_mesh(
                                &mut mesh,
                                material,
                                &content_mapping,
                                Some(surface_idx),
                            ) {
                                // Track material for texture loading with surface-specific key
                                let material_key = format!("{}:{}", path, surface_idx);
                                state.pending_materials.insert(
                                    material_key,
                                    ModifierMaterialItem {
                                        dcl_material,
                                        weak_ref: weakref(&godot_material.to_variant()),
                                        waiting_textures: true,
                                    },
                                );
                            }
                        }
                    }
                }
            }

            // Update applied paths
            state.applied_paths = new_applied_paths;

            updated_count += 1;
            current_time_us = (std::time::Instant::now() - *ref_time).as_micros() as i64;
            if current_time_us > end_time_us {
                break;
            }
        }

        // If we didn't process all entities, re-add remaining to pending set
        if updated_count < entities_to_process.len() {
            for entity in entities_to_process.iter().skip(updated_count) {
                scene.gltf_node_modifiers_pending.insert(*entity);
            }
            return false;
        }
    }

    true
}

/// Type alias for mesh instance collection with paths
type MeshInstancesWithPaths = Vec<(String, Gd<MeshInstance3D>)>;

/// Helper to collect mesh instances grouped by their base path
#[allow(clippy::type_complexity)]
fn collect_paths_with_meshes(root: &Gd<Node3D>) -> Vec<(String, MeshInstancesWithPaths)> {
    let meshes = collect_all_mesh_instances(root);
    // Group by top-level path
    let mut grouped: HashMap<String, MeshInstancesWithPaths> = HashMap::new();
    for info in meshes {
        let top_level = info.node_path.split('/').next().unwrap_or("").to_string();
        grouped
            .entry(top_level)
            .or_default()
            .push((info.node_path, info.mesh_instance));
    }
    grouped.into_iter().collect()
}

/// Check and apply pending textures for modifier materials
pub fn update_modifier_textures(scene: &mut Scene) {
    // Early exit if no states have pending materials
    let has_pending = scene
        .gltf_node_modifier_states
        .values()
        .any(|state| !state.pending_materials.is_empty());

    if !has_pending {
        return;
    }

    let mut content_provider = DclGlobal::singleton().bind().get_content_provider();

    for state in scene.gltf_node_modifier_states.values_mut() {
        if state.pending_materials.is_empty() {
            continue;
        }

        // Remove entries where material is no longer valid
        state.pending_materials.retain(|_, item| {
            if !item.waiting_textures {
                return false;
            }

            // Check if material is still valid
            let material_variant = item.weak_ref.call("get_ref", &[]);
            if material_variant.is_nil() {
                return false;
            }

            let Ok(material) = material_variant.try_to::<Gd<StandardMaterial3D>>() else {
                return false;
            };

            // Try to apply textures
            let all_loaded =
                check_and_apply_textures(&item.dcl_material, material, content_provider.bind_mut());
            item.waiting_textures = !all_loaded;

            // Keep if still waiting
            item.waiting_textures
        });
    }
}

/// Check if textures are loaded and apply them to the material
fn check_and_apply_textures(
    dcl_material: &DclMaterial,
    mut material: Gd<StandardMaterial3D>,
    mut content_provider: GdMut<crate::content::content_provider::ContentProvider>,
) -> bool {
    let mut all_loaded = true;

    match dcl_material {
        DclMaterial::Unlit(unlit) => {
            if !check_texture(
                TextureParam::ALBEDO,
                &unlit.texture,
                &mut material,
                &mut content_provider,
            ) {
                all_loaded = false;
            }
        }
        DclMaterial::Pbr(pbr) => {
            if !check_texture(
                TextureParam::ALBEDO,
                &pbr.texture,
                &mut material,
                &mut content_provider,
            ) {
                all_loaded = false;
            }
            if !check_texture(
                TextureParam::NORMAL,
                &pbr.bump_texture,
                &mut material,
                &mut content_provider,
            ) {
                all_loaded = false;
            }
            if !check_texture(
                TextureParam::EMISSION,
                &pbr.emissive_texture,
                &mut material,
                &mut content_provider,
            ) {
                all_loaded = false;
            }
            // Alpha texture uses the same slot as albedo in Godot
            if pbr.alpha_texture.is_some()
                && pbr.texture.is_none()
                && !check_texture(
                    TextureParam::ALBEDO,
                    &pbr.alpha_texture,
                    &mut material,
                    &mut content_provider,
                )
            {
                all_loaded = false;
            }
        }
    }

    all_loaded
}

/// Check if a single texture is loaded and apply it
fn check_texture(
    param: TextureParam,
    dcl_texture: &Option<crate::dcl::components::material::DclTexture>,
    material: &mut Gd<StandardMaterial3D>,
    content_provider: &mut GdMut<crate::content::content_provider::ContentProvider>,
) -> bool {
    let Some(dcl_texture) = dcl_texture else {
        return true;
    };

    match &dcl_texture.source {
        DclSourceTex::Texture(content_hash) => {
            if content_provider.is_resource_from_hash_loaded(content_hash.to_godot()) {
                if let Some(resource) =
                    content_provider.get_texture_from_hash(content_hash.to_godot())
                {
                    material.set_texture(param, &resource.upcast::<Texture2D>());
                }
                true
            } else {
                false
            }
        }
        DclSourceTex::VideoTexture(_) => {
            // Video textures need to be updated every frame
            // Return false to keep the material in pending state
            // Actual texture binding happens in update_modifier_video_textures()
            false
        }
        DclSourceTex::AvatarTexture(_) => {
            // Avatar textures not supported in GltfNodeModifiers
            true
        }
    }
}

/// Update video textures on modifier materials.
/// This is called separately because video textures need mutable access to video_players.
pub fn update_modifier_video_textures(scene: &mut Scene) {
    // Early exit if no states have pending materials
    let has_pending = scene
        .gltf_node_modifier_states
        .values()
        .any(|state| !state.pending_materials.is_empty());

    if !has_pending {
        return;
    }

    // Collect video texture bindings we need to update
    // Format: (material weak_ref, texture_param, video_entity_id)
    let mut video_texture_updates: Vec<(Variant, TextureParam, SceneEntityId)> = Vec::new();

    for state in scene.gltf_node_modifier_states.values() {
        if state.pending_materials.is_empty() {
            continue;
        }

        for item in state.pending_materials.values() {
            if !item.waiting_textures {
                continue;
            }

            let material_ref = item.weak_ref.call("get_ref", &[]);
            if material_ref.is_nil() {
                continue;
            }

            // Collect video textures based on material type
            let textures_to_check: Vec<(
                TextureParam,
                &Option<crate::dcl::components::material::DclTexture>,
            )> = match &item.dcl_material {
                DclMaterial::Unlit(unlit) => {
                    vec![(TextureParam::ALBEDO, &unlit.texture)]
                }
                DclMaterial::Pbr(pbr) => {
                    vec![
                        (TextureParam::ALBEDO, &pbr.texture),
                        (TextureParam::NORMAL, &pbr.bump_texture),
                        (TextureParam::EMISSION, &pbr.emissive_texture),
                    ]
                }
            };

            for (param, dcl_texture) in textures_to_check {
                if let Some(tex) = dcl_texture {
                    if let DclSourceTex::VideoTexture(video_entity_id) = &tex.source {
                        video_texture_updates.push((material_ref.clone(), param, *video_entity_id));
                    }
                }
            }
        }
    }

    // Now apply the video textures (we can mutably borrow video_players here)
    for (material_ref, param, video_entity_id) in video_texture_updates {
        if let Some(video_player) = scene.video_players.get_mut(&video_entity_id) {
            let mut material = material_ref.to::<Gd<StandardMaterial3D>>();

            // Try get_backend_texture first (works for ExoPlayer's ExternalTexture)
            let backend_texture = video_player.bind_mut().get_backend_texture();
            if let Some(texture) = backend_texture {
                material.set_texture(param, &texture.upcast::<Texture2D>());
            } else {
                // Fallback to dcl_texture (works for LiveKit's ImageTexture)
                if let Some(texture) = video_player.bind().get_dcl_texture() {
                    material.set_texture(param, &texture.upcast::<Texture2D>());
                }
            }
        }
    }
}
