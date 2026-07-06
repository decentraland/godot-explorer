//! One scenario's lifecycle: prep → launch → RESULT → pull → compare/bless.

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::ui::{print_message, print_section, MessageType};

use super::adb::prepare_scenario;
use super::golden::{bless, compare};
use super::launch::{launch, wait_for_result};
use super::screenshots::pull_screenshots;
use super::GameLoopOptions;

/// Runs one scenario end to end. Returns whether it passed overall (scenario PASS
/// AND — unless blessing — goldens match).
pub(super) fn run_one(
    device: &str,
    scenario: u32,
    opts: &GameLoopOptions,
    out_dir: &Path,
    golden_dir: &Path,
) -> anyhow::Result<bool> {
    print_section(&format!(
        "Game Loop · scenario {scenario} · device {device}"
    ));

    prepare_scenario(device, opts.clear);
    launch(device, scenario, opts.guest_seed.as_deref())?;

    let result = wait_for_result(device, opts.timeout);
    log_result(&result, opts.timeout);

    // Always try to pull whatever screenshots exist — useful even on failure.
    let pulled = pull_screenshots(device, out_dir).unwrap_or_else(|e| {
        print_message(MessageType::Warning, &format!("could not pull shots: {e}"));
        Vec::new()
    });
    if !pulled.is_empty() {
        print_message(
            MessageType::Info,
            &format!(
                "pulled {} screenshot(s) → {}",
                pulled.len(),
                out_dir.display()
            ),
        );
    }

    let scenario_passed = matches!(result, Some((true, _)));

    if opts.bless {
        return bless_run(scenario_passed, &pulled, golden_dir);
    }

    let compare_ok = compare(golden_dir, out_dir, opts.threshold)?;
    Ok(scenario_passed && compare_ok)
}

/// Logs the scenario's RESULT (or a timeout notice).
fn log_result(result: &Option<(bool, String)>, timeout: Duration) {
    match result {
        Some((true, detail)) => {
            print_message(MessageType::Success, &format!("scenario PASS — {detail}"))
        }
        Some((false, detail)) => {
            print_message(MessageType::Error, &format!("scenario FAIL — {detail}"))
        }
        None => print_message(
            MessageType::Error,
            &format!(
                "no RESULT after {}s (scenario hung or never launched)",
                timeout.as_secs()
            ),
        ),
    }
}

/// `--bless` path: only bless a passing run that actually produced screenshots.
fn bless_run(scenario_passed: bool, pulled: &[PathBuf], golden_dir: &Path) -> anyhow::Result<bool> {
    if !scenario_passed {
        print_message(MessageType::Warning, "not blessing: scenario did not pass");
        return Ok(false);
    }
    if pulled.is_empty() {
        print_message(MessageType::Warning, "not blessing: no screenshots pulled");
        return Ok(false);
    }
    bless(pulled, golden_dir)?;
    print_message(
        MessageType::Success,
        &format!(
            "blessed {} baseline(s) → {}",
            pulled.len(),
            golden_dir.display()
        ),
    );
    Ok(true)
}
