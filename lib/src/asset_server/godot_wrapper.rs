//! Godot wrapper for the asset optimization server.
//!
//! This module provides a Godot class that can be instantiated from GDScript
//! to start the asset optimization server.

use godot::classes::Node;
use godot::prelude::*;

use super::server::start_server;
use crate::scene_runner::tokio_runtime::TokioRuntime;

/// Godot wrapper for the asset optimization server.
#[derive(GodotClass)]
#[class(base=Node)]
pub struct DclAssetServer {
    base: Base<Node>,
    port: u16,
    is_running: bool,
}

#[godot_api]
impl INode for DclAssetServer {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            port: 8080,
            is_running: false,
        }
    }

    /// Per-frame main-thread tick that drains the impostor bake queue.
    /// Workers parked under `BAKE_QUEUE` get resolved here, where
    /// `RenderingServer::force_draw()` actually flushes SubViewport
    /// rasterization. One drain per frame, so a scene that registers
    /// 50 candidates resolves in a single render and unblocks its
    /// worker immediately.
    fn process(&mut self, _delta: f64) {
        let mut parent: Gd<Node> = self.base().clone().upcast();
        crate::content::gltf::octahedral_impostor::drain_bake_queue_on_main(&mut parent);
    }
}

#[godot_api]
impl DclAssetServer {
    /// Set the port for the server.
    #[func]
    pub fn set_port(&mut self, port: i32) {
        self.port = port as u16;
    }

    /// Get the current port.
    #[func]
    pub fn get_port(&self) -> i32 {
        self.port as i32
    }

    /// Check if the server is running.
    #[func]
    pub fn is_running(&self) -> bool {
        self.is_running
    }

    /// Start the asset optimization server.
    ///
    /// This function spawns the HTTP server on a separate tokio runtime.
    /// It returns immediately after starting the server.
    #[func]
    pub fn start(&mut self) {
        if self.is_running {
            tracing::warn!("Asset server is already running");
            return;
        }

        let port = self.port;
        tracing::info!("Starting asset optimization server on port {}", port);

        // Spawn the server on the tokio runtime
        TokioRuntime::spawn(async move {
            if let Err(e) = start_server(port).await {
                tracing::error!("Asset server error: {}", e);
            }
        });

        self.is_running = true;

        // Print startup message
        godot_print!("Asset Optimization Server started on port {}", port);
        godot_print!("Endpoints:");
        godot_print!("  POST /process - Submit an asset for processing");
        godot_print!("  GET  /status/{{job_id}} - Get job status");
        godot_print!("  GET  /jobs - List all jobs");
        godot_print!("  GET  /health - Health check");
    }

    /// Stop the asset optimization server.
    ///
    /// Note: Currently this doesn't actually stop the server since we don't
    /// have a clean shutdown mechanism. The server will stop when the process exits.
    #[func]
    pub fn stop(&mut self) {
        if !self.is_running {
            tracing::warn!("Asset server is not running");
            return;
        }

        // TODO: Implement clean shutdown using a channel/signal
        tracing::warn!("Asset server stop requested - server will stop when process exits");
        self.is_running = false;
    }
}
