/*
 * Content Converter Server
 *
 * HTTP server for converting GLB/GLTF/Images to optimized Godot resources (.scn/.res)
 * and packaging them into mobile-optimized ZIP bundles.
 *
 * Usage:
 *   cargo run -- converter-server --port 3000 --cache-folder ./cache
 */

mod handlers;
mod server;
mod zip_builder;

pub use server::ConverterServer;
