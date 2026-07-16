//! Golden baseline handling: bless this run's screenshots, or compare against them.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context};

use crate::image_comparison::compare_images_similarity_masked;
use crate::ui::{print_message, print_section, MessageType};

/// Suffix marking a hand-authored mask file (`<golden-stem>_mask.png`) beside a golden.
const MASK_SUFFIX: &str = "_mask.png";

/// Copies this run's screenshots into `golden_dir` as the new baseline.
pub(super) fn bless(pulled: &[PathBuf], golden_dir: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(golden_dir)?;
    for src in pulled {
        let name = src
            .file_name()
            .ok_or_else(|| anyhow!("bad screenshot path {src:?}"))?;
        fs::copy(src, golden_dir.join(name))
            .with_context(|| format!("copying {src:?} into {golden_dir:?}"))?;
    }
    Ok(())
}

/// Compares each golden in `golden_dir` against the matching pulled screenshot in
/// `out_dir` (by filename). A golden with no result, a resolution mismatch, or a
/// similarity below `threshold` fails. Goldens are the source of truth for WHICH
/// screens matter — delete a noisy one (e.g. the live 3D world) to stop comparing it.
/// Returns whether every golden passed. Missing/empty baseline is informational (Ok).
pub(super) fn compare(golden_dir: &Path, out_dir: &Path, threshold: f64) -> anyhow::Result<bool> {
    let goldens = list_pngs(golden_dir);
    if goldens.is_empty() {
        print_message(
            MessageType::Warning,
            &format!(
                "no golden baseline at {} — run with --bless to establish one",
                golden_dir.display()
            ),
        );
        return Ok(true);
    }

    print_section("Golden comparison");
    let mut failures = 0;
    for golden in &goldens {
        if !compare_one(golden, out_dir, threshold) {
            failures += 1;
        }
    }

    if failures == 0 {
        print_message(
            MessageType::Success,
            &format!("all {} golden(s) match", goldens.len()),
        );
    } else {
        print_message(
            MessageType::Error,
            &format!("{failures}/{} golden(s) failed", goldens.len()),
        );
    }
    Ok(failures == 0)
}

/// Compares one golden against its matching pulled shot; logs the outcome and returns
/// whether it passed. If a `<golden-stem>_mask.png` sits next to the golden it is applied,
/// so masked-out (dynamic) regions don't count toward the score.
fn compare_one(golden: &Path, out_dir: &Path, threshold: f64) -> bool {
    let name = golden.file_name().unwrap_or_default().to_string_lossy();
    let candidate = out_dir.join(golden.file_name().unwrap_or_default());
    if !candidate.exists() {
        print_message(
            MessageType::Error,
            &format!("{name}: MISSING (golden has no matching screenshot this run)"),
        );
        return false;
    }
    // Always look for a mask beside the golden; apply it if present, else compare all pixels.
    let mask = mask_for(golden);
    let tag = if mask.is_some() { " (masked)" } else { "" };
    match compare_images_similarity_masked(golden, &candidate, mask.as_deref()) {
        Ok(sim) => {
            let ok = sim >= threshold;
            let msg = format!(
                "{name}: {:.3}% similar (min {:.1}%){tag}",
                sim * 100.0,
                threshold * 100.0
            );
            print_message(
                if ok {
                    MessageType::Success
                } else {
                    MessageType::Error
                },
                &msg,
            );
            ok
        }
        Err(e) => {
            print_message(MessageType::Error, &format!("{name}: {e}"));
            false
        }
    }
}

/// The hand-authored mask path for a golden (`foo.png` -> `foo_mask.png`), if it exists.
fn mask_for(golden: &Path) -> Option<PathBuf> {
    let stem = golden.file_stem()?.to_str()?;
    let mask = golden.with_file_name(format!("{stem}{MASK_SUFFIX}"));
    mask.is_file().then_some(mask)
}

/// Golden PNGs in `dir`, excluding mask files (`*_mask.png`) which pair with them.
fn list_pngs(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = match fs::read_dir(dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("png"))
            .filter(|p| !p.to_string_lossy().ends_with(MASK_SUFFIX))
            .collect(),
        Err(_) => Vec::new(),
    };
    files.sort();
    files
}
