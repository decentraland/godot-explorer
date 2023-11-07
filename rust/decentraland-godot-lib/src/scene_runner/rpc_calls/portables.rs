use crate::{
    common::rpc::{PortableLocation, RpcResultSender, SpawnResponse},
    godot_classes::dcl_global::DclGlobal,
    scene_runner::scene::Scene,
};

pub fn spawn_portable(
    scene: &Scene,
    location: PortableLocation,
    response: RpcResultSender<Result<SpawnResponse, String>>,
) {
    let mut portable_experience_controller = DclGlobal::singleton()
        .bind_mut()
        .portable_experience_controller
        .clone();

    portable_experience_controller.bind_mut().spawn(
        location,
        response,
        &scene.definition.entity_id,
        false,
    );
}

pub fn kill_portable(location: PortableLocation, response: RpcResultSender<bool>) {
    let mut portable_experience_controller = DclGlobal::singleton()
        .bind()
        .portable_experience_controller
        .clone();

    portable_experience_controller
        .bind_mut()
        .kill(location, response.clone());
}

pub fn list_portables(response: RpcResultSender<Vec<SpawnResponse>>) {
    let portable_experience_controller = DclGlobal::singleton()
        .bind()
        .portable_experience_controller
        .clone();

    let pe_list = portable_experience_controller
        .bind()
        .get_running_portable_experience_list();

    response.send(pe_list);
}
