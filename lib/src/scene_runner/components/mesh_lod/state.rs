//! Per-Scene state for the mesh-LOD pass.

use std::collections::HashSet;

use crate::dcl::components::SceneEntityId;

use super::metrics::LodStats;

#[derive(Default)]
pub struct MeshLodState {
    pub stats: LodStats,
    /// Entities the LOD pass has already inspected at least once. Avoids
    /// re-walking the same GLTF when the entity bounces back into the
    /// promotion queue.
    pub classified: HashSet<SceneEntityId>,

    pub meshes_baked: u32,
}
