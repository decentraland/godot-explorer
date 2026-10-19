//! The app's own services: what the world root owns besides the engine itself (settings
//! application, music, snapshots, logging checks, chat commands, ...).

pub mod analytics_controller;
pub mod async_shaders;
pub mod audio_settings;
pub mod chat_handler;
pub mod deep_link;
pub mod feature_flags;
pub mod general_settings;
pub mod global_api;
pub mod global_runtime;
pub mod gltf_importer;
pub mod graphics;
pub mod graphics_autotune;
pub mod hardware_benchmark;
pub mod locale;
pub mod logging_selftest;
pub mod music_player;
pub mod phone_frame;
pub mod position_broadcaster;
pub mod raycast_debugger;
pub mod safe_area_presets;
pub mod scene_music;
pub mod session;
pub mod sign_out;
pub mod snapshot;
