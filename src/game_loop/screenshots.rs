//! Pull the scenario's screenshots off the device.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::bail;

use crate::ui::{print_message, MessageType};

use super::adb::adb;
use super::PACKAGE;

/// Scenarios save screenshots to `user://gameloop_<name>.png`; on Android `user://`
/// is the app-internal `files/` dir (pulled via `run-as`, debug APK is debuggable).
const SHOT_PREFIX: &str = "gameloop_";

/// Pulls every `files/gameloop_*.png` off the device into `out_dir`, stripping the
/// `gameloop_` prefix locally. Uses `run-as` (works because the debug APK is
/// debuggable). Returns the local paths, sorted.
pub(super) fn pull_screenshots(device: &str, out_dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    fs::create_dir_all(out_dir)?;
    // Clear stale PNGs from a previous run so a screen skipped this run (or renamed by a
    // varying sequence) can't linger and pollute the golden comparison.
    for entry in fs::read_dir(out_dir)?.flatten() {
        let p = entry.path();
        if p.extension().and_then(|x| x.to_str()) == Some("png") {
            let _ = fs::remove_file(&p);
        }
    }
    let ls = adb(device, &["exec-out", "run-as", PACKAGE, "ls", "files"]).output()?;
    if !ls.status.success() {
        bail!(
            "`run-as {PACKAGE} ls files` failed — is a *debug* (debuggable) APK installed? {}",
            String::from_utf8_lossy(&ls.stderr).trim()
        );
    }
    let names: Vec<String> = String::from_utf8_lossy(&ls.stdout)
        .split_whitespace()
        .filter(|n| n.starts_with(SHOT_PREFIX) && n.ends_with(".png"))
        .map(String::from)
        .collect();

    let mut pulled = Vec::new();
    for name in names {
        let out = adb(
            device,
            &[
                "exec-out",
                "run-as",
                PACKAGE,
                "cat",
                &format!("files/{name}"),
            ],
        )
        .output()?;
        if !out.status.success() || out.stdout.is_empty() {
            print_message(MessageType::Warning, &format!("failed to read {name}"));
            continue;
        }
        let local = name.strip_prefix(SHOT_PREFIX).unwrap_or(&name);
        let dest = out_dir.join(local);
        fs::write(&dest, &out.stdout)?;
        pulled.push(dest);
    }
    pulled.sort();
    Ok(pulled)
}
