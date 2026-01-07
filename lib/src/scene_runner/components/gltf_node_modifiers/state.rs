//! State tracking for GLTF node modifiers.

use std::collections::{HashMap, HashSet};

use godot::{
    classes::{geometry_instance_3d::ShadowCastingSetting, Material, MeshInstance3D},
    prelude::*,
};

use crate::dcl::components::{
    material::DclMaterial,
    proto_components::sdk::components::pb_gltf_node_modifiers::GltfNodeModifier,
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
    /// Hash of last applied modifiers to detect changes (avoids redundant reprocessing)
    pub last_modifiers_hash: u64,
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
}
