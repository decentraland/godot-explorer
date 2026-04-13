//! SceneLogDispatcher: GodotClass node that bridges Rust scene log entries to GDScript.
//!
//! Follows the NetworkInspector pattern: receives entries via tokio mpsc channel,
//! drains them in process() each frame, serializes to JSON, and emits a signal
//! that GDScript connects to for dispatching to WebSocket / EngineDebugger / file.

use godot::classes::performance::Monitor;
use godot::classes::Performance;
use godot::prelude::*;
use tokio::sync::mpsc;

use super::config::SceneLoggingConfig;
use super::logger::{
    current_timestamp_ms, PerformanceSnapshotEntry, SceneLogEntry, SceneLoggerSender,
    SessionEndEntry, SessionStartEntry,
};
use super::storage::StorageManager;
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
    storage: Option<StorageManager>,
    perf_timer: f64,
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
                    total_crdt_messages: 0,
                    total_op_calls: 0,
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
}

impl SceneLogDispatcher {
    /// Gets a clone of the sender for Rust-side callers.
    pub fn get_sender(&self) -> SceneLoggerSender {
        self.sender.clone()
    }

    /// Collects a performance snapshot from Godot Performance singleton,
    /// SceneManager (Deno/V8 stats), and ContentProvider (asset loading).
    fn collect_performance_snapshot(&self) -> Option<String> {
        let performance = Performance::singleton();

        let fps = performance.get_monitor(Monitor::TIME_FPS);
        let dt = 1.0 / fps.max(1.0);
        let draw_calls =
            performance.get_monitor(Monitor::RENDER_TOTAL_DRAW_CALLS_IN_FRAME) as i64;
        let primitives =
            performance.get_monitor(Monitor::RENDER_TOTAL_PRIMITIVES_IN_FRAME) as i64;
        let objects_in_frame =
            performance.get_monitor(Monitor::RENDER_TOTAL_OBJECTS_IN_FRAME) as i64;
        let mem_static_mb =
            performance.get_monitor(Monitor::MEMORY_STATIC) / 1_048_576.0;
        let mem_gpu_mb =
            performance.get_monitor(Monitor::RENDER_VIDEO_MEM_USED) / 1_048_576.0;

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
        let (js_heap_total_mb, js_heap_used_mb, js_heap_limit_mb, js_external_mb, scene_count, assets_loading, assets_loaded, download_speed_mbs) =
            if let Some(global) = DclGlobal::try_singleton() {
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

                (js_total, js_used, js_limit, js_external, sc, loading, loaded, speed)
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

        SceneLogDispatcher {
            receiver,
            sender,
            session_id,
            storage: None,
            perf_timer: 0.0,
            _base,
        }
    }

    fn process(&mut self, dt: f64) {
        let mut batch = Vec::new();
        let mut count = 0;

        while count < MAX_ENTRIES_PER_FRAME {
            match self.receiver.try_recv() {
                Ok(entry) => {
                    // Serialize to JSON
                    match serde_json::to_string(&entry) {
                        Ok(json) => {
                            // Write to file if enabled
                            if let Some(ref mut storage) = self.storage {
                                let _ = storage.write_entry(&entry);
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

        // Collect performance snapshot every PERF_INTERVAL seconds
        self.perf_timer += dt;
        if self.perf_timer >= PERF_INTERVAL {
            self.perf_timer = 0.0;
            if let Some(snapshot_json) = self.collect_performance_snapshot() {
                batch.push(snapshot_json);
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
