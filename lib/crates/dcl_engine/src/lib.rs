/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::prelude::*;

// Linked for its exported FFI symbols (the Kotlin and Swift host bindings).
#[cfg(feature = "ffi")]
extern crate dcl_host as _;

pub use dcl_core::USER_AGENT;

// Use the tracking allocator to monitor Rust heap usage in real-time when memory debugging is enabled
#[cfg(feature = "use_memory_debugger")]
#[global_allocator]
static GLOBAL: tools::memory_debugger::TrackingAllocator =
    tools::memory_debugger::TrackingAllocator;

pub mod analytics;
pub mod app;
#[cfg(feature = "asset_server")]
pub mod asset_server;
pub mod auth;
pub mod av;
pub mod avatars;
pub mod comms;
pub mod content;
pub mod dcl;
pub use dcl_core::deep_link;
pub use dcl_core::env;
pub mod engine;
pub mod fetcher;
pub mod godot_classes;
pub mod http_request;
pub mod lights;
pub mod media;
pub mod nft;
pub mod player;
pub mod profile;
pub mod realm;
pub mod scene_runner;
pub mod sky;
pub mod test_runner;
pub mod text_shape;
pub mod tools;
pub mod ui;
pub use dcl_core::urls;
pub mod utils;
pub mod world;

struct DecentralandGodotLibrary;

// SAFETY: gdext requires this marker; the library keeps the contract (no engine access before
// `InitLevel::Scene`, no `Gd` handed to another thread without a deferred call).
#[gdextension]
unsafe impl ExtensionLibrary for DecentralandGodotLibrary {
    fn on_stage_deinit(stage: InitStage) {
        if stage == InitStage::MainLoop {
            release_thread_local_engine_objects();
        }
    }
}

/// Rust drops thread locals when their thread exits, and Godot's main thread exits after
/// `Main::cleanup()` has torn the engine down, so releasing an engine object then aborts (Android:
/// "Pure virtual function called!" on every exit). The main loop's shutdown is the first step of
/// that cleanup: every main-thread thread local that keeps a resource, an `Array`, a `Dictionary`
/// or a `Callable` is emptied there. A new one must be added here.
fn release_thread_local_engine_objects() {
    app::global_api::release_engine_objects();
    avatars::mesh_assembler::release_engine_objects();
    godot_classes::dcl_ui_background::release_engine_objects();
    godot_classes::dcl_video_player::release_engine_objects();
    lights::component::release_engine_objects();
    scene_runner::components::material::release_engine_objects();
    scene_runner::gltf_loading::release_engine_objects();
    scene_runner::scene_cache::release_engine_objects();
    text_shape::fallbacks::release_engine_objects();
    text_shape::glyph_cache::release_engine_objects();
    text_shape::node::release_engine_objects();
    world::release_host_engine_objects();
}

pub mod framework {
    use godot::prelude::*;

    // Registers all the `#[itest]` tests.
    godot::sys::shard_registry!(pub(crate) __GODOT_ITEST: RustTestCase);

    pub struct TestContext {
        #[allow(dead_code)]
        pub scene_tree: Gd<Node>,
    }

    #[derive(Copy, Clone)]
    pub struct RustTestCase {
        pub name: &'static str,
        pub file: &'static str,
        pub skipped: bool,
        /// If one or more tests are focused, only they will be executed. Helpful for debugging and working on specific features.
        pub focused: bool,
        /// Tests that need the editor; the in-game runner skips them.
        pub editor_only: bool,
        #[allow(dead_code)]
        pub line: u32,
        pub function: fn(&TestContext),
    }

    /// Finds all `#[itest]` tests.
    pub fn collect_rust_tests() -> (Vec<RustTestCase>, usize, bool) {
        let mut all_files = std::collections::HashSet::new();
        let mut tests: Vec<RustTestCase> = vec![];
        let mut is_focus_run = false;

        godot::sys::shard_foreach!(__GODOT_ITEST; |test: &RustTestCase| {
            if test.editor_only {
                return;
            }

            // First time a focused test is encountered, switch to "focused" mode and throw everything away.
            if !is_focus_run && test.focused {
                tests.clear();
                all_files.clear();
                is_focus_run = true;
            }

            // Only collect tests if normal mode, or focus mode and test is focused.
            if !is_focus_run || test.focused {
                all_files.insert(test.file);
                tests.push(*test);
            }
        });

        // Sort alphabetically for deterministic run order
        tests.sort_by_key(|test| test.file);

        (tests, all_files.len(), is_focus_run)
    }
}
#[cfg(test)]
mod call_targets_test;
