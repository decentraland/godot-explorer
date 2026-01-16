//! GLTF loading module for scenes, wearables, and emotes.
//!
//! This module provides async functions to download, process, and save GLTF assets
//! for the Decentraland explorer.

mod common;
mod emote;
mod scene;
mod wearable;

// Re-export public API (maintains compatibility with content_provider.rs)
pub use emote::{
    build_dcl_emote_gltf, get_last_16_alphanumeric, load_and_save_emote_gltf, DclEmoteGltf,
};
pub use scene::load_and_save_scene_gltf;
pub use wearable::load_and_save_wearable_gltf;
