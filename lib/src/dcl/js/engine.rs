use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
};

use deno_core::{op2, OpDecl, OpState};
use tokio::sync::mpsc::Receiver;

use crate::dcl::{
    common::{
        CommunicatedWithRenderer, SceneDying, SceneElapsedTime, SceneLogs, SceneMainCrdtFileContent,
    },
    crdt::{
        message::{append_gos_component, process_many_messages, put_or_delete_lww_component},
        SceneCrdtState,
    },
    scene_apis::{LocalCall, RpcCall},
    serialization::{reader::DclReader, writer::DclWriter},
    RendererResponse, SceneId, SceneResponse, SharedSceneCrdtState,
};

use super::{
    comms::{InternalPendingBinaryMessages, COMMS_MSG_TYPE_BINARY},
    events::process_events,
    players::{get_player_data, get_players},
};

// list of op declarations
pub fn ops() -> Vec<OpDecl> {
    vec![op_crdt_send_to_renderer(), op_crdt_recv_from_renderer()]
}

// receive and process a buffer of crdt messages
#[op2(fast)]
fn op_crdt_send_to_renderer(op_state: Rc<RefCell<OpState>>, #[arraybuffer] messages: &[u8]) {
    let mut op_state = op_state.borrow_mut();
    let elapsed_time = op_state.borrow::<SceneElapsedTime>().0;
    let scene_id = op_state.take::<SceneId>();

    let logs = op_state.take::<SceneLogs>();
    op_state.put(SceneLogs(Vec::new()));

    let mutex_scene_crdt_state = op_state.take::<SharedSceneCrdtState>();
    let cloned_scene_crdt = mutex_scene_crdt_state.clone();
    let mut scene_crdt_state = cloned_scene_crdt.lock().unwrap();

    let mut stream = DclReader::new(messages);
    process_many_messages(&mut stream, &mut scene_crdt_state);

    let dirty = scene_crdt_state.take_dirty();

    // This drop unlock the mutex
    drop(scene_crdt_state);
    drop(cloned_scene_crdt);

    op_state.put(mutex_scene_crdt_state);
    op_state.put(scene_id);

    let rpc_calls = std::mem::take(op_state.borrow_mut::<Vec<RpcCall>>());

    // Get the latest Deno memory stats
    let deno_memory_stats = op_state
        .try_borrow::<super::super::DenoMemoryStats>()
        .copied();

    let sender = op_state.borrow_mut::<std::sync::mpsc::SyncSender<SceneResponse>>();

    sender
        .send(SceneResponse::Ok {
            scene_id,
            dirty_crdt_state: Box::new(dirty),
            logs: logs.0,
            delta: elapsed_time,
            rpc_calls,
            deno_memory_stats,
        })
        .expect("error sending scene response!!")
}

#[op2(async)]
#[serde]
async fn op_crdt_recv_from_renderer(
    op_state: Rc<RefCell<OpState>>,
) -> Result<Vec<Vec<u8>>, anyhow::Error> {
    let receiver = op_state
        .borrow_mut()
        .borrow_mut::<Arc<tokio::sync::Mutex<Receiver<RendererResponse>>>>()
        .clone();
    let response = receiver.lock().await.recv().await;

    let mut op_state = op_state.borrow_mut();
    op_state.put(receiver);

    let local_api_calls = op_state.take::<Vec<LocalCall>>();
    let mutex_scene_crdt_state = op_state.take::<Arc<Mutex<SceneCrdtState>>>();
    let cloned_scene_crdt = mutex_scene_crdt_state.clone();
    let scene_crdt_state = cloned_scene_crdt.lock().unwrap();

    let data = match response {
        Some(RendererResponse::Ok {
            dirty_crdt_state,
            incoming_comms_message,
        }) => {
            let mut data_buf = Vec::new();
            let mut data_writter = DclWriter::new(&mut data_buf);

            if !dirty_crdt_state.entities.died.is_empty() {
                tracing::info!(
                    "recv_from_renderer: {} entities died, {} born, {} lww dirty components, {} gos dirty components",
                    dirty_crdt_state.entities.died.len(),
                    dirty_crdt_state.entities.born.len(),
                    dirty_crdt_state.lww.len(),
                    dirty_crdt_state.gos.len(),
                );
                for entity_id in dirty_crdt_state.entities.died.iter() {
                    tracing::debug!("  died entity: {:?}", entity_id);
                }
            }

            let mut skipped_lww = 0u32;
            let mut skipped_gos = 0u32;

            // Skip component updates for entities that died — the renderer handles
            // entity lifecycle on its own side. Sending component deletions for dead
            // entities would corrupt the JS SDK's syncEntity state.
            // This matches bevy-explorer, which never sends entity deaths or their
            // component deletions back to JS.
            for (component_id, entities) in dirty_crdt_state.lww.iter() {
                for entity_id in entities {
                    if dirty_crdt_state.entities.died.contains(entity_id) {
                        skipped_lww += 1;
                        continue;
                    }
                    if let Err(err) = put_or_delete_lww_component(
                        &scene_crdt_state,
                        entity_id,
                        component_id,
                        &mut data_writter,
                    ) {
                        tracing::error!("error writing crdt message: {err}");
                    }
                }
            }

            for (component_id, entities) in dirty_crdt_state.gos.iter() {
                for (entity_id, element_count) in entities {
                    if dirty_crdt_state.entities.died.contains(entity_id) {
                        skipped_gos += 1;
                        continue;
                    }
                    if let Err(err) = append_gos_component(
                        &scene_crdt_state,
                        entity_id,
                        component_id,
                        element_count,
                        &mut data_writter,
                    ) {
                        tracing::error!("error writing crdt message: {err}");
                    }
                }
            }

            if skipped_lww > 0 || skipped_gos > 0 {
                tracing::debug!(
                    "recv_from_renderer: skipped {} lww + {} gos updates for dead entities",
                    skipped_lww,
                    skipped_gos,
                );
            }

            let (comms_binary, comms_string): (_, Vec<_>) = incoming_comms_message
                .into_iter()
                .filter(|v| !v.1.is_empty())
                .partition(|v| v.1[0] == COMMS_MSG_TYPE_BINARY);

            if !comms_binary.is_empty() || !comms_string.is_empty() {
                tracing::debug!(
                    "🔀 comms partition: {} binary, {} string. First bytes: {:?}",
                    comms_binary.len(),
                    comms_string.len(),
                    comms_binary
                        .iter()
                        .chain(comms_string.iter())
                        .map(|(addr, data)| format!("{:#x}:byte[0]={}", addr, data[0]))
                        .collect::<Vec<_>>()
                );
            }

            if !comms_binary.is_empty() {
                let mut internal_pending_binary_messages = op_state
                    .try_take::<InternalPendingBinaryMessages>()
                    .unwrap_or_default();

                internal_pending_binary_messages
                    .messages
                    .extend(comms_binary.into_iter());
                op_state.put(internal_pending_binary_messages);
            }

            process_local_api_calls(local_api_calls, &scene_crdt_state);
            process_events(
                &mut op_state,
                &scene_crdt_state,
                &dirty_crdt_state,
                comms_string,
            );

            data_buf
        }
        _ => {
            // channel has been closed, shutdown gracefully
            tracing::debug!("{}: shutting down", std::thread::current().name().unwrap());

            // TODO: handle recv from renderer
            op_state.put(SceneDying(true));

            Default::default()
        }
    };

    op_state.put(CommunicatedWithRenderer);

    op_state.put(Vec::<LocalCall>::new());
    op_state.put(mutex_scene_crdt_state);
    let mut ret = Vec::<Vec<u8>>::with_capacity(1);
    if let Some(main_crdt) = op_state.try_take::<SceneMainCrdtFileContent>() {
        ret.push(main_crdt.0);
    }
    ret.push(data);
    Ok(ret)
}

fn process_local_api_calls(local_api_calls: Vec<LocalCall>, crdt_state: &SceneCrdtState) {
    for local_call in local_api_calls {
        match local_call {
            LocalCall::PlayersGetPlayerData { user_id, response } => {
                response.send(get_player_data(user_id, crdt_state));
            }
            LocalCall::PlayersGetPlayersInScene { response } => {
                response.send(get_players(crdt_state, true));
            }
            LocalCall::PlayersGetConnectedPlayers { response } => {
                response.send(get_players(crdt_state, false));
            }
        }
    }
}
