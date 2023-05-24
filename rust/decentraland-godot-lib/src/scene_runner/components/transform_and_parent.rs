use godot::prelude::EulerOrder;

use crate::{
    dcl::{
        components::SceneComponentId,
        crdt::{last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState},
        DirtyComponents, DirtyEntities,
    },
    scene_runner::GodotDclScene,
};

pub fn update_transform_and_parent(
    godot_dcl_scene: &mut GodotDclScene,
    crdt_state: &mut SceneCrdtState,
    _dirty_entities: &DirtyEntities,
    dirty_components: &DirtyComponents,
) {
    let transform_component = crdt_state.get_transform();

    if let Some(dirty_transform) = dirty_components.get(&SceneComponentId::TRANSFORM) {
        for entity in dirty_transform {
            let value = transform_component.get(*entity);
            let node = godot_dcl_scene.ensure_node_mut(entity);
            if let Some(entry) = value {
                if let Some(transform) = entry.value.clone() {
                    node.base
                        .set_rotation(transform.rotation.to_euler(EulerOrder::XYZ));
                    node.base.set_position(transform.translation);
                    node.base.set_scale(transform.scale);
                }
            }
        }
    }
}

// #[itest]
// fn cyclic() {
//     // Mock all params needed to run update_transform_and_parent
//     let mut scene = GodotDclScene {
//         definition: SceneDefinition {
//             offset: Vector3::new(0.0, 0.0, 0.0),
//             ..Default::default()
//         },
//         ..Default::default()
//     };
//     let mut crdt_state = SceneCrdtState::new();
//     let dirty_entities = DirtyEntities {
//         born: HashSet::new(),
//         died: HashSet::new(),
//     };
//     let mut dirty_components = DirtyComponents::new();
// }
