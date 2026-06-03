//! Import-time asset transforms applied by `cli.asset_server` during the bake
//! pass. Auto-attaches occluders so the device culls draw cost behind them.
pub mod mesh_occluder;
