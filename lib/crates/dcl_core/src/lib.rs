//! Engine-free core of the Decentraland explorer: sessions, networking and rules.

pub mod analytics;
pub mod app;
pub mod attestation;
pub mod avatar;
pub mod cache;
pub mod clock;
pub mod connection;
mod core;
pub mod deep_link;
pub mod deep_link_route;
pub mod destination;
pub mod env;
#[cfg(feature = "ffi")]
pub mod ffi;
pub mod flags;
pub mod gates;
pub mod http;
pub mod hub;
mod lifecycle;
pub mod logging;
pub mod network_inspector;
pub mod notifications;
mod open_url;
pub mod places;
pub mod profile;
pub mod realm;
pub mod report;
pub mod review;
pub mod scene;
pub mod scene_requests;
pub mod session;
pub mod settings;
pub mod urls;

#[cfg(feature = "ffi")]
uniffi::setup_scaffolding!();

pub use crate::core::{data_file, Core, CoreConfig, Platform};
pub use crate::open_url::OpenUrlRequest;

/// User-Agent for all outgoing reqwest HTTP clients. reqwest sends none by
/// default, and CDN bot protection (e.g. arweave.net's CDN77) rate-limits
/// UA-less clients into 403s (#1766). We reuse Unity Explorer's UA: if a CDN
/// ever blocks it, both clients break together and the issue gets flagged fast.
pub const USER_AGENT: &str = "DecentralandGodotExplorer UnityWebRequest/1.0 (UnityWebRequest)";
