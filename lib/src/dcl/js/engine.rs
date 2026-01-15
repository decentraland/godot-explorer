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
        message::{append_gos_component, delete_entity, put_or_delete_lww_component},
        SceneCrdtState,
    },
    scene_apis::{LocalCall, RpcCall},
    serialization::{reader::DclReader, writer::DclWriter},
    RendererResponse, SceneId, SceneResponse, SharedSceneCrdtState,
};

#[cfg(not(feature = "scene_logging"))]
use crate::dcl::crdt::message::process_many_messages;

#[cfg(feature = "scene_logging")]
use crate::dcl::crdt::CrdtLoggingContext;

/// Tick counter for scene logging (increments each time CRDT messages are processed)
#[cfg(feature = "scene_logging")]
pub struct SceneTickCounter(pub std::sync::atomic::AtomicU32);

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

    #[cfg(feature = "scene_logging")]
    {
        use crate::dcl::crdt::message::process_many_messages_with_logging;
        use crate::tools::scene_logging::{get_logger_sender, CrdtDirection};

        let logging_ctx = get_logger_sender().map(|sender| {
            // Get or create tick counter
            let tick = op_state
                .try_borrow::<SceneTickCounter>()
                .map(|tc| tc.0.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
                .unwrap_or(0);

            CrdtLoggingContext::new(sender, tick, CrdtDirection::SceneToRenderer)
        });

        process_many_messages_with_logging(&mut stream, &mut scene_crdt_state, logging_ctx.as_ref());
    }

    #[cfg(not(feature = "scene_logging"))]
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

            for (component_id, entities) in dirty_crdt_state.lww.iter() {
                for entity_id in entities {
                    // Log renderer->scene CRDT operation
                    #[cfg(feature = "scene_logging")]
                    {
                        use crate::dcl::serialization::writer::DclWriter;
                        use crate::tools::scene_logging::{log_crdt_renderer_to_scene, CrdtOperation};

                        if let Some(comp_def) =
                            scene_crdt_state.get_lww_component_definition(*component_id)
                        {
                            if let Some(opaque) = comp_def.get_opaque(*entity_id) {
                                let operation = if opaque.value.is_some() {
                                    CrdtOperation::Put
                                } else {
                                    CrdtOperation::Delete
                                };

                                // Get binary payload data for serialization
                                let payload_data = if opaque.value.is_some() {
                                    let mut payload_buf = Vec::new();
                                    let mut payload_writer = DclWriter::new(&mut payload_buf);
                                    if comp_def.to_binary(*entity_id, &mut payload_writer).is_ok() {
                                        Some(payload_buf)
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                };

                                log_crdt_renderer_to_scene(
                                    0, // tick not available here
                                    entity_id.as_i32() as u32,
                                    component_id.0,
                                    operation,
                                    opaque.timestamp.0,
                                    payload_data.as_deref(),
                                );
                            }
                        }
                    }

                    if let Err(err) = put_or_delete_lww_component(
                        &scene_crdt_state,
                        entity_id,
                        component_id,
                        &mut data_writter,
                    ) {
                        tracing::info!("error writing crdt message: {err}");
                    }
                }
            }

            for (component_id, entities) in dirty_crdt_state.gos.iter() {
                for (entity_id, element_count) in entities {
                    // Log renderer->scene GOS append operation
                    #[cfg(feature = "scene_logging")]
                    {
                        use crate::tools::scene_logging::{log_crdt_renderer_to_scene, CrdtOperation};

                        log_crdt_renderer_to_scene(
                            0, // tick not available here
                            entity_id.as_i32() as u32,
                            component_id.0,
                            CrdtOperation::Append,
                            0, // GOS doesn't have timestamp
                            None,
                        );
                    }

                    if let Err(err) = append_gos_component(
                        &scene_crdt_state,
                        entity_id,
                        component_id,
                        element_count,
                        &mut data_writter,
                    ) {
                        tracing::info!("error writing crdt message: {err}");
                    }
                }
            }

            for entity_id in dirty_crdt_state.entities.died.iter() {
                // Log renderer->scene entity delete operation
                #[cfg(feature = "scene_logging")]
                {
                    use crate::tools::scene_logging::{log_crdt_renderer_to_scene, CrdtOperation};

                    log_crdt_renderer_to_scene(
                        0, // tick not available here
                        entity_id.as_i32() as u32,
                        0, // no component for entity delete
                        CrdtOperation::DeleteEntity,
                        0,
                        None,
                    );
                }

                delete_entity(entity_id, &mut data_writter);
            }

            let (comms_binary, comms_string): (_, Vec<_>) = incoming_comms_message
                .into_iter()
                .filter(|v| !v.1.is_empty())
                .partition(|v| v.1[0] == COMMS_MSG_TYPE_BINARY);

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
            tracing::info!("{}: shutting down", std::thread::current().name().unwrap());

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
