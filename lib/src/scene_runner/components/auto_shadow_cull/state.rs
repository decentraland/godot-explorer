use std::collections::HashSet;

use crate::dcl::components::SceneEntityId;

use super::metrics::CullStats;

#[derive(Default)]
pub struct AutoShadowCullState {
    pub stats: CullStats,
    pub classified: HashSet<SceneEntityId>,
    pub shadows_disabled: u32,
}
