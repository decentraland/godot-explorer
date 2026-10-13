//! The engine side of the app: the handles the Rust code used to find by node path, behind one
//! persistent node that lives under `/root`.

mod dcl_engine;

pub use dcl_engine::DclEngine;
