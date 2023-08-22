/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::prelude::*;

pub mod av;
pub mod avatars;
pub mod comms;
pub mod dcl;
pub mod http_request;
pub mod realm;
pub mod scene_runner;
pub mod session;
pub mod test_runner;
struct DecentralandGodotLibrary;

#[gdextension]
unsafe impl ExtensionLibrary for DecentralandGodotLibrary {}

// Registers all the `#[itest]` tests.
godot::sys::plugin_registry!(__GODOT_ITEST: RustTestCase);

/// Finds all `#[itest]` tests.
fn collect_rust_tests() -> (Vec<RustTestCase>, usize, bool) {
    let mut all_files = std::collections::HashSet::new();
    let mut tests: Vec<RustTestCase> = vec![];
    let mut is_focus_run = false;

    godot::sys::plugin_foreach!(__GODOT_ITEST; |test: &RustTestCase| {
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

pub struct TestContext {
    #[allow(dead_code)]
    scene_tree: Gd<Node>,
}

#[derive(Copy, Clone)]
struct RustTestCase {
    name: &'static str,
    file: &'static str,
    skipped: bool,
    /// If one or more tests are focused, only they will be executed. Helpful for debugging and working on specific features.
    focused: bool,
    #[allow(dead_code)]
    line: u32,
    function: fn(&TestContext),
}
