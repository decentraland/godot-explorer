use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex, OnceLock,
    },
};

use deno_core::{op2, OpDecl, OpState};
use tokio::sync::mpsc::Receiver;

use crate::dcl::{
    common::{
        CommunicatedWithRenderer, SceneDying, SceneElapsedTime, SceneLogs, SceneMainCrdtFileContent,
    },
    crdt::{
        message::{
            append_gos_component, process_many_messages_with_logging, put_or_delete_lww_component,
        },
        CrdtLoggingContext, SceneCrdtState,
    },
    scene_apis::{LocalCall, RpcCall},
    serialization::{reader::DclReader, writer::DclWriter},
    RendererResponse, SceneId, SceneResponse, SharedSceneCrdtState,
};

use super::scene_inspector_ops::SceneDebugFlag;

/// Current frame tick, set by `scene_thread` at the start of each `onUpdate`
/// iteration and read by `op_crdt_send_to_renderer` / `op_crdt_recv_from_renderer`
/// to tag CRDT log entries with the correct frame number. Always present in
/// `op_state`; only consulted when `SceneDebugFlag` is on, but kept unconditional
/// so the hot path doesn't pay an extra `try_borrow` lookup.
///
/// `Cell<u32>` rather than `AtomicU32`: `OpState` is `!Send` and the scene runs
/// single-threaded inside its own tokio runtime, so we don't need atomics.
pub struct SceneTickCounter(pub Cell<u32>);

/// Cross-thread CRDT throughput counters. With ~70 GP scene threads each
/// running their own JsRuntime, we need atomics to aggregate. Drained from
/// the GP benchmark runner via `SceneManager::drain_crdt_metrics` so we can
/// quantify the V8↔Rust round-trip cost per frame in the JSON output.
static CRDT_SEND_BYTES: AtomicU64 = AtomicU64::new(0);
static CRDT_SEND_OPS: AtomicU64 = AtomicU64::new(0);
static CRDT_RECV_BYTES: AtomicU64 = AtomicU64::new(0);
static CRDT_RECV_OPS: AtomicU64 = AtomicU64::new(0);
static CRDT_DIRTY_LWW_ENTRIES: AtomicU64 = AtomicU64::new(0);
static CRDT_DIRTY_GOS_ENTRIES: AtomicU64 = AtomicU64::new(0);

/// Gates the per-component breakdown maps. Without it, every recv across all
/// scene threads contends on a global Mutex<HashMap>, serializing the CRDT
/// pipeline. The bench runner flips this on right before sampling and the
/// drain function turns it off afterwards.
pub static CRDT_BREAKDOWN_ENABLED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Per-component-id breakdown of dirty entries seen on the Rust→V8 path.
/// Lazy-init Mutex<HashMap> — only touched when bench instrumentation is
/// reading/draining; the hot path is a single `lock()` + `entry().or_insert(0)`
/// per component-id per recv (~70 entries/frame, not per-message).
fn crdt_dirty_lww_by_component() -> &'static Mutex<HashMap<u32, u64>> {
    static M: OnceLock<Mutex<HashMap<u32, u64>>> = OnceLock::new();
    M.get_or_init(|| Mutex::new(HashMap::new()))
}

fn crdt_dirty_gos_by_component() -> &'static Mutex<HashMap<u32, u64>> {
    static M: OnceLock<Mutex<HashMap<u32, u64>>> = OnceLock::new();
    M.get_or_init(|| Mutex::new(HashMap::new()))
}

#[derive(Debug, Default, Clone, Copy)]
pub struct CrdtMetricsSnapshot {
    pub send_bytes: u64,
    pub send_ops: u64,
    pub recv_bytes: u64,
    pub recv_ops: u64,
    pub dirty_lww_entries: u64,
    pub dirty_gos_entries: u64,
}

pub fn drain_crdt_metrics() -> CrdtMetricsSnapshot {
    CrdtMetricsSnapshot {
        send_bytes: CRDT_SEND_BYTES.swap(0, Ordering::Relaxed),
        send_ops: CRDT_SEND_OPS.swap(0, Ordering::Relaxed),
        recv_bytes: CRDT_RECV_BYTES.swap(0, Ordering::Relaxed),
        recv_ops: CRDT_RECV_OPS.swap(0, Ordering::Relaxed),
        dirty_lww_entries: CRDT_DIRTY_LWW_ENTRIES.swap(0, Ordering::Relaxed),
        dirty_gos_entries: CRDT_DIRTY_GOS_ENTRIES.swap(0, Ordering::Relaxed),
    }
}

pub fn reset_crdt_metrics() {
    CRDT_SEND_BYTES.store(0, Ordering::Relaxed);
    CRDT_SEND_OPS.store(0, Ordering::Relaxed);
    CRDT_RECV_BYTES.store(0, Ordering::Relaxed);
    CRDT_RECV_OPS.store(0, Ordering::Relaxed);
    CRDT_DIRTY_LWW_ENTRIES.store(0, Ordering::Relaxed);
    CRDT_DIRTY_GOS_ENTRIES.store(0, Ordering::Relaxed);
    crdt_dirty_lww_by_component().lock().unwrap().clear();
    crdt_dirty_gos_by_component().lock().unwrap().clear();
    CRDT_BREAKDOWN_ENABLED.store(true, Ordering::Relaxed);
}

#[derive(Debug, Default, Clone)]
pub struct CrdtComponentBreakdown {
    pub lww: Vec<(u32, u64)>,
    pub gos: Vec<(u32, u64)>,
}

/// Drain per-component-id dirty counts (lww, gos), sorted descending by count.
/// Used by bench runner to identify which SDK7 components dominate the V8↔Rust
/// round-trip. Side-effect: leaves recording disabled so post-sampling work
/// doesn't pollute the next bench window.
pub fn drain_crdt_component_breakdown() -> CrdtComponentBreakdown {
    CRDT_BREAKDOWN_ENABLED.store(false, Ordering::Relaxed);
    let mut lww: Vec<(u32, u64)> =
        std::mem::take(&mut *crdt_dirty_lww_by_component().lock().unwrap())
            .into_iter()
            .collect();
    lww.sort_by(|a, b| b.1.cmp(&a.1));
    let mut gos: Vec<(u32, u64)> =
        std::mem::take(&mut *crdt_dirty_gos_by_component().lock().unwrap())
            .into_iter()
            .collect();
    gos.sort_by(|a, b| b.1.cmp(&a.1));
    CrdtComponentBreakdown { lww, gos }
}

use super::{
    comms::{InternalPendingBinaryMessages, COMMS_MSG_TYPE_BINARY},
    events::process_events,
    players::{get_player_data, get_players},
};

// list of op declarations
pub fn ops() -> Vec<OpDecl> {
    vec![
        op_crdt_send_to_renderer(),
        op_crdt_recv_wait(),
        op_crdt_recv_drain(),
    ]
}

/// Framed recv buffer produced by `op_crdt_recv_wait`, drained into a
/// JS-owned `Uint8Array` by `op_crdt_recv_drain`.
///
/// Layout: `[u32 LE main_crdt_len][main_crdt bytes][data bytes]`.
///
/// Two ops because async ops can't take `#[buffer] &mut [u8]` (deno_core:
/// the borrow can't span an `await`). The split lets JS reuse the same
/// `Uint8Array` across every recv — no per-call V8 BackingStore allocation
/// in steady state.
struct PendingRecvBuffer(Vec<u8>);

// receive and process a buffer of crdt messages
#[op2(fast)]
fn op_crdt_send_to_renderer(op_state: Rc<RefCell<OpState>>, #[arraybuffer] messages: &[u8]) {
    CRDT_SEND_BYTES.fetch_add(messages.len() as u64, Ordering::Relaxed);
    CRDT_SEND_OPS.fetch_add(1, Ordering::Relaxed);
    let mut op_state = op_state.borrow_mut();
    let elapsed_time = op_state.borrow::<SceneElapsedTime>().0;
    let scene_id = op_state.take::<SceneId>();

    let logs = op_state.take::<SceneLogs>();
    op_state.put(SceneLogs(Vec::new()));

    let mutex_scene_crdt_state = op_state.take::<SharedSceneCrdtState>();
    let cloned_scene_crdt = mutex_scene_crdt_state.clone();
    let mut scene_crdt_state = cloned_scene_crdt.lock().unwrap();

    let mut stream = DclReader::new(messages);

    let debug = op_state.borrow::<SceneDebugFlag>().0;
    let logging_ctx = if debug {
        build_send_logging_ctx(&op_state)
    } else {
        None
    };
    process_many_messages_with_logging(&mut stream, &mut scene_crdt_state, logging_ctx.as_ref());

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

/// Awaits the renderer response, builds the framed CRDT payload, stashes
/// it for `op_crdt_recv_drain` to copy into JS memory. Returns the total
/// framed length so JS can grow its persistent recv buffer before draining.
///
/// Frame layout: `[u32 LE main_crdt_len][main_crdt bytes][data bytes]`.
/// `main_crdt_len` is 0 in steady state; main_crdt bytes only present on
/// the first recv per scene (carries the snapshot from disk). Length is
/// always >= 4 (the prefix).
#[op2(async)]
async fn op_crdt_recv_wait(op_state: Rc<RefCell<OpState>>) -> Result<u32, anyhow::Error> {
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
            CRDT_RECV_OPS.fetch_add(1, Ordering::Relaxed);
            CRDT_DIRTY_LWW_ENTRIES.fetch_add(
                dirty_crdt_state.lww.values().map(|v| v.len() as u64).sum(),
                Ordering::Relaxed,
            );
            CRDT_DIRTY_GOS_ENTRIES.fetch_add(
                dirty_crdt_state.gos.values().map(|v| v.len() as u64).sum(),
                Ordering::Relaxed,
            );
            // Per-component breakdown — gated by CRDT_BREAKDOWN_ENABLED so the
            // global mutex isn't contended outside the bench sampling window.
            // Each scene thread hits this on every recv (~70 scenes × 30 fps),
            // and a shared lock here serializes the whole CRDT pipeline.
            if CRDT_BREAKDOWN_ENABLED.load(Ordering::Relaxed) {
                {
                    let mut map = crdt_dirty_lww_by_component().lock().unwrap();
                    for (component_id, entities) in dirty_crdt_state.lww.iter() {
                        *map.entry(component_id.0).or_insert(0) += entities.len() as u64;
                    }
                }
                {
                    let mut map = crdt_dirty_gos_by_component().lock().unwrap();
                    for (component_id, entities) in dirty_crdt_state.gos.iter() {
                        *map.entry(component_id.0).or_insert(0) += entities.len() as u64;
                    }
                }
            }

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

            let debug = op_state.borrow::<SceneDebugFlag>().0;
            let scene_id_val = op_state.try_borrow::<SceneId>().map(|id| id.0).unwrap_or(0);
            let current_tick = if debug {
                op_state
                    .try_borrow::<SceneTickCounter>()
                    .map(|tc| tc.0.get())
                    .unwrap_or(0)
            } else {
                0
            };

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
                    if debug {
                        log_lww_renderer_to_scene(
                            &scene_crdt_state,
                            *component_id,
                            *entity_id,
                            current_tick,
                            scene_id_val,
                        );
                    }

                    if let Err(err) = put_or_delete_lww_component(
                        &scene_crdt_state,
                        entity_id,
                        component_id,
                        &mut data_writter,
                    ) {
                        tracing::debug!("error writing crdt message: {err}");
                    }
                }
            }

            for (component_id, entities) in dirty_crdt_state.gos.iter() {
                for (entity_id, element_count) in entities {
                    if dirty_crdt_state.entities.died.contains(entity_id) {
                        skipped_gos += 1;
                        continue;
                    }
                    if debug {
                        log_gos_renderer_to_scene(
                            *component_id,
                            *entity_id,
                            current_tick,
                            scene_id_val,
                        );
                    }

                    if let Err(err) = append_gos_component(
                        &scene_crdt_state,
                        entity_id,
                        component_id,
                        element_count,
                        &mut data_writter,
                    ) {
                        tracing::debug!("error writing crdt message: {err}");
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

    // Build the framed payload directly into a Vec<u8>. EngineApi.js
    // reconstructs the (optional main_crdt, data) split from the leading
    // u32 length prefix.
    let main_crdt = op_state
        .try_take::<SceneMainCrdtFileContent>()
        .map(|m| m.0)
        .unwrap_or_default();
    if !main_crdt.is_empty() {
        CRDT_RECV_BYTES.fetch_add(main_crdt.len() as u64, Ordering::Relaxed);
    }
    CRDT_RECV_BYTES.fetch_add(data.len() as u64, Ordering::Relaxed);

    let main_len = main_crdt.len() as u32;
    let mut framed = Vec::with_capacity(4 + main_crdt.len() + data.len());
    framed.extend_from_slice(&main_len.to_le_bytes());
    framed.extend_from_slice(&main_crdt);
    framed.extend_from_slice(&data);
    let total_len = framed.len() as u32;
    op_state.put(PendingRecvBuffer(framed));
    Ok(total_len)
}

/// Copies the buffer stashed by `op_crdt_recv_wait` into the JS-owned
/// `Uint8Array out`. Returns the byte count written, or 0 if `out` was
/// too small (the stash is preserved so the caller can grow + retry) or
/// no pending buffer exists.
#[op2(fast)]
fn op_crdt_recv_drain(state: &mut OpState, #[buffer] out: &mut [u8]) -> u32 {
    let Some(pending) = state.try_take::<PendingRecvBuffer>() else {
        return 0;
    };
    let len = pending.0.len();
    if len > out.len() {
        // Buffer too small. Put the stash back so the caller can grow + retry.
        state.put(pending);
        return 0;
    }
    out[..len].copy_from_slice(&pending.0);
    len as u32
}

/// Build the per-call CRDT logging context for the scene→renderer direction.
/// `#[cold]` so the logging branch never inflates the hot send path.
#[cold]
#[inline(never)]
fn build_send_logging_ctx(op_state: &OpState) -> Option<CrdtLoggingContext> {
    use crate::tools::scene_inspector::{get_logger_sender, CrdtDirection};

    let sender = get_logger_sender()?;
    let scene_id = op_state.try_borrow::<SceneId>().map(|id| id.0).unwrap_or(0);
    let tick = op_state
        .try_borrow::<SceneTickCounter>()
        .map(|tc| tc.0.get())
        .unwrap_or(0);
    Some(CrdtLoggingContext::new(
        sender,
        scene_id,
        tick,
        CrdtDirection::SceneToRenderer,
    ))
}

#[cold]
#[inline(never)]
fn log_lww_renderer_to_scene(
    scene_crdt_state: &SceneCrdtState,
    component_id: crate::dcl::components::SceneComponentId,
    entity_id: crate::dcl::components::SceneEntityId,
    current_tick: u32,
    scene_id: i32,
) {
    use crate::dcl::serialization::writer::DclWriter;
    use crate::tools::scene_inspector::{log_crdt_renderer_to_scene, CrdtOperation};

    let Some(comp_def) = scene_crdt_state.get_lww_component_definition(component_id) else {
        return;
    };
    let Some(opaque) = comp_def.get_opaque(entity_id) else {
        return;
    };

    let operation = if opaque.value.is_some() {
        CrdtOperation::Put
    } else {
        CrdtOperation::Delete
    };
    let payload_data = if opaque.value.is_some() {
        let mut payload_buf = Vec::new();
        let mut payload_writer = DclWriter::new(&mut payload_buf);
        if comp_def.to_binary(entity_id, &mut payload_writer).is_ok() {
            Some(payload_buf)
        } else {
            None
        }
    } else {
        None
    };

    log_crdt_renderer_to_scene(
        scene_id,
        current_tick,
        entity_id.as_i32() as u32,
        component_id.0,
        operation,
        opaque.timestamp.0,
        payload_data.as_deref(),
    );
}

#[cold]
#[inline(never)]
fn log_gos_renderer_to_scene(
    component_id: crate::dcl::components::SceneComponentId,
    entity_id: crate::dcl::components::SceneEntityId,
    current_tick: u32,
    scene_id: i32,
) {
    use crate::tools::scene_inspector::{log_crdt_renderer_to_scene, CrdtOperation};
    log_crdt_renderer_to_scene(
        scene_id,
        current_tick,
        entity_id.as_i32() as u32,
        component_id.0,
        CrdtOperation::Append,
        0,
        None,
    );
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
