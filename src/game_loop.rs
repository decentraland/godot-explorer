//! Local driver for the Android **Game Loop** harness (see
//! `godot/src/test/game_loop/`), so a scenario can be run against a USB-connected
//! device with a single `cargo run -- game-loop` — no Firebase Test Lab / Blaze.
//!
//! It reproduces exactly what Test Lab does, over `adb`:
//!   1. (optional) `pm clear` + install the exported debug APK,
//!   2. fire the `com.google.intent.action.TEST_LOOP` intent (integer `scenario`
//!      extra, optional `guest-seed` for the deterministic golden guest),
//!   3. tail logcat for the runner's `[GAMELOOP] RESULT scenario=N status=...` line,
//!   4. pull the `user://gameloop_*.png` screenshots off the device,
//!   5. compare them against the golden baselines (`tests/snapshots/gameloop/`) —
//!      or `--bless` this run's shots as the new baselines.
//!
//! The device stays fixed, so the golden screenshots are pixel-stable run to run
//! (that's the whole reason to use a real device rather than Test Lab's rotating
//! device farm for regression goldens).
//!
//! Split by responsibility: [`options`] (config), [`adb`] (device plumbing),
//! [`launch`] (intent + RESULT), [`screenshots`] (pull), [`golden`] (compare/bless),
//! [`scenario`] (one scenario's lifecycle). This file only fans out over scenarios.

mod adb;
mod golden;
mod launch;
mod options;
mod scenario;
mod screenshots;

use std::path::PathBuf;

use anyhow::anyhow;

use crate::ui::{print_message, print_section, MessageType};

use adb::{ensure_adb, install_apk, resolve_device};

pub use options::GameLoopOptions;

/// App package id — shared by every adb call that targets the app.
const PACKAGE: &str = "org.decentraland.godotexplorer";

/// Scenarios `--scenario all` runs. Mirror of the GDScript `SCENARIOS` registry —
/// keep in sync when a scenario is added there.
pub const ALL_SCENARIOS: &[u32] = &[1, 2, 3, 4, 5];

pub fn run(opts: GameLoopOptions) -> anyhow::Result<()> {
    ensure_adb()?;
    let device = resolve_device(opts.device.as_deref())?;

    // Install once up front, not per scenario.
    if opts.install {
        install_apk(&device)?;
    }

    let multi = opts.scenarios.len() > 1;
    let mut outcomes: Vec<(u32, bool)> = Vec::new();
    for &scenario in &opts.scenarios {
        let (out_dir, golden_dir) = scenario_dirs(&opts, scenario, multi);
        let passed = scenario::run_one(&device, scenario, &opts, &out_dir, &golden_dir)
            .unwrap_or_else(|e| {
                print_message(MessageType::Error, &format!("scenario {scenario}: {e}"));
                false
            });
        outcomes.push((scenario, passed));
    }

    report(&outcomes, multi)
}

/// Where a scenario's pulled shots and golden baselines live. An explicit --out/--golden
/// override is only meaningful (and honored) for a single scenario; a multi-scenario run
/// always derives per-scenario dirs.
fn scenario_dirs(opts: &GameLoopOptions, scenario: u32, multi: bool) -> (PathBuf, PathBuf) {
    let out_dir = opts
        .out_override
        .clone()
        .filter(|_| !multi)
        .unwrap_or_else(|| PathBuf::from(format!("target/gameloop/scenario_{scenario}")));
    let golden_dir = opts
        .golden_override
        .clone()
        .filter(|_| !multi)
        .unwrap_or_else(|| PathBuf::from(format!("tests/snapshots/gameloop/scenario_{scenario}")));
    (out_dir, golden_dir)
}

/// Prints the per-scenario summary (multi-scenario runs) and returns Ok iff all passed.
fn report(outcomes: &[(u32, bool)], multi: bool) -> anyhow::Result<()> {
    if multi {
        print_section("Summary");
        for (scenario, passed) in outcomes {
            let msg = format!(
                "scenario {scenario}: {}",
                if *passed { "PASS" } else { "FAIL" }
            );
            let kind = if *passed {
                MessageType::Success
            } else {
                MessageType::Error
            };
            print_message(kind, &msg);
        }
    }

    let failed: Vec<u32> = outcomes
        .iter()
        .filter(|(_, p)| !p)
        .map(|(s, _)| *s)
        .collect();
    if failed.is_empty() {
        print_message(MessageType::Success, "GAME LOOP PASSED");
        Ok(())
    } else {
        Err(anyhow!("game-loop failed for scenario(s): {failed:?}"))
    }
}
