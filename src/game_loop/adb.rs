//! adb device plumbing for the Game Loop driver: locate `adb`, pick the device,
//! install the APK, and do the per-scenario device prep.

use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context};

use crate::consts::EXPORTS_FOLDER;
use crate::ui::{create_spinner, print_message, MessageType};

use super::PACKAGE;

/// Builds an `adb -s <device> <args...>` command.
pub(super) fn adb(device: &str, args: &[&str]) -> Command {
    let mut c = Command::new("adb");
    c.arg("-s").arg(device);
    c.args(args);
    c
}

/// Errors unless `adb` is on PATH and runnable.
pub(super) fn ensure_adb() -> anyhow::Result<()> {
    let ok = Command::new("adb")
        .arg("version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !ok {
        bail!("adb not found. Install the Android SDK platform-tools and put adb on PATH");
    }
    Ok(())
}

/// Resolves the target device: the explicit serial if given (and connected), else
/// the single connected device. Errors when none are connected.
pub(super) fn resolve_device(explicit: Option<&str>) -> anyhow::Result<String> {
    let out = Command::new("adb")
        .arg("devices")
        .output()
        .context("failed to run `adb devices`")?;
    let text = String::from_utf8_lossy(&out.stdout);
    let devices: Vec<String> = text
        .lines()
        .skip(1) // "List of devices attached"
        .filter_map(|line| {
            let mut it = line.split_whitespace();
            match (it.next(), it.next()) {
                (Some(serial), Some("device")) => Some(serial.to_string()),
                _ => None,
            }
        })
        .collect();

    if let Some(want) = explicit {
        if devices.iter().any(|d| d == want) {
            return Ok(want.to_string());
        }
        bail!("device '{want}' is not connected (adb devices: {devices:?})");
    }

    match devices.as_slice() {
        [] => bail!("no Android device connected (check `adb devices` and USB debugging)"),
        [single] => Ok(single.clone()),
        many => {
            print_message(
                MessageType::Warning,
                &format!(
                    "{} devices connected, using the first ({}). Pass --device to choose.",
                    many.len(),
                    many[0]
                ),
            );
            Ok(many[0].clone())
        }
    }
}

/// Installs (`-r`) the exported debug APK onto `device`.
pub(super) fn install_apk(device: &str) -> anyhow::Result<()> {
    let apk = format!("{EXPORTS_FOLDER}decentraland.godot.client.apk");
    if !Path::new(&apk).exists() {
        bail!("APK not found at {apk} — run `cargo run -- export --target android --format apk` first");
    }
    let spinner = create_spinner("Installing APK...");
    let status = adb(device, &["install", "-r", &apk]).status()?;
    spinner.finish();
    if !status.success() {
        bail!("adb install failed");
    }
    print_message(MessageType::Success, "APK installed");
    Ok(())
}

/// Per-scenario device prep, all best-effort:
///   * optional `pm clear` (fresh guest / clean state),
///   * (re)grant POST_NOTIFICATIONS so the OS dialog doesn't steal focus mid-flow
///     (no-op / harmless below API 33),
///   * wipe last run's screenshots so we only pull (and compare) shots THIS run
///     produced — a screen skipped this run (e.g. AVATAR_CREATE on the COMEBACK path)
///     must not be silently satisfied by a stale file. Keeps the rest of app state
///     (guest identity) so the seeded COMEBACK flow stays deterministic,
///   * clear logcat so we read only this run's RESULT line.
pub(super) fn prepare_scenario(device: &str, clear: bool) {
    if clear {
        print_message(MessageType::Step, "pm clear (fresh app state)");
        let _ = adb(device, &["shell", "pm", "clear", PACKAGE]).status();
    }
    let _ = adb(
        device,
        &[
            "shell",
            "pm",
            "grant",
            PACKAGE,
            "android.permission.POST_NOTIFICATIONS",
        ],
    )
    .status();
    let _ = adb(
        device,
        &[
            "exec-out",
            "run-as",
            PACKAGE,
            "sh",
            "-c",
            "rm -f files/gameloop_*.png",
        ],
    )
    .status();
    let _ = adb(device, &["logcat", "-c"]).status();
}
