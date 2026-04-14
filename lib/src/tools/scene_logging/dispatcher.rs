//! SceneLogDispatcher: GodotClass node that bridges Rust scene log entries to GDScript.
//!
//! Follows the NetworkInspector pattern: receives entries via tokio mpsc channel,
//! drains them in process() each frame, serializes to JSON, and emits a signal
//! that GDScript connects to for dispatching to WebSocket / EngineDebugger / file.

use std::borrow::Cow;
use std::collections::{HashMap, VecDeque};

use godot::classes::performance::Monitor;
use godot::classes::Performance;
use godot::prelude::*;
use tokio::sync::mpsc;

use super::config::SceneLoggingConfig;
use super::logger::{
    current_timestamp_ms, CrdtLogEntry, CrdtOperation, PerformanceSnapshotEntry, SceneLogEntry,
    SceneLoggerSender, SessionEndEntry, SessionStartEntry,
};
use super::storage::StorageManager;
use super::{is_lifecycle_verbose, set_lifecycle_verbose, take_dropped_count, try_send_entry};
use crate::godot_classes::dcl_global::DclGlobal;

/// Channel capacity for log entries.
const CHANNEL_CAPACITY: usize = 10_000;

/// Maximum entries to drain per frame to avoid frame spikes.
const MAX_ENTRIES_PER_FRAME: usize = 500;

/// Interval in seconds between performance snapshots.
const PERF_INTERVAL: f64 = 2.0;

#[derive(GodotClass)]
#[class(base=Node)]
pub struct SceneLogDispatcher {
    receiver: mpsc::Receiver<SceneLogEntry>,
    sender: mpsc::Sender<SceneLogEntry>,
    session_id: String,
    device_name: Option<String>,
    storage: Option<StorageManager>,
    perf_timer: f64,
    paused: bool,
    entry_count: u64,
    perf_interval: f64,
    /// Snapshot of latest LWW CRDT state: (scene_id, entity_id, component_name) → serialized JSON
    crdt_lww_snapshot: HashMap<(i32, u32, Cow<'static, str>), String>,
    /// Snapshot of GOS (append) CRDT state: (scene_id, entity_id, component_name) → serialized JSON entries.
    /// `VecDeque` so the per-key cap (see `update_crdt_snapshot`) drops the oldest entry in O(1).
    crdt_gos_snapshot: HashMap<(i32, u32, Cow<'static, str>), VecDeque<String>>,
    _base: Base<Node>,
}

#[godot_api]
impl SceneLogDispatcher {
    #[signal]
    fn scene_log_batch(entries_json: GString);

    /// Enable or disable JSONL file logging. Creates StorageManager on demand.
    #[func]
    fn set_file_logging(&mut self, enabled: bool) {
        if enabled && self.storage.is_none() {
            let config = SceneLoggingConfig::default();
            match StorageManager::new(config, self.session_id.clone()) {
                Ok(mut storage) => {
                    // Write session start marker
                    let start_entry = SceneLogEntry::SessionStart(SessionStartEntry {
                        session_id: self.session_id.clone(),
                        timestamp_ms: current_timestamp_ms(),
                        version: env!("CARGO_PKG_VERSION").to_string(),
                        platform: std::env::consts::OS.to_string(),
                        device_name: self.device_name.clone(),
                    });
                    let _ = storage.write_entry(&start_entry);
                    let _ = storage.flush();
                    self.storage = Some(storage);
                    tracing::info!(
                        "Scene log file logging enabled, session={}",
                        self.session_id
                    );
                }
                Err(e) => {
                    tracing::error!("Failed to create scene log storage: {}", e);
                }
            }
        } else if !enabled {
            // Flush and drop storage
            if let Some(ref mut storage) = self.storage {
                let end_entry = SceneLogEntry::SessionEnd(SessionEndEntry {
                    session_id: self.session_id.clone(),
                    timestamp_ms: current_timestamp_ms(),
                });
                let _ = storage.write_entry(&end_entry);
                let _ = storage.flush();
            }
            self.storage = None;
        }
    }

    /// Returns the unique session ID for this logging session.
    #[func]
    fn get_session_id(&self) -> GString {
        GString::from(&self.session_id)
    }

    /// Track paused state for status reporting. Scene processing is paused from GDScript
    /// via `SceneManager.set_scene_is_paused`; this field is informational only.
    #[func]
    fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }

    /// Returns whether scene processing is currently paused.
    #[func]
    fn is_paused(&self) -> bool {
        self.paused
    }

    /// Returns the total number of entries processed since session start.
    #[func]
    fn get_entry_count(&self) -> u64 {
        self.entry_count
    }

    /// Change the performance snapshot interval (in seconds). Clamped to [0.5, 60.0].
    #[func]
    fn set_perf_interval(&mut self, seconds: f64) {
        self.perf_interval = seconds.clamp(0.5, 60.0);
    }

    /// Returns the current performance snapshot interval.
    #[func]
    fn get_perf_interval(&self) -> f64 {
        self.perf_interval
    }

    /// Returns whether file logging is currently enabled.
    #[func]
    fn is_file_logging(&self) -> bool {
        self.storage.is_some()
    }

    /// Toggle per-tick lifecycle events (`OnUpdate` / `OnUpdateEnd`).
    /// Default is `true`; disable when CRDT/ops are the only signal of interest.
    #[func]
    fn set_lifecycle_verbose(&mut self, enabled: bool) {
        set_lifecycle_verbose(enabled);
    }

    #[func]
    fn is_lifecycle_verbose(&self) -> bool {
        is_lifecycle_verbose()
    }

    /// Returns a JSON array of the current CRDT state snapshot for hot-connect.
    /// Includes all LWW and GOS entries so a newly connected inspector can
    /// reconstruct the full entity tree.
    #[func]
    fn get_crdt_snapshot_json(&self) -> GString {
        let mut entries: Vec<&str> = Vec::with_capacity(
            self.crdt_lww_snapshot.len()
                + self
                    .crdt_gos_snapshot
                    .values()
                    .map(|v| v.len())
                    .sum::<usize>(),
        );
        for json in self.crdt_lww_snapshot.values() {
            entries.push(json);
        }
        for gos_entries in self.crdt_gos_snapshot.values() {
            for json in gos_entries {
                entries.push(json);
            }
        }
        if entries.is_empty() {
            return GString::new();
        }
        let result = format!("[{}]", entries.join(","));
        GString::from(&result)
    }

    /// Emit a session_start entry into the channel (sent over WS on next batch).
    /// Resolves device name lazily since mobile plugins may not be ready at init time.
    #[func]
    fn emit_session_start(&mut self) {
        // Resolve device name lazily if not yet set
        if self.device_name.is_none() {
            self.device_name = Self::detect_device_name();
        }
        let entry = SceneLogEntry::SessionStart(SessionStartEntry {
            session_id: self.session_id.clone(),
            timestamp_ms: current_timestamp_ms(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            platform: std::env::consts::OS.to_string(),
            device_name: self.device_name.clone(),
        });
        try_send_entry(&self.sender, entry);
    }
}

impl SceneLogDispatcher {
    /// Gets a clone of the sender for Rust-side callers.
    pub fn get_sender(&self) -> SceneLoggerSender {
        self.sender.clone()
    }

    /// Update the CRDT snapshot based on an incoming CRDT entry.
    fn update_crdt_snapshot(&mut self, crdt: &CrdtLogEntry, json: &str) {
        let sid = crdt.scene_id;
        let key = (sid, crdt.entity_id, crdt.component_name.clone());
        match crdt.operation {
            CrdtOperation::Put => {
                self.crdt_lww_snapshot.insert(key, json.to_string());
            }
            CrdtOperation::Delete => {
                self.crdt_lww_snapshot.remove(&key);
            }
            CrdtOperation::DeleteEntity => {
                let eid = crdt.entity_id;
                self.crdt_lww_snapshot
                    .retain(|k, _| !(k.0 == sid && k.1 == eid));
                self.crdt_gos_snapshot
                    .retain(|k, _| !(k.0 == sid && k.1 == eid));
            }
            CrdtOperation::Append => {
                // NOTE: cap is per-(scene,entity,component). Kept per-key (not
                // global) so a single chatty component can't evict snapshot
                // entries from quieter ones; revisit if total snapshot memory
                // becomes a concern.
                let entries = self.crdt_gos_snapshot.entry(key).or_default();
                if entries.len() >= 100 {
                    entries.pop_front();
                }
                entries.push_back(json.to_string());
            }
        }
    }

    /// Detect device name from iOS/Android plugins, or fallback to OS name.
    fn detect_device_name() -> Option<String> {
        #[cfg(target_os = "ios")]
        {
            if let Some(info) =
                crate::godot_classes::dcl_ios_plugin::DclIosPlugin::get_mobile_device_info_internal(
                )
            {
                let name = format!("{} {}", info.device_brand, info.device_model)
                    .trim()
                    .to_string();
                if !name.is_empty() {
                    return Some(name);
                }
            }
        }

        #[cfg(target_os = "android")]
        {
            if let Some(info) = crate::godot_classes::dcl_android_plugin::DclAndroidPlugin::get_mobile_device_info_internal()
            {
                let name = format!("{} {}", info.device_brand, info.device_model)
                    .trim()
                    .to_string();
                if !name.is_empty() {
                    return Some(name);
                }
            }
        }

        // Desktop fallback
        Some(std::env::consts::OS.to_string())
    }

    /// Collects a performance snapshot from Godot Performance singleton,
    /// SceneManager (Deno/V8 stats), and ContentProvider (asset loading).
    fn collect_performance_snapshot(&self) -> Option<String> {
        let performance = Performance::singleton();

        let fps = performance.get_monitor(Monitor::TIME_FPS);
        let dt = 1.0 / fps.max(1.0);
        let draw_calls = performance.get_monitor(Monitor::RENDER_TOTAL_DRAW_CALLS_IN_FRAME) as i64;
        let primitives = performance.get_monitor(Monitor::RENDER_TOTAL_PRIMITIVES_IN_FRAME) as i64;
        let objects_in_frame =
            performance.get_monitor(Monitor::RENDER_TOTAL_OBJECTS_IN_FRAME) as i64;
        let mem_static_mb = performance.get_monitor(Monitor::MEMORY_STATIC) / 1_048_576.0;
        let mem_gpu_mb = performance.get_monitor(Monitor::RENDER_VIDEO_MEM_USED) / 1_048_576.0;

        // Rust heap (feature-gated)
        #[cfg(feature = "use_memory_debugger")]
        let mem_rust_mb = {
            let allocated =
                crate::tools::memory_debugger::ALLOCATED.load(std::sync::atomic::Ordering::Relaxed);
            let deallocated = crate::tools::memory_debugger::DEALLOCATED
                .load(std::sync::atomic::Ordering::Relaxed);
            (allocated.saturating_sub(deallocated)) as f64 / 1_048_576.0
        };
        #[cfg(not(feature = "use_memory_debugger"))]
        let mem_rust_mb = 0.0;

        // Deno/V8 memory + asset loading from DclGlobal
        let (
            js_heap_total_mb,
            js_heap_used_mb,
            js_heap_limit_mb,
            js_external_mb,
            scene_count,
            assets_loading,
            assets_loaded,
            download_speed_mbs,
        ) = if let Some(global) = DclGlobal::try_singleton() {
            let global_ref = global.bind();
            let sr = global_ref.scene_runner.bind();
            let js_total = sr.get_total_deno_heap_size_mb();
            let js_used = sr.get_total_deno_memory_mb();
            let js_external = sr.get_total_deno_external_memory_mb();
            let js_limit = sr.get_total_deno_heap_limit_mb();
            let sc = sr.get_deno_scene_count();
            drop(sr);

            let cp = global_ref.content_provider.bind();
            let loading = cp.count_loading_resources();
            let loaded = cp.count_loaded_resources();
            let speed = cp.get_download_speed_mbs();
            drop(cp);

            (
                js_total,
                js_used,
                js_limit,
                js_external,
                sc,
                loading,
                loaded,
                speed,
            )
        } else {
            (0.0, 0.0, 0.0, 0.0, 0, 0, 0, 0.0)
        };

        let entry = SceneLogEntry::PerformanceSnapshot(PerformanceSnapshotEntry {
            timestamp_ms: current_timestamp_ms(),
            fps,
            dt,
            draw_calls,
            primitives,
            objects_in_frame,
            mem_static_mb,
            mem_gpu_mb,
            mem_rust_mb,
            js_heap_total_mb,
            js_heap_used_mb,
            js_heap_limit_mb,
            js_external_mb,
            assets_loading,
            assets_loaded,
            download_speed_mbs,
            scene_count,
        });

        match serde_json::to_string(&entry) {
            Ok(json) => Some(json),
            Err(e) => {
                tracing::warn!("Failed to serialize perf snapshot: {}", e);
                None
            }
        }
    }
}

#[godot_api]
impl INode for SceneLogDispatcher {
    fn init(_base: Base<Node>) -> Self {
        let (sender, receiver) = mpsc::channel(CHANNEL_CAPACITY);
        let session_id = uuid::Uuid::new_v4().to_string();

        // Detect device name from mobile plugins or OS
        let device_name = Self::detect_device_name();

        SceneLogDispatcher {
            receiver,
            sender,
            session_id,
            device_name,
            storage: None,
            perf_timer: 0.0,
            paused: false,
            entry_count: 0,
            perf_interval: PERF_INTERVAL,
            crdt_lww_snapshot: HashMap::new(),
            crdt_gos_snapshot: HashMap::new(),
            _base,
        }
    }

    fn process(&mut self, dt: f64) {
        let mut batch = Vec::new();
        let mut count = 0;

        while count < MAX_ENTRIES_PER_FRAME {
            match self.receiver.try_recv() {
                Ok(entry) => {
                    self.entry_count += 1;
                    // Serialize to JSON
                    match serde_json::to_string(&entry) {
                        Ok(json) => {
                            // Maintain CRDT snapshot for hot-connect
                            if let SceneLogEntry::CrdtMessage(ref crdt) = entry {
                                self.update_crdt_snapshot(crdt, &json);
                            }
                            // Write to file if enabled. Reuse the already-
                            // serialized `json` rather than asking storage to
                            // serialize the entry again.
                            if let Some(ref mut storage) = self.storage {
                                let _ = storage.write_serialized(&json);
                            }
                            batch.push(json);
                        }
                        Err(e) => {
                            tracing::warn!("Failed to serialize scene log entry: {}", e);
                        }
                    }
                    count += 1;
                }
                Err(_) => break,
            }
        }

        // Collect performance snapshot every perf_interval seconds
        self.perf_timer += dt;
        if self.perf_timer >= self.perf_interval {
            self.perf_timer = 0.0;
            if let Some(snapshot_json) = self.collect_performance_snapshot() {
                batch.push(snapshot_json);
            }
            // Report any entries dropped because the channel was full since the
            // previous tick. Silent drops would mask real loss of telemetry.
            let dropped = take_dropped_count();
            if dropped > 0 {
                tracing::warn!(
                    "Scene logging dropped {} entries (channel full) in the last {:.1}s",
                    dropped,
                    self.perf_interval
                );
            }
        }

        // Flush file storage periodically
        if count > 0 {
            if let Some(ref mut storage) = self.storage {
                let _ = storage.flush();
            }
        }

        // Emit signal with batched entries as JSON array
        if !batch.is_empty() {
            let json_array = format!("[{}]", batch.join(","));
            self.base_mut().emit_signal(
                "scene_log_batch",
                &[GString::from(&json_array).to_variant()],
            );
        }
    }
}
