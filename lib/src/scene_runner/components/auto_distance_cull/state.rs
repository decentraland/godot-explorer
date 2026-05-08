//! Per-Scene state for the auto-distance-cull pass.

use std::collections::HashSet;

use crate::dcl::components::SceneEntityId;

use super::metrics::CullStats;

#[derive(Default)]
pub struct AutoDistanceCullState {
    pub stats: CullStats,
    pub classified: HashSet<SceneEntityId>,
    pub mis_set: u32,
}
