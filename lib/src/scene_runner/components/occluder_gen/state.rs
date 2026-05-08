//! Per-Scene state for occluder auto-gen.

use std::collections::HashSet;

use crate::dcl::components::SceneEntityId;

use super::metrics::OccluderStats;

#[derive(Default)]
pub struct OccluderGenState {
    pub stats: OccluderStats,
    pub classified: HashSet<SceneEntityId>,
    pub occluders_added: u32,
}
