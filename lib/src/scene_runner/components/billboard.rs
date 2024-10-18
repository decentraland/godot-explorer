use crate::{
    dcl::crdt::{SceneCrdtState, SceneCrdtStateProtoComponents},
    scene_runner::scene::Scene,
};
use godot::prelude::*;

pub enum Billboard {
    None,
    Y,
    YX,
    All,
}

impl From<Option<i32>> for Billboard {
    fn from(value: Option<i32>) -> Self {
        match value {
            Some(0) => Billboard::None,
            Some(2) => Billboard::Y,
            Some(3) => Billboard::YX,
            _ => Billboard::All,
        }
    }
}

pub fn update_billboard(
    scene: &mut Scene,
    crdt_state: &mut SceneCrdtState,
    camera_global_transform: &Transform3D,
) {
    let billboard_component = SceneCrdtStateProtoComponents::get_billboard(crdt_state);
    let camera_position = camera_global_transform.origin;

    for (entity, entry) in billboard_component.values.iter() {
        if let Some(billboard) = entry.value.as_ref() {
            let billboard_mode = Billboard::from(billboard.billboard_mode);
            if let Billboard::None = billboard_mode {
                continue;
            }

            let (_, mut node_3d) = scene.godot_dcl_scene.ensure_node_3d(entity);
            let original_scale = node_3d.get_scale();

            match Billboard::from(billboard.billboard_mode) {
                Billboard::None => {}
                Billboard::Y => {
                    let origin = node_3d.get_global_position();
                    let direction = node_3d.get_global_position() - camera_position;
                    let basis = Basis::new_looking_at(direction, Vector3::UP, false);

                    let mut euler_vector = basis.to_euler(EulerOrder::YXZ);
                    euler_vector.z = 0.0;
                    euler_vector.x = 0.0;
                    let basis = Basis::from_euler(EulerOrder::YXZ, euler_vector);

                    node_3d.set_global_transform(Transform3D { basis, origin });
                }
                // TODO: we do not distinguish between YX and All for now
                Billboard::All | Billboard::YX => {
                    let origin = node_3d.get_global_position();
                    let direction = node_3d.get_global_position() - camera_position;
                    let basis = Basis::new_looking_at(direction, Vector3::UP, false);
                    node_3d.set_global_transform(Transform3D { basis, origin });
                }
            }
            node_3d.set_scale(original_scale);
        }
    }
}

mod test {
    use godot::prelude::{Basis, Transform3D, Vector3};

    use crate::{
        dcl::{
            components::{proto_components::sdk::components::PbBillboard, SceneEntityId},
            crdt::{
                last_write_wins::LastWriteWinsComponentOperation, SceneCrdtStateProtoComponents,
            },
        },
        framework::TestContext,
        scene_runner::scene::Scene,
    };

    use super::update_billboard;

    #[godot::test::itest]
    fn test_billboard_empty(scene_context: &TestContext) {
        let mut scene = Scene::unsafe_default();
        let crdt = scene.dcl_scene.scene_crdt.clone();
        let mut crdt_state = crdt.try_lock().unwrap();
        scene_context
            .scene_tree
            .clone()
            .add_child(scene.godot_dcl_scene.root_node_3d.clone().upcast());

        let camera_global_transform = Transform3D::IDENTITY;
        update_billboard(&mut scene, &mut crdt_state, &camera_global_transform);
    }

    #[godot::test::itest]
    fn test_billboard(scene_context: &TestContext) {
        let mut scene = Scene::unsafe_default();
        let crdt = scene.dcl_scene.scene_crdt.clone();
        let mut crdt_state = crdt.try_lock().unwrap();
        scene_context
            .scene_tree
            .clone()
            .add_child(scene.godot_dcl_scene.root_node_3d.clone().upcast());

        let camera_global_transform =
            Transform3D::new(Basis::IDENTITY, Vector3::new(1.0, 0.0, 1.0));

        let entity = SceneEntityId::new(1333, 0);
        scene.godot_dcl_scene.ensure_node_3d(&entity);
        SceneCrdtStateProtoComponents::get_billboard_mut(&mut crdt_state).put(
            entity,
            Some(PbBillboard {
                billboard_mode: Some(3),
            }),
        );

        update_billboard(&mut scene, &mut crdt_state, &camera_global_transform);

        let node = scene.godot_dcl_scene.get_node_or_null_3d(&entity).unwrap();
        assert_eq!(
            node.get_global_rotation(),
            Vector3 {
                x: 0.0,
                y: std::f32::consts::FRAC_PI_4,
                z: 0.0
            }
        );
    }
}
