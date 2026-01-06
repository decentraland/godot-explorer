//! Path matching logic for GLTF node modifiers.
//!
//! Supports multiple matching strategies for compatibility with Unity/Bevy:
//! - Empty path: global modifier, matches everything
//! - Full path with mesh: "Screen/PlaneScreen/Plane.105" (exact match)
//! - Node path: "Screen/PlaneScreen" (exact node path match)
//! - Node name only: "PlaneScreen" (matches any node with that name)
//! - Mesh name only: "Plane.105" (matches any mesh with that resource name)
//! - Prefix/subtree: "Screen" matches "Screen/PlaneScreen" and children
//! - Suffix match: partial paths match at the end of full paths
//! - Virtual primitive: "Sphere.001/Sphere_2" targets surface 2 on Sphere.001

use std::collections::HashMap;

use crate::dcl::components::proto_components::sdk::components::pb_gltf_node_modifiers::GltfNodeModifier;

use super::state::{MeshInstanceInfo, ModifierMatch};

/// Result of path matching - includes optional surface index for per-primitive targeting
#[derive(Clone, Copy)]
pub struct PathMatchResult {
    /// Whether the path matched
    pub matched: bool,
    /// Optional surface index if targeting a specific primitive (e.g., "Sphere_1" -> surface 1)
    pub surface_index: Option<i32>,
}

impl PathMatchResult {
    pub fn no_match() -> Self {
        Self {
            matched: false,
            surface_index: None,
        }
    }

    pub fn matched_all() -> Self {
        Self {
            matched: true,
            surface_index: None,
        }
    }

    pub fn matched_surface(index: i32) -> Self {
        Self {
            matched: true,
            surface_index: Some(index),
        }
    }
}

/// Key for resolved modifier mappings - combines node path and optional surface index
#[derive(Clone, Hash, PartialEq, Eq)]
pub struct ModifierKey {
    pub node_path: String,
    /// None means all surfaces, Some(i) means specific surface
    pub surface_index: Option<i32>,
}

impl ModifierKey {
    pub fn new(node_path: &str, surface_index: Option<i32>) -> Self {
        Self {
            node_path: node_path.to_string(),
            surface_index,
        }
    }

    pub fn all_surfaces(node_path: &str) -> Self {
        Self::new(node_path, None)
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
pub fn path_matches(modifier_path: &str, info: &MeshInstanceInfo) -> PathMatchResult {
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
pub fn normalize_segment(s: &str) -> String {
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

/// Resolve which modifier applies to each mesh instance.
/// Specific path modifiers take priority over global modifiers.
/// More specific matches (longer paths) take priority over less specific ones.
/// Returns map from (node_path, surface_index) to modifier match.
pub fn resolve_modifiers<'a>(
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
