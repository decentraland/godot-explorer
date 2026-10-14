//! The engine side of the app: the handles the Rust code used to find by node path, behind one
//! persistent node that lives under `/root`.

pub mod bootstrap;
mod core_link;
mod dcl_engine;
mod itests;
pub mod root;
pub mod scene_requests;

pub use dcl_engine::DclEngine;
