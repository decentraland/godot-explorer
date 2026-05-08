//! Per-Scene state for the material atlas.

use std::collections::HashSet;

use crate::dcl::components::SceneEntityId;

use super::metrics::MergerStats;

#[derive(Default)]
pub struct MaterialAtlasState {
    pub stats: MergerStats,
    /// Entities the classifier has already inspected at least once. Avoids
    /// re-walking the same GLTF when the entity bounces back into the
    /// promotion queue (e.g. a transform write).
    pub classified: HashSet<SceneEntityId>,

    pub layers_allocated: u32,
    pub mis_replaced: u32,
}
