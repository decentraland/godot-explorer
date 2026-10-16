//! The debug hub: the client end of the developer-only command channel (the Scene Inspector
//! protocol). External tools (the hub, an MCP server, the inspector app) connect to the hub, which
//! relays to the client over one WebSocket: stream the inspector's entries and run inspection and
//! control commands.

mod bridge;
mod collector;
mod commands;
mod frames;
mod json;
mod socket;

pub use bridge::SceneInspectorBridge;
