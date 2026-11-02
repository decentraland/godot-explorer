//! Per-scene bookkeeping of EXTERNAL (non-deployed) content a scene pulls at
//! runtime, powering the preview scene-stats "External content" row:
//!
//! - textures whose source is an http(s) URL — downloaded by the content
//!   provider into `user://content/hashed_{hex}`; only the cache file NAME is
//!   recorded here — sizes are summed from the ResourceProvider's in-memory
//!   cache metadata (`ContentProvider.get_cache_size_for_base_names`)
//! - bytes the scene's JS consumed through `fetch()` — never stored on disk,
//!   so they are accumulated directly
//!
//! External video is streamed (never stored) and intentionally not tracked.
//! Everything here is passive in-memory bookkeeping — no I/O.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use godot::prelude::*;
use once_cell::sync::Lazy;

#[derive(Default)]
struct SceneExternalContent {
    files: HashSet<String>,
    fetch_bytes: u64,
}

/// Keyed by scene entity-definition id (stable across the scene's lifetime;
/// a reload spawns a fresh scene with the same id, cleared on scene removal).
static REGISTRY: Lazy<Mutex<HashMap<String, SceneExternalContent>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// On-disk cache filename for a url-sourced texture. MUST stay in sync with
/// the naming used by ContentProvider's url-texture branches (which call this
/// same helper).
pub fn external_texture_file_name(url: &GString) -> String {
    format!("hashed_{:x}", url.hash_u32())
}

pub fn register_external_texture(scene_ent_id: &str, url: &str) {
    let file_name = external_texture_file_name(&GString::from(url));
    if let Ok(mut registry) = REGISTRY.lock() {
        registry
            .entry(scene_ent_id.to_owned())
            .or_default()
            .files
            .insert(file_name);
    }
}

pub fn add_fetch_bytes(scene_ent_id: &str, bytes: u64) {
    if let Ok(mut registry) = REGISTRY.lock() {
        registry
            .entry(scene_ent_id.to_owned())
            .or_default()
            .fetch_bytes += bytes;
    }
}

/// (cache filenames, cumulative fetch bytes) for a scene; empty if unknown.
pub fn snapshot(scene_ent_id: &str) -> (Vec<String>, u64) {
    if let Ok(registry) = REGISTRY.lock() {
        if let Some(entry) = registry.get(scene_ent_id) {
            return (entry.files.iter().cloned().collect(), entry.fetch_bytes);
        }
    }
    (Vec::new(), 0)
}

/// Drop a scene's bookkeeping when the scene is removed, so a reloaded scene
/// starts counting from zero instead of accumulating across lives.
pub fn clear_scene(scene_ent_id: &str) {
    if let Ok(mut registry) = REGISTRY.lock() {
        registry.remove(scene_ent_id);
    }
}
