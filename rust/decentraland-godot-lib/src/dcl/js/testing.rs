use deno_core::{
    anyhow::{self, anyhow},
    error::AnyError,
    op, Op, OpDecl, OpState,
};
use godot::builtin::{Vector2, Vector3};

use crate::dcl::{SceneResponse, TakeAndCompareSnapshotResponse};

use super::SceneEnv;

pub fn ops() -> Vec<OpDecl> {
    vec![op_take_and_compare_snapshot::DECL]
}

#[op]
fn op_take_and_compare_snapshot(
    state: &mut OpState,
    id: String,
    camera_position: [f32; 3],
    camera_target: [f32; 3],
    snapshot_frame_size: [f32; 2],
    tolerance: f32,
) -> Result<TakeAndCompareSnapshotResponse, AnyError> {
    let scene_env = state.borrow::<SceneEnv>();
    if !scene_env.testing_enable {
        return Err(anyhow::anyhow!("Testing mode not available"));
    }

    let (sx, rx) =
        tokio::sync::oneshot::channel::<Result<TakeAndCompareSnapshotResponse, String>>();

    let sender = state.borrow_mut::<std::sync::mpsc::SyncSender<SceneResponse>>();
    sender
        .send(SceneResponse::TakeSnapshot {
            id,
            camera_position: Vector3 {
                x: camera_position[0],
                y: camera_position[1],
                z: camera_position[2],
            },
            camera_target: Vector3 {
                x: camera_target[0],
                y: camera_target[1],
                z: camera_target[2],
            },
            snapshot_frame_size: Vector2 {
                x: snapshot_frame_size[0],
                y: snapshot_frame_size[1],
            },
            tolerance,
            response: sx.into(),
        })
        .expect("error sending scene response!!");

    rx.blocking_recv()
        .map_err(|e| anyhow::anyhow!(e))?
        .map_err(|e| anyhow!(e))
}
