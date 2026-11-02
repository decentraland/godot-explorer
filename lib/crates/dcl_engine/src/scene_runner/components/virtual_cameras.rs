use crate::{
    dcl::{
        components::{SceneComponentId, SceneEntityId},
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
    },
    scene_runner::scene::Scene,
};

pub fn update_main_and_virtual_cameras(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let dirty_lww_components = &scene.current_dirty.lww_components;

    // if there is no new information about main or virtual camera, this functions has nothing to do...
    if !dirty_lww_components.contains_key(&SceneComponentId::MAIN_CAMERA)
        && !dirty_lww_components.contains_key(&SceneComponentId::VIRTUAL_CAMERA)
    {
        return;
    }

    // Update main camera
    let virtual_camera_targeted = {
        let main_camera_component = SceneCrdtStateProtoComponents::get_main_camera(crdt_state);
        main_camera_component
            .get(&SceneEntityId::CAMERA)
            .and_then(|x| x.value.as_ref())
            .and_then(|x| x.virtual_camera_entity)
            .map(|v| SceneEntityId::from_i32(v as i32))
    };

    match virtual_camera_targeted {
        None => {
            // update scene is not using a virtual camera
            scene.virtual_camera.bind_mut().clear();
        }

        Some(virtual_camera_entity_id) => {
            let virtual_cameras = SceneCrdtStateProtoComponents::get_virtual_camera(crdt_state);

            match virtual_cameras
                .get(&virtual_camera_entity_id)
                .and_then(|x| x.value.as_ref())
            {
                None => {
                    scene
                        .virtual_camera
                        .bind_mut()
                        .set_transform(&virtual_camera_entity_id);
                }
                Some(virtual_camera_value) => {
                    scene
                        .virtual_camera
                        .bind_mut()
                        .set_virtual_camera(&virtual_camera_entity_id, virtual_camera_value);
                }
            }
        }
    }
}
