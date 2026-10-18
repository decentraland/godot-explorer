//! The engine side of the app: the handles the Rust code used to find by node path, behind one
//! persistent node that lives under `/root`.

pub mod bootstrap;
mod core_link;
mod dcl_engine;
pub mod engine_fork;
pub mod frame_budget;
mod itests;
pub mod local_task;
pub mod root;
mod scene_link;
pub mod scene_requests;
mod screenshot_link;
pub(crate) mod settings_link;

pub use dcl_engine::DclEngine;
pub(crate) use settings_link::{benchmark_gpu_score, texture_quality_setting};
