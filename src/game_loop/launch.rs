//! Fire the TEST_LOOP intent and wait for the runner's RESULT line in logcat.

use std::thread::sleep;
use std::time::{Duration, Instant};

use anyhow::bail;

use crate::ui::{create_spinner, print_message, MessageType};

use super::adb::adb;
use super::PACKAGE;

/// Exported activity-alias carrying the TEST_LOOP intent-filter. GodotApp itself is
/// `exported=false`, so an external implicit/explicit TEST_LOOP intent targets this.
const GAME_LOOP_ACTIVITY: &str = "com.godot.game.GameLoopLauncher";
/// The intent action Firebase Test Lab (and this driver) launches the Game Loop with.
const TEST_LOOP_ACTION: &str = "com.google.intent.action.TEST_LOOP";
/// Marker the GDScript runner prints to logcat:
/// `[GAMELOOP] RESULT scenario=N status=PASS|FAIL detail=...`.
const RESULT_MARKER: &str = "RESULT scenario=";

/// Fires the TEST_LOOP intent. `-W` waits until the activity has launched (not until
/// the scenario finishes — that's what the logcat RESULT line is for).
pub(super) fn launch(device: &str, scenario: u32, guest_seed: Option<&str>) -> anyhow::Result<()> {
    let component = format!("{PACKAGE}/{GAME_LOOP_ACTIVITY}");
    let scenario = scenario.to_string();
    let mut args: Vec<&str> = vec![
        "shell",
        "am",
        "start",
        "-W",
        "-a",
        TEST_LOOP_ACTION,
        "-n",
        &component,
        "--ei",
        "scenario",
        &scenario,
    ];
    if let Some(seed) = guest_seed {
        // Stringified on the Kotlin side either way; the runner parses it as an int.
        args.push("--es");
        args.push("guest-seed");
        args.push(seed);
        print_message(
            MessageType::Info,
            &format!("golden mode: guest-seed={seed}"),
        );
    }
    let status = adb(device, &args).status()?;
    if !status.success() {
        bail!("`am start` failed to launch the Game Loop intent");
    }
    print_message(MessageType::Success, "TEST_LOOP intent launched");
    Ok(())
}

/// Polls `adb logcat -d` until the runner's RESULT line appears or `timeout` elapses.
/// Returns `Some((passed, detail))`, or `None` on timeout.
pub(super) fn wait_for_result(device: &str, timeout: Duration) -> Option<(bool, String)> {
    let spinner = create_spinner("Waiting for scenario RESULT...");
    let start = Instant::now();
    let mut found = None;
    while start.elapsed() < timeout {
        if let Ok(out) = adb(device, &["logcat", "-d"]).output() {
            let text = String::from_utf8_lossy(&out.stdout);
            if let Some(r) = parse_result(&text) {
                found = Some(r);
                break;
            }
        }
        sleep(Duration::from_secs(2));
    }
    spinner.finish();
    found
}

/// Extracts `(passed, detail)` from the first `RESULT scenario=...` logcat line.
fn parse_result(log: &str) -> Option<(bool, String)> {
    for line in log.lines() {
        if let Some(idx) = line.find(RESULT_MARKER) {
            let rest = &line[idx..];
            let passed = rest.contains("status=PASS");
            let detail = rest
                .split_once("detail=")
                .map(|(_, d)| d.trim().to_string())
                .unwrap_or_default();
            return Some((passed, detail));
        }
    }
    None
}
