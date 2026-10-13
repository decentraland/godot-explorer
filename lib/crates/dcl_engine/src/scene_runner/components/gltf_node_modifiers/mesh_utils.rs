//! Mesh collection and traversal utilities for GLTF nodes.

use std::collections::HashMap;

use godot::{
    classes::{MeshInstance3D, Node, Node3D},
    prelude::*,
};

use crate::godot_classes::dcl_gltf_container::DclGltfContainer;

use super::state::MeshInstanceInfo;

/// Find the GltfContainer node for an entity
pub fn get_gltf_container(node_3d: &Gd<Node3D>) -> Option<Gd<DclGltfContainer>> {
    node_3d.try_get_node_as::<DclGltfContainer>("GltfContainer")
}

/// Collect all MeshInstance3D nodes in the GLTF hierarchy with full info
pub fn collect_all_mesh_instances(root: &Gd<Node3D>) -> Vec<MeshInstanceInfo> {
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
pub fn find_node_by_path(root: &Gd<Node3D>, path: &str) -> Option<Gd<Node3D>> {
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

/// Type alias for mesh instance collection with paths
pub type MeshInstancesWithPaths = Vec<(String, Gd<MeshInstance3D>)>;

/// Helper to collect mesh instances grouped by their base path
#[allow(clippy::type_complexity)]
pub fn collect_paths_with_meshes(root: &Gd<Node3D>) -> Vec<(String, MeshInstancesWithPaths)> {
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
