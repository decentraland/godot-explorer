//! Engine-free core of the Decentraland explorer: sessions, networking and rules.

pub mod clock;
mod core;
pub mod deep_link;
pub mod env;
pub mod http;
pub mod network_inspector;
mod open_url;
pub mod proto;
pub mod session;
pub mod social;
pub mod urls;

pub use crate::core::{data_file, Core, CoreConfig, Platform};
pub use crate::open_url::OpenUrlRequest;

/// User-Agent for all outgoing reqwest HTTP clients. reqwest sends none by
/// default, and CDN bot protection (e.g. arweave.net's CDN77) rate-limits
/// UA-less clients into 403s (#1766). We reuse Unity Explorer's UA: if a CDN
/// ever blocks it, both clients break together and the issue gets flagged fast.
pub const USER_AGENT: &str = "DecentralandGodotExplorer UnityWebRequest/1.0 (UnityWebRequest)";
