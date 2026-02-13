use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context};

use crate::ui::{self, MessageType};

fn current_branch() -> Result<String, anyhow::Error> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .context("Failed to run git rev-parse")?;
    if !output.status.success() {
        bail!("Failed to detect current git branch");
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn resolve_branch(branch: Option<&str>) -> Result<String, anyhow::Error> {
    match branch {
        Some(b) => Ok(b.to_string()),
        None => {
            let b = current_branch()?;
            ui::print_message(MessageType::Info, &format!("Using current branch: '{}'", b));
            Ok(b)
        }
    }
}

fn resolve_run_id(run_id: Option<&str>, branch: &str) -> Result<String, anyhow::Error> {
    match run_id {
        Some(id) => {
            ui::print_message(MessageType::Info, &format!("Using run ID: {}", id));
            Ok(id.to_string())
        }
        None => {
            ui::print_message(
                MessageType::Info,
                &format!("Finding latest successful run on branch '{}'...", branch),
            );
            let id = find_latest_run_id(branch, "runner.yml")?;
            ui::print_message(MessageType::Info, &format!("Found run ID: {}", id));
            Ok(id)
        }
    }
}

fn ensure_gh_available() -> Result<(), anyhow::Error> {
    let gh_version = Command::new("gh").arg("--version").output();
    match gh_version {
        Ok(output) if output.status.success() => {}
        _ => {
            bail!(
                "GitHub CLI (gh) is not installed.\n\
                 Install it from: https://cli.github.com/\n\
                 Then run: gh auth login"
            );
        }
    }

    let auth_status = Command::new("gh")
        .args(["auth", "status"])
        .output()
        .context("Failed to check gh auth status")?;

    if !auth_status.status.success() {
        bail!(
            "GitHub CLI is not authenticated.\n\
             Run: gh auth login"
        );
    }

    Ok(())
}

fn find_latest_run_id(branch: &str, workflow: &str) -> Result<String, anyhow::Error> {
    let output = Command::new("gh")
        .args([
            "run",
            "list",
            "--branch",
            branch,
            "--workflow",
            workflow,
            "--limit",
            "1",
            "--json",
            "databaseId",
        ])
        .output()
        .context("Failed to run gh run list")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Failed to list workflow runs: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let runs: serde_json::Value =
        serde_json::from_str(&stdout).context("Failed to parse gh run list output")?;

    let runs = runs.as_array().context("Expected JSON array from gh")?;
    if runs.is_empty() {
        bail!(
            "No runs found for workflow '{}' on branch '{}'",
            workflow,
            branch
        );
    }

    let run_id = runs[0]["databaseId"]
        .as_u64()
        .context("Failed to extract run ID from JSON")?;

    Ok(run_id.to_string())
}

fn create_temp_dir(prefix: &str) -> Result<PathBuf, anyhow::Error> {
    let mut temp = std::env::temp_dir();
    temp.push(format!("{}-{}", prefix, std::process::id()));
    std::fs::create_dir_all(&temp)
        .with_context(|| format!("Failed to create temp directory: {}", temp.display()))?;
    Ok(temp)
}

fn remove_temp_dir(path: &Path) {
    if path.exists() {
        let _ = std::fs::remove_dir_all(path);
    }
}

fn download_artifact(
    run_id: &str,
    artifact_name: &str,
    dest_dir: &Path,
) -> Result<(), anyhow::Error> {
    let spinner = ui::create_spinner(&format!(
        "Downloading artifact '{}' from run {}...",
        artifact_name, run_id
    ));

    let output = Command::new("gh")
        .args([
            "run",
            "download",
            run_id,
            "--name",
            artifact_name,
            "--dir",
            &dest_dir.to_string_lossy(),
        ])
        .output()
        .context("Failed to run gh run download")?;

    spinner.finish_and_clear();

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "Failed to download artifact '{}': {}",
            artifact_name,
            stderr
        );
    }

    ui::print_message(
        MessageType::Success,
        &format!("Downloaded artifact '{}'", artifact_name),
    );
    Ok(())
}

fn copy_snapshot_files(temp_dir: &Path, mappings: &[(&str, &str)]) -> Result<usize, anyhow::Error> {
    let mut total_copied = 0;

    for (src_relative, dest_relative) in mappings {
        let src_dir = temp_dir.join(src_relative);
        let dest_dir = Path::new(dest_relative);

        if !src_dir.exists() {
            ui::print_message(
                MessageType::Warning,
                &format!("Source directory not found: {}", src_dir.display()),
            );
            continue;
        }

        std::fs::create_dir_all(dest_dir)
            .with_context(|| format!("Failed to create directory: {}", dest_dir.display()))?;

        let entries = std::fs::read_dir(&src_dir)
            .with_context(|| format!("Failed to read directory: {}", src_dir.display()))?;

        let mut count = 0;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("png") {
                let file_name = path.file_name().unwrap();
                let dest_path = dest_dir.join(file_name);
                std::fs::copy(&path, &dest_path).with_context(|| {
                    format!(
                        "Failed to copy {} to {}",
                        path.display(),
                        dest_path.display()
                    )
                })?;
                count += 1;
            }
        }

        if count > 0 {
            ui::print_message(
                MessageType::Info,
                &format!("Copied {} file(s) to {}", count, dest_dir.display()),
            );
        }
        total_copied += count;
    }

    Ok(total_copied)
}

pub fn update_coverage_snapshots(
    run_id: Option<&str>,
    branch: Option<&str>,
) -> Result<(), anyhow::Error> {
    ui::print_section("Update Coverage Snapshots");

    ensure_gh_available()?;

    let branch = resolve_branch(branch)?;
    let resolved_run_id = resolve_run_id(run_id, &branch)?;

    let temp_dir = create_temp_dir("dcl-coverage-snapshots")?;
    let result = (|| {
        download_artifact(&resolved_run_id, "coverage-snapshots", &temp_dir)?;

        let mappings = [
            (
                "tests/snapshots/scenes/comparison",
                "tests/snapshots/scenes",
            ),
            (
                "tests/snapshots/avatar-image-generation/comparison",
                "tests/snapshots/avatar-image-generation",
            ),
            (
                "tests/snapshots/client/comparison",
                "tests/snapshots/client",
            ),
        ];

        let total = copy_snapshot_files(&temp_dir, &mappings)?;

        ui::print_message(
            MessageType::Success,
            &format!(
                "Updated {} snapshot file(s) from run {}",
                total, resolved_run_id
            ),
        );

        Ok(())
    })();

    remove_temp_dir(&temp_dir);
    result
}

pub fn update_docker_snapshots(
    run_id: Option<&str>,
    branch: Option<&str>,
) -> Result<(), anyhow::Error> {
    ui::print_section("Update Docker Snapshots");

    ensure_gh_available()?;

    let branch = resolve_branch(branch)?;
    let resolved_run_id = resolve_run_id(run_id, &branch)?;

    let temp_dir = create_temp_dir("dcl-docker-snapshots")?;
    let result = (|| {
        download_artifact(&resolved_run_id, "docker-snapshots", &temp_dir)?;

        let mappings = [
            ("avatars-output", "tests/snapshots/avatar-image-generation"),
            ("scenes-output", "tests/snapshots/scene-image-generation"),
        ];

        let total = copy_snapshot_files(&temp_dir, &mappings)?;

        ui::print_message(
            MessageType::Success,
            &format!(
                "Updated {} snapshot file(s) from run {}",
                total, resolved_run_id
            ),
        );

        Ok(())
    })();

    remove_temp_dir(&temp_dir);
    result
}
