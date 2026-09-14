//! Main-thread cache of the component `PackedScene`s the scene runner
//! instantiates per entity (`gltf_container.tscn`, `mesh_renderer.tscn`, …).
//!
//! `godot::tools::load` returns a `Gd<PackedScene>` that is dropped right after
//! `instantiate()`. Godot's `ResourceCache` only holds weak references, so once
//! the last strong ref is gone the next `load` re-reads and re-parses the scene
//! from the pack — for Genesis Plaza that was ~600 GltfContainer entities in
//! the first seconds, each paying a `ResourceFormatLoaderBinary::load` +
//! `GDScriptCache::get_full_script` on the main thread. Keeping one strong
//! reference per path makes every later call a hash-map lookup.

use std::cell::RefCell;
use std::collections::HashMap;

use godot::classes::PackedScene;
use godot::prelude::*;

thread_local! {
    static CACHE: RefCell<HashMap<&'static str, Gd<PackedScene>>> = RefCell::new(HashMap::new());
}

/// The `PackedScene` at `path`, loaded once per thread and kept alive.
pub fn packed_scene(path: &'static str) -> Gd<PackedScene> {
    CACHE.with(|cache| {
        if let Some(scene) = cache.borrow().get(path) {
            return scene.clone();
        }
        let scene = godot::tools::load::<PackedScene>(path);
        cache.borrow_mut().insert(path, scene.clone());
        scene
    })
}

/// Instantiate a cached `PackedScene` and cast the root to `T`.
pub fn instantiate<T: Inherits<Node>>(path: &'static str) -> Gd<T> {
    packed_scene(path)
        .instantiate()
        .unwrap_or_else(|| panic!("failed to instantiate {path}"))
        .cast::<T>()
}
