use deno_core::{anyhow::anyhow, error::AnyError, op, Op, OpDecl, OpState};
use godot::builtin::{Vector2, Vector3};

use crate::dcl::{
    common::{
        SceneTestPlan, SceneTestResult, TakeAndCompareSnapshotResponse,
        TestingScreenshotComparisonMethodRequest,
    },
    scene_apis::RpcCall,
    SceneId, SceneResponse,
};

use super::SceneEnv;

pub fn ops() -> Vec<OpDecl> {
    vec![
        op_take_and_compare_snapshot::DECL,
        op_log_test_result::DECL,
        op_log_test_plan::DECL,
    ]
}

#[op]
fn op_take_and_compare_snapshot(
    state: &mut OpState,
    src_stored_snapshot: String,
    camera_position: [f32; 3],
    camera_target: [f32; 3],
    screeshot_size: [f32; 2],
    method: TestingScreenshotComparisonMethodRequest,
) -> Result<TakeAndCompareSnapshotResponse, AnyError> {
    let scene_env = state.borrow::<SceneEnv>();
    if !scene_env.testing_enable {
        return Err(anyhow::anyhow!("Testing mode not available"));
    }

    let (sx, mut rx) =
        tokio::sync::oneshot::channel::<Result<TakeAndCompareSnapshotResponse, String>>();

    let scene_id = *state.borrow::<SceneId>();
    let sender = state.borrow_mut::<std::sync::mpsc::SyncSender<SceneResponse>>();
    sender
        .send(SceneResponse::TakeSnapshot {
            scene_id,
            src_stored_snapshot,
            camera_position: Vector3 {
                x: camera_position[0],
                y: camera_position[1],
                z: -camera_position[2],
            },
            camera_target: Vector3 {
                x: camera_target[0],
                y: camera_target[1],
                z: -camera_target[2],
            },
            screeshot_size: Vector2 {
                x: screeshot_size[0],
                y: screeshot_size[1],
            },
            method,
            response: sx.into(),
        })
        .expect("error sending scene response!!");

    // TODO: This is a workaround to wait for the response to be ready
    //      block_on is not available for runtimes
    let response = {
        let mut value;
        loop {
            value = match rx.try_recv() {
                Ok(value) => Some(Ok(value)),
                Err(e) => match e {
                    tokio::sync::oneshot::error::TryRecvError::Empty => None,
                    tokio::sync::oneshot::error::TryRecvError::Closed => {
                        Some(Err(anyhow::anyhow!("Scene response channel closed")))
                    }
                },
            };
            if value.is_none() {
                std::thread::sleep(std::time::Duration::from_millis(100));
            } else {
                break;
            }
        }
        value.unwrap()
    };

    response
        .map_err(|e| anyhow::anyhow!(e))?
        .map_err(|e| anyhow!(e))
}

#[op]
fn op_log_test_result(state: &mut OpState, body: SceneTestResult) {
    state
        .borrow_mut::<Vec<RpcCall>>()
        .push(RpcCall::SceneTestResult { body });
}

#[op]
fn op_log_test_plan(state: &mut OpState, body: SceneTestPlan) {
    state
        .borrow_mut::<Vec<RpcCall>>()
        .push(RpcCall::SceneTestPlan { body });
}
