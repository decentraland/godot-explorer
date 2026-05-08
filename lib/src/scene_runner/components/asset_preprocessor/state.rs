//! Per-Scene state for asset preprocessor.

use std::collections::HashSet;

use crate::dcl::components::SceneEntityId;

#[derive(Default)]
pub struct AssetPreprocessorState {
    pub classified: HashSet<SceneEntityId>,
    pub meshes_decimated: u32,
    pub meshes_stripped: u32,
    pub occluders_added: u32,
}
