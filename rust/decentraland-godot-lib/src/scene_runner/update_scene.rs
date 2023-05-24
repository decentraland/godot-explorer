use super::{
    components::{
        mesh_renderer::update_mesh_renderer, transform_and_parent::update_transform_and_parent,
    },
    Scene,
};
use crate::dcl::{crdt::SceneCrdtState, DirtyComponents, DirtyEntities};

pub fn update_scene(
    _dt: f64,
    scene: &mut Scene,
    crdt_state: &mut SceneCrdtState,
    _dirty_entities: &DirtyEntities,
    dirty_components: &DirtyComponents,
) {
    scene.waiting_for_updates = false;

    update_transform_and_parent(
        &mut scene.godot_dcl_scene,
        crdt_state,
        _dirty_entities,
        dirty_components,
    );
    update_mesh_renderer(
        &mut scene.godot_dcl_scene,
        crdt_state,
        _dirty_entities,
        dirty_components,
    );
}
