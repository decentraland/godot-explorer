use crate::{
    dcl::{
        components::SceneEntityId,
        crdt::{SceneCrdtState, SceneCrdtStateProtoComponents},
    },
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

            // When target_entity is set (and isn't the camera reserved entity), the
            // billboard faces that entity instead of the camera. If the target doesn't
            // exist (yet, or anymore), reorientation is disabled until it does. The
            // CRDT liveness check matters besides the node lookup: deleting an entity
            // marks its Transform dirty, and the transform update recreates a ghost
            // node at the origin for it — a dead target must not face that ghost.
            let target_position = match billboard.target_entity {
                None => camera_position,
                Some(raw) => {
                    let target_id = SceneEntityId::from_i32(raw as i32);
                    if target_id == SceneEntityId::CAMERA {
                        camera_position
                    } else if crdt_state.entities.is_dead(&target_id) {
                        continue;
                    } else if let Some(target_node) =
                        scene.godot_dcl_scene.get_node_or_null_3d(&target_id)
                    {
                        target_node.get_global_position()
                    } else {
                        continue;
                    }
                }
            };

            let (_, mut node_3d) = scene.godot_dcl_scene.ensure_node_3d(entity);
            let original_scale = node_3d.get_scale();

            let direction = node_3d.get_global_position() - target_position;
            // Skip degenerate directions: zero length (e.g. an entity targeting
            // itself) or (near-)collinear with UP (target directly above/below) —
            // looking_at would error out, and for Y mode the yaw is undefined.
            let length_squared = direction.length_squared();
            let horizontal_squared = direction.x * direction.x + direction.z * direction.z;
            if length_squared < 1e-10 || horizontal_squared < length_squared * 1e-8 {
                continue;
            }

            match billboard_mode {
                Billboard::None => {}
                Billboard::Y => {
                    let origin = node_3d.get_global_position();
                    let basis = Basis::looking_at(direction, Vector3::UP, false);

                    let mut euler_vector = basis.get_euler();
                    euler_vector.z = 0.0;
                    euler_vector.x = 0.0;
                    let basis = Basis::from_euler(EulerOrder::YXZ, euler_vector);

                    node_3d.set_global_transform(Transform3D { basis, origin });
                }
                // TODO: we do not distinguish between YX and All for now
                Billboard::All | Billboard::YX => {
                    let origin = node_3d.get_global_position();
                    let basis = Basis::looking_at(direction, Vector3::UP, false);
                    node_3d.set_global_transform(Transform3D { basis, origin });
                }
            }
            node_3d.set_scale(original_scale);
        }
    }
}

mod test {
    use godot::classes::Node;
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
            .add_child(&scene.godot_dcl_scene.root_node_3d.clone().upcast::<Node>());

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
            .add_child(&scene.godot_dcl_scene.root_node_3d.clone().upcast::<Node>());

        let camera_global_transform =
            Transform3D::new(Basis::IDENTITY, Vector3::new(1.0, 0.0, 1.0));

        let entity = SceneEntityId::new(1333, 0);
        scene.godot_dcl_scene.ensure_node_3d(&entity);
        SceneCrdtStateProtoComponents::get_billboard_mut(&mut crdt_state).put(
            entity,
            // Whether `..Default::default()` is needed depends on the pinned @dcl/protocol:
            // newer builds add optional PBBillboard fields (e.g. target_entity), older ones
            // don't. Keep it so protocol bumps can't break this test again.
            #[allow(clippy::needless_update)]
            Some(PbBillboard {
                billboard_mode: Some(3),
                ..Default::default()
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

    #[godot::test::itest]
    fn test_billboard_target_entity(scene_context: &TestContext) {
        let mut scene = Scene::unsafe_default();
        let crdt = scene.dcl_scene.scene_crdt.clone();
        let mut crdt_state = crdt.try_lock().unwrap();
        scene_context
            .scene_tree
            .clone()
            .add_child(&scene.godot_dcl_scene.root_node_3d.clone().upcast::<Node>());

        // Camera placed where it would produce a different rotation than the target.
        let camera_global_transform =
            Transform3D::new(Basis::IDENTITY, Vector3::new(-5.0, 0.0, 0.0));

        let target = SceneEntityId::new(1500, 0);
        let (_, mut target_node) = scene.godot_dcl_scene.ensure_node_3d(&target);
        target_node.set_global_position(Vector3::new(1.0, 0.0, 1.0));

        let entity = SceneEntityId::new(1333, 0);
        scene.godot_dcl_scene.ensure_node_3d(&entity);
        SceneCrdtStateProtoComponents::get_billboard_mut(&mut crdt_state).put(
            entity,
            // Same rationale as in test_billboard: survive protocol bumps that
            // add optional PBBillboard fields.
            #[allow(clippy::needless_update)]
            Some(PbBillboard {
                billboard_mode: Some(3),
                target_entity: Some(target.as_i32() as u32),
                ..Default::default()
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

    #[godot::test::itest]
    fn test_billboard_target_entity_missing(scene_context: &TestContext) {
        let mut scene = Scene::unsafe_default();
        let crdt = scene.dcl_scene.scene_crdt.clone();
        let mut crdt_state = crdt.try_lock().unwrap();
        scene_context
            .scene_tree
            .clone()
            .add_child(&scene.godot_dcl_scene.root_node_3d.clone().upcast::<Node>());

        let camera_global_transform =
            Transform3D::new(Basis::IDENTITY, Vector3::new(1.0, 0.0, 1.0));

        let entity = SceneEntityId::new(1333, 0);
        let (_, mut node_3d) = scene.godot_dcl_scene.ensure_node_3d(&entity);
        let initial_rotation = Vector3::new(0.0, 1.0, 0.0);
        node_3d.set_global_rotation(initial_rotation);

        SceneCrdtStateProtoComponents::get_billboard_mut(&mut crdt_state).put(
            entity,
            #[allow(clippy::needless_update)]
            Some(PbBillboard {
                billboard_mode: Some(3),
                // this entity was never created: reorientation must be disabled
                target_entity: Some(SceneEntityId::new(4444, 0).as_i32() as u32),
                ..Default::default()
            }),
        );

        update_billboard(&mut scene, &mut crdt_state, &camera_global_transform);

        let node = scene.godot_dcl_scene.get_node_or_null_3d(&entity).unwrap();
        assert_eq!(node.get_global_rotation(), initial_rotation);
    }

    #[godot::test::itest]
    fn test_billboard_target_entity_dead(scene_context: &TestContext) {
        let mut scene = Scene::unsafe_default();
        let crdt = scene.dcl_scene.scene_crdt.clone();
        let mut crdt_state = crdt.try_lock().unwrap();
        scene_context
            .scene_tree
            .clone()
            .add_child(&scene.godot_dcl_scene.root_node_3d.clone().upcast::<Node>());

        let camera_global_transform =
            Transform3D::new(Basis::IDENTITY, Vector3::new(1.0, 0.0, 1.0));

        // The target has a (ghost) node in the tree — deleting an entity marks its
        // Transform dirty and the transform update re-creates a node at the origin —
        // but the entity is dead in the CRDT, so the billboard must stay frozen.
        let target = SceneEntityId::new(1500, 0);
        scene.godot_dcl_scene.ensure_node_3d(&target);
        crdt_state.entities.kill(target);

        let entity = SceneEntityId::new(1333, 0);
        let (_, mut node_3d) = scene.godot_dcl_scene.ensure_node_3d(&entity);
        node_3d.set_global_position(Vector3::new(4.0, 2.0, 8.0));
        let initial_rotation = Vector3::new(0.0, 1.0, 0.0);
        node_3d.set_global_rotation(initial_rotation);

        SceneCrdtStateProtoComponents::get_billboard_mut(&mut crdt_state).put(
            entity,
            #[allow(clippy::needless_update)]
            Some(PbBillboard {
                billboard_mode: Some(3),
                target_entity: Some(target.as_i32() as u32),
                ..Default::default()
            }),
        );

        update_billboard(&mut scene, &mut crdt_state, &camera_global_transform);

        let node = scene.godot_dcl_scene.get_node_or_null_3d(&entity).unwrap();
        assert_eq!(node.get_global_rotation(), initial_rotation);
    }

    #[godot::test::itest]
    fn test_billboard_target_entity_overhead(scene_context: &TestContext) {
        let mut scene = Scene::unsafe_default();
        let crdt = scene.dcl_scene.scene_crdt.clone();
        let mut crdt_state = crdt.try_lock().unwrap();
        scene_context
            .scene_tree
            .clone()
            .add_child(&scene.godot_dcl_scene.root_node_3d.clone().upcast::<Node>());

        let camera_global_transform =
            Transform3D::new(Basis::IDENTITY, Vector3::new(1.0, 0.0, 1.0));

        // Target exactly above the billboard: the look direction is collinear with
        // UP, so reorientation must be skipped (no errors, rotation unchanged).
        let target = SceneEntityId::new(1500, 0);
        let (_, mut target_node) = scene.godot_dcl_scene.ensure_node_3d(&target);
        target_node.set_global_position(Vector3::new(0.0, 5.0, 0.0));

        let entity = SceneEntityId::new(1333, 0);
        let (_, mut node_3d) = scene.godot_dcl_scene.ensure_node_3d(&entity);
        let initial_rotation = Vector3::new(0.0, 1.0, 0.0);
        node_3d.set_global_rotation(initial_rotation);

        SceneCrdtStateProtoComponents::get_billboard_mut(&mut crdt_state).put(
            entity,
            #[allow(clippy::needless_update)]
            Some(PbBillboard {
                billboard_mode: Some(3),
                target_entity: Some(target.as_i32() as u32),
                ..Default::default()
            }),
        );

        update_billboard(&mut scene, &mut crdt_state, &camera_global_transform);

        let node = scene.godot_dcl_scene.get_node_or_null_3d(&entity).unwrap();
        assert_eq!(node.get_global_rotation(), initial_rotation);
    }

    #[godot::test::itest]
    fn test_billboard_target_entity_camera(scene_context: &TestContext) {
        let mut scene = Scene::unsafe_default();
        let crdt = scene.dcl_scene.scene_crdt.clone();
        let mut crdt_state = crdt.try_lock().unwrap();
        scene_context
            .scene_tree
            .clone()
            .add_child(&scene.godot_dcl_scene.root_node_3d.clone().upcast::<Node>());

        let camera_global_transform =
            Transform3D::new(Basis::IDENTITY, Vector3::new(1.0, 0.0, 1.0));

        let entity = SceneEntityId::new(1333, 0);
        scene.godot_dcl_scene.ensure_node_3d(&entity);
        SceneCrdtStateProtoComponents::get_billboard_mut(&mut crdt_state).put(
            entity,
            #[allow(clippy::needless_update)]
            Some(PbBillboard {
                billboard_mode: Some(3),
                // targeting the camera reserved entity is the same as no target
                target_entity: Some(SceneEntityId::CAMERA.as_i32() as u32),
                ..Default::default()
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
