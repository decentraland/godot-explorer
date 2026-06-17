//! Import-time asset transforms applied by `cli.asset_server` during
//! the bake pass. Each stage trims something the device would otherwise
//! pay for at runtime (vertex count, vertex stream bytes, draw cost
//! behind occluders).
pub mod mesh_occluder;
pub mod vertex_strip;

pub mod metrics;

pub use metrics::drain_global_stats;
