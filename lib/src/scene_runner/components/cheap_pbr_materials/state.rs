use std::collections::HashSet;

use crate::dcl::components::SceneEntityId;

#[derive(Default)]
pub struct CheapPbrState {
    pub classified: HashSet<SceneEntityId>,
    pub materials_tweaked: u32,
}
