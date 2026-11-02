//! The world the engine runs and the host that drives it for the shell: the explorer scene, its
//! controls, the avatar-only scene, the chat commands and the commands the core sends.

pub mod api;
pub mod avatar_host;
mod backpack;
pub mod chat;
mod chat_commands;
mod controls;
pub mod enter;
pub mod explorer;
mod host;
mod hud;
mod navigate;
mod session_watch;
pub mod state;
mod tick;

pub use chat_commands::submit_message;
pub use controls::{capture_mouse, release_mouse, reset_cursor_position, set_cursor_position};
pub use host::EngineHost;
