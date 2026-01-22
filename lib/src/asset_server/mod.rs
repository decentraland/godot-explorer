//! Asset Optimization Server
//!
//! This module provides an HTTP server mode that processes Decentraland assets
//! (Scene GLTF, Emote GLTF, Wearable GLTF, Textures) on-demand and caches them persistently.
//!
//! Run via: `cargo run -- run --asset-server`

mod godot_wrapper;
mod handlers;
mod job_manager;
mod packer;
mod processor;
mod scene_fetcher;
mod server;
mod types;

pub use godot_wrapper::DclAssetServer;
pub use server::AssetServer;
pub use types::{
    AssetType, BatchStatus, JobStatus, ProcessRequest, ProcessSceneRequest,
    SceneOptimizationMetadata,
};
