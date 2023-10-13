use crate::{
    dcl::{
        components::{proto_components, SceneComponentId},
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
    },
    godot_classes::dcl_camera_mode_area_3d::DclCameraModeArea3D,
    scene_runner::scene::Scene,
};
use godot::prelude::*;

pub fn update_camera_mode_area(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let dirty_lww_components = &scene.current_dirty.lww_components;
    let camera_mode_area_component =
        SceneCrdtStateProtoComponents::get_camera_mode_area(crdt_state);

    if let Some(camera_mode_area_dirty) =
        dirty_lww_components.get(&SceneComponentId::CAMERA_MODE_AREA)
    {
        for entity in camera_mode_area_dirty {
            let new_value = camera_mode_area_component.get(*entity);

            let Some(new_value) = new_value else {
                continue; // no value, continue
            };

            let node = godot_dcl_scene.ensure_node_mut(entity);

            let new_value = new_value.value.clone();

            let existing = node
                .base
                .try_get_node_as::<Node>(NodePath::from("DCLCameraModeArea3D"));

            if new_value.is_none() {
                if let Some(camera_mode_area_node) = existing {
                    node.base.remove_child(camera_mode_area_node);
                }
            } else if let Some(new_value) = new_value {
                let area = new_value
                    .area
                    .unwrap_or(proto_components::common::Vector3::default());
                let forced_camera_mode = new_value.mode;

                if let Some(camera_mode_area_node) = existing {
                    let mut camera_mode_area_3d =
                        camera_mode_area_node.cast::<DclCameraModeArea3D>();

                    camera_mode_area_3d
                        .bind_mut()
                        .set_area(Vector3::new(area.x, area.y, area.z));
                    camera_mode_area_3d
                        .bind_mut()
                        .set_forced_camera_mode(forced_camera_mode);
                } else {
                    let mut camera_mode_area_3d = godot::engine::load::<PackedScene>(
                        "res://src/decentraland_components/camera_mode_area.tscn",
                    )
                    .instantiate()
                    .unwrap()
                    .cast::<DclCameraModeArea3D>();

                    camera_mode_area_3d
                        .bind_mut()
                        .set_area(Vector3::new(area.x, area.y, area.z));
                    camera_mode_area_3d
                        .bind_mut()
                        .set_forced_camera_mode(forced_camera_mode);
                    camera_mode_area_3d.set_name(GodotString::from("DCLCameraModeArea3D"));
                    node.base.add_child(camera_mode_area_3d.clone().upcast());
                }
            }
        }
    }
}
