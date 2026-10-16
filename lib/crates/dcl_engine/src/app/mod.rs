//! The app's own services: what the world root owns besides the engine itself (settings
//! application, music, snapshots, logging checks, chat commands, ...).

pub mod audio_settings;
pub mod chat_handler;
pub mod general_settings;
pub mod gltf_importer;
pub mod graphics;
pub mod graphics_autotune;
pub mod hardware_benchmark;
pub mod locale;
pub mod music_player;
pub mod position_broadcaster;
pub mod scene_music;
pub mod snapshot;
