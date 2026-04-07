use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use base64::{engine::general_purpose::STANDARD, Engine};

use crate::consts::RUST_LIB_PROJECT_FOLDER;
use crate::helpers::BinPaths;
use crate::image_comparison::compare_images_similarity;
use crate::ui::{self, format_duration, MessageType, SummaryRow};
use crate::{check_gdscript, run, tests, version_check};

#[derive(Clone, Debug)]
#[allow(dead_code)]
enum StepStatus {
    Pass,
    Fail(String),
    Skip(String),
}

struct StepResult {
    name: String,
    duration: Duration,
    status: StepStatus,
}

impl StepResult {
    fn to_summary_row(&self) -> SummaryRow {
        let (duration_str, passed) = match &self.status {
            StepStatus::Pass => (format_duration(self.duration), Some(true)),
            StepStatus::Fail(_) => (format_duration(self.duration), Some(false)),
            StepStatus::Skip(_) => ("--".to_string(), None),
        };
        SummaryRow {
            name: self.name.clone(),
            duration: duration_str,
            passed,
        }
    }
}

/// Tracks test steps, handles continue-on-failure logic, and produces the summary.
struct StepRunner {
    results: Vec<StepResult>,
    continue_on_failure: bool,
    start: Instant,
}

/// Returned by `StepRunner::step` to signal whether execution should continue.
enum StepOutcome {
    Continue,
    Abort,
}

impl StepRunner {
    fn new(continue_on_failure: bool) -> Self {
        Self {
            results: Vec::new(),
            continue_on_failure,
            start: Instant::now(),
        }
    }

    /// Run a step, record its result. Returns `Abort` if the step failed and
    /// `continue_on_failure` is false.
    fn step<F>(&mut self, name: &str, f: F) -> StepOutcome
    where
        F: FnOnce() -> Result<(), anyhow::Error>,
    {
        ui::print_section(name);
        let start = Instant::now();
        let status = match f() {
            Ok(()) => StepStatus::Pass,
            Err(e) => StepStatus::Fail(format!("{}", e)),
        };
        let duration = start.elapsed();

        match &status {
            StepStatus::Pass => ui::print_message(
                MessageType::Success,
                &format!("{} completed in {}", name, format_duration(duration)),
            ),
            StepStatus::Fail(msg) => ui::print_message(
                MessageType::Error,
                &format!("{} failed in {}: {}", name, format_duration(duration), msg),
            ),
            StepStatus::Skip(_) => {}
        }

        let passed = matches!(status, StepStatus::Pass);
        self.results.push(StepResult {
            name: name.to_string(),
            duration,
            status,
        });

        if !passed && !self.continue_on_failure {
            StepOutcome::Abort
        } else {
            StepOutcome::Continue
        }
    }

    /// Record a skipped step.
    fn skip(&mut self, name: &str, reason: &str) {
        ui::print_message(
            MessageType::Warning,
            &format!("{} skipped: {}", name, reason),
        );
        self.results.push(StepResult {
            name: name.to_string(),
            duration: Duration::ZERO,
            status: StepStatus::Skip(reason.to_string()),
        });
    }

    fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    fn print_summary(&self) {
        let rows: Vec<SummaryRow> = self.results.iter().map(|r| r.to_summary_row()).collect();
        ui::print_summary_table(&rows, self.elapsed());
    }

    fn has_failures(&self) -> bool {
        self.results
            .iter()
            .any(|r| matches!(r.status, StepStatus::Fail(_)))
    }

    fn abort_error(&self) -> anyhow::Error {
        let failed: Vec<String> = self
            .results
            .iter()
            .filter_map(|r| {
                if let StepStatus::Fail(msg) = &r.status {
                    Some(format!("  - {}: {}", r.name, msg))
                } else {
                    None
                }
            })
            .collect();
        anyhow::anyhow!("Failed steps:\n{}", failed.join("\n"))
    }
}

/// Convenience macro to run a step and early-return on abort.
macro_rules! step {
    ($runner:expr, $name:expr, $body:expr) => {
        if let StepOutcome::Abort = $runner.step($name, $body) {
            $runner.print_summary();
            return Err($runner.abort_error());
        }
    };
}

/// Like `step!` but tolerates failure when `$tolerate` is true (e.g. update_snapshots mode).
macro_rules! step_tolerant {
    ($runner:expr, $name:expr, $tolerate:expr, $body:expr) => {
        if let StepOutcome::Abort = $runner.step($name, $body) {
            if !$tolerate {
                $runner.print_summary();
                return Err($runner.abort_error());
            }
        }
    };
}

/// Run an external command, returning Ok if exit code is 0.
fn run_external_command(
    program: &str,
    args: &[&str],
    working_dir: Option<&str>,
) -> Result<(), anyhow::Error> {
    let mut cmd = std::process::Command::new(program);
    cmd.args(args);
    if let Some(dir) = working_dir {
        cmd.current_dir(dir);
    }

    // Set PROTOC if available (needed for cargo check/clippy/test)
    let protoc_path = BinPaths::protoc_bin();
    if protoc_path.exists() {
        if let Ok(canonical) = std::fs::canonicalize(&protoc_path) {
            cmd.env("PROTOC", canonical.to_string_lossy().to_string());
        }
    }

    let status = cmd.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "Command `{} {}` exited with status: {}",
            program,
            args.join(" "),
            status
        ))
    }
}

const GDTOOLKIT_FORK_URL: &str =
    "https://github.com/dcl-regenesislabs/godot-gdscript-toolkit.git";
const GDTOOLKIT_VENV_DIR: &str = ".bin/gdtoolkit-venv";

/// Get the path to gdformat/gdlint binaries inside the local venv.
fn gdtoolkit_bin(tool: &str) -> String {
    format!("{}/bin/{}", GDTOOLKIT_VENV_DIR, tool)
}

/// Ensure the correct (forked) version of gdtoolkit is installed in .bin/gdtoolkit-venv/.
fn ensure_gdtoolkit() -> Result<(), anyhow::Error> {
    let venv_dir = Path::new(GDTOOLKIT_VENV_DIR);
    let gdformat_bin = venv_dir.join("bin/gdformat");

    if gdformat_bin.exists() {
        ui::print_message(
            MessageType::Success,
            "gdtoolkit fork already installed in .bin/gdtoolkit-venv/",
        );
        return Ok(());
    }

    ui::print_message(
        MessageType::Info,
        "Installing DCL fork of gdtoolkit into .bin/gdtoolkit-venv/...",
    );

    let status = std::process::Command::new("python3")
        .args(["-m", "venv", GDTOOLKIT_VENV_DIR])
        .status()?;
    if !status.success() {
        return Err(anyhow::anyhow!(
            "Failed to create Python venv at {}",
            GDTOOLKIT_VENV_DIR
        ));
    }

    let pip_bin = format!("{}/bin/pip", GDTOOLKIT_VENV_DIR);
    let status = std::process::Command::new(&pip_bin)
        .args(["install", &format!("git+{}", GDTOOLKIT_FORK_URL)])
        .status()?;
    if !status.success() {
        let _ = std::fs::remove_dir_all(venv_dir);
        return Err(anyhow::anyhow!(
            "Failed to install gdtoolkit fork into venv"
        ));
    }

    ui::print_message(
        MessageType::Success,
        "gdtoolkit fork installed in .bin/gdtoolkit-venv/",
    );
    Ok(())
}

/// Copy comparison output images back to snapshot baseline directories.
fn update_local_snapshots() -> Result<(), anyhow::Error> {
    ui::print_section("Updating Snapshots");

    let mut total_copied = 0;

    for (base_dir, _) in SNAPSHOT_DIRS {
        let comparison_dir = Path::new(base_dir).join("comparison");
        if !comparison_dir.exists() {
            continue;
        }

        let mut count = 0;
        for entry in fs::read_dir(&comparison_dir)?.flatten() {
            let path = entry.path();
            let is_png = path.extension().and_then(|e| e.to_str()) == Some("png");
            let is_diff = path
                .file_name()
                .and_then(|f| f.to_str())
                .map_or(false, |f| f.ends_with(".diff.png"));

            if is_png && !is_diff {
                let dest_path = Path::new(base_dir).join(path.file_name().unwrap());
                fs::copy(&path, &dest_path)?;
                count += 1;
            }
        }

        if count > 0 {
            ui::print_message(
                MessageType::Info,
                &format!("Updated {} snapshot(s) in {}", count, base_dir),
            );
        }
        total_copied += count;
    }

    if total_copied > 0 {
        ui::print_message(
            MessageType::Success,
            &format!("Updated {} total snapshot file(s)", total_copied),
        );
    } else {
        ui::print_message(
            MessageType::Warning,
            "No comparison images found to update. Run visual tests first.",
        );
    }

    Ok(())
}

pub fn run_full_tests(
    continue_on_failure: bool,
    skip_visual: bool,
    update_snapshots: bool,
    report: bool,
) -> Result<(), anyhow::Error> {
    if update_snapshots && skip_visual {
        return Err(anyhow::anyhow!(
            "--update-snapshots requires visual tests to run. Remove --skip-visual."
        ));
    }

    let mut runner = StepRunner::new(continue_on_failure);
    let lib_dir = Some(RUST_LIB_PROJECT_FOLDER);

    // ── Setup: ensure gdtoolkit fork ──
    ensure_gdtoolkit()?;

    // ── Phase 1: Static Checks ──

    ui::print_message(MessageType::Step, "Phase 1: Static Checks");

    let gdformat_bin = gdtoolkit_bin("gdformat");
    let gdlint_bin = gdtoolkit_bin("gdlint");

    step!(runner, "Cargo fmt", || {
        run_external_command("cargo", &["fmt", "--all", "--", "--check"], lib_dir)
    });
    step!(runner, "GDScript format", || {
        run_external_command(&gdformat_bin, &["-d", "godot/"], None)
    });
    step!(runner, "GDScript lint", || {
        run_external_command(&gdlint_bin, &["godot/"], None)
    });
    step!(runner, "Cargo check", || {
        run_external_command("cargo", &["check"], lib_dir)
    });
    step!(runner, "Cargo clippy", || {
        run_external_command("cargo", &["clippy", "--", "-D", "warnings"], lib_dir)
    });
    step!(runner, "Asset import check", || {
        run_external_command("python3", &["tests/check_asset_imports.py"], None)
    });
    step!(runner, "Version check", version_check::run_version_check);

    // ── Phase 2: Rust Unit Tests ──

    ui::print_message(MessageType::Step, "Phase 2: Rust Unit Tests");

    step!(runner, "Rust unit tests", || {
        run_external_command("cargo", &["test", "--", "--skip", "auth"], lib_dir)
    });

    // ── Phase 3: Build & Godot Tests ──

    ui::print_message(MessageType::Step, "Phase 3: Build & Godot Tests");

    step!(runner, "Build lib", || {
        run::build(false, false, vec![], None, None)
    });
    step!(runner, "Import assets", || {
        let status = crate::export::import_assets();
        if status.success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "import-assets exited with status: {}",
                status
            ))
        }
    });
    step!(runner, "GDScript validation", check_gdscript::check_gdscript);
    step!(runner, "Integration tests", || {
        run::run(false, true, vec![], false, false, false)
    });

    // ── Phase 4: Visual Tests ──

    if skip_visual {
        ui::print_message(MessageType::Step, "Phase 4: Visual Tests (skipped)");
        runner.skip("Client tests", "--skip-visual");
        runner.skip("Avatar generation", "--skip-visual");
        runner.skip("Scene generation", "--skip-visual");
    } else {
        ui::print_message(MessageType::Step, "Phase 4: Visual Tests");

        // Clean stale comparison dirs to avoid leftover files from previous runs
        for (dir, _) in SNAPSHOT_DIRS {
            let comparison_dir = Path::new(dir).join("comparison");
            if comparison_dir.exists() {
                let _ = fs::remove_dir_all(&comparison_dir);
            }
        }

        let client_snapshot_folder = Path::new("./tests/snapshots/client").canonicalize()?;
        let client_snapshot_str = client_snapshot_folder.to_string_lossy().to_string();
        let tolerate = update_snapshots;

        step_tolerant!(runner, "Client tests", tolerate, || {
            let extra_args = vec![
                "--snapshot-folder".to_string(),
                client_snapshot_str.clone(),
            ];
            run::run(false, false, extra_args, false, true, false)
        });

        step_tolerant!(runner, "Avatar generation", tolerate, || {
            tests::test_avatar_generation(None)
        });

        step_tolerant!(runner, "Scene generation", tolerate, || {
            tests::test_scene_generation(None)
        });

        if update_snapshots {
            step!(runner, "Update snapshots", update_local_snapshots);
        }
    }

    // ── Summary ──

    runner.print_summary();

    if report {
        if let Err(e) = generate_html_report(&runner.results, runner.elapsed()) {
            ui::print_message(
                MessageType::Warning,
                &format!("Failed to generate report: {}", e),
            );
        }
    }

    if runner.has_failures() {
        Err(runner.abort_error())
    } else {
        Ok(())
    }
}

// ── HTML Report Generation ──

const SNAPSHOT_DIRS: &[(&str, &str)] = &[
    (
        "tests/snapshots/avatar-image-generation",
        "Avatar Image Generation",
    ),
    (
        "tests/snapshots/scene-image-generation",
        "Scene Image Generation",
    ),
    ("tests/snapshots/scenes", "Scene Tests"),
    ("tests/snapshots/client", "Client Tests"),
];

struct SnapshotComparison {
    name: String,
    category: String,
    baseline_path: PathBuf,
    comparison_path: PathBuf,
    similarity: Option<f64>,
    error_msg: Option<String>,
    passed: bool,
}

/// Scan snapshot directories for baseline vs comparison pairs.
fn collect_snapshot_comparisons() -> Vec<SnapshotComparison> {
    let mut comparisons = Vec::new();

    for (dir, category) in SNAPSHOT_DIRS {
        let comparison_dir = Path::new(dir).join("comparison");
        if !comparison_dir.exists() {
            continue;
        }

        let entries = match fs::read_dir(&comparison_dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let is_png = path.extension().and_then(|e| e.to_str()) == Some("png");
            let is_diff = path
                .file_name()
                .and_then(|f| f.to_str())
                .map_or(false, |f| f.ends_with(".diff.png"));

            if !is_png || is_diff {
                continue;
            }

            let file_name = path.file_name().unwrap();
            let baseline_path = Path::new(dir).join(file_name);

            let (similarity, error_msg) = if baseline_path.exists() {
                match compare_images_similarity(&baseline_path, &path) {
                    Ok(s) => (Some(s), None),
                    Err(e) => (None, Some(e)),
                }
            } else {
                (None, Some("Missing baseline".to_string()))
            };

            comparisons.push(SnapshotComparison {
                name: file_name.to_string_lossy().to_string(),
                category: category.to_string(),
                baseline_path,
                comparison_path: path,
                similarity,
                error_msg,
                passed: similarity.map_or(false, |s| s >= 0.90),
            });
        }
    }

    comparisons
}

/// Encode a PNG file as a base64 data URI, or return an SVG placeholder.
fn image_to_data_uri(path: &Path) -> String {
    if !path.exists() {
        return "data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='512' height='512'><rect width='512' height='512' fill='%23333'/><text x='50%25' y='50%25' fill='%23999' font-size='24' text-anchor='middle' dy='.3em'>No image</text></svg>".to_string();
    }
    match fs::read(path) {
        Ok(bytes) => format!("data:image/png;base64,{}", STANDARD.encode(&bytes)),
        Err(_) => String::new(),
    }
}

fn generate_html_report(
    results: &[StepResult],
    total_duration: Duration,
) -> Result<(), anyhow::Error> {
    ui::print_section("Generating HTML Report");

    let comparisons = collect_snapshot_comparisons();

    // Build the steps table rows
    let mut steps_html = String::new();
    for r in results {
        let (status_class, status_text, duration_text) = match &r.status {
            StepStatus::Pass => ("pass", "PASS", format_duration(r.duration)),
            StepStatus::Fail(_) => ("fail", "FAIL", format_duration(r.duration)),
            StepStatus::Skip(_) => ("skip", "SKIP", "--".to_string()),
        };
        let error_detail = if let StepStatus::Fail(msg) = &r.status {
            format!(
                "<div class=\"error-detail\">{}</div>",
                html_escape(msg)
            )
        } else {
            String::new()
        };
        steps_html.push_str(&format!(
            "<tr class=\"{}\"><td>{}</td><td>{}</td><td class=\"status\">{}</td></tr>{}\n",
            status_class,
            html_escape(&r.name),
            duration_text,
            status_text,
            error_detail
        ));
    }

    // Build snapshot comparison cards
    let mut snapshots_html = String::new();
    if comparisons.is_empty() {
        snapshots_html
            .push_str("<p class=\"muted\">No snapshot comparisons found. Run visual tests first.</p>");
    } else {
        for c in &comparisons {
            let status_class = if c.passed { "pass" } else { "fail" };
            let similarity_pct = match (&c.similarity, &c.error_msg) {
                (Some(s), _) => format!("{:.2}%", s * 100.0),
                (None, Some(e)) => html_escape(e),
                (None, None) => "Unknown".to_string(),
            };
            let baseline_uri = image_to_data_uri(&c.baseline_path);
            let comparison_uri = image_to_data_uri(&c.comparison_path);
            let open_attr = if c.passed { "" } else { " open" };

            snapshots_html.push_str(&format!(
                r#"<details class="snapshot-card {status_class}"{open_attr}><summary class="snapshot-header">
    <span class="snapshot-name">{name}</span>
    <span class="snapshot-category">{category}</span>
    <span class="snapshot-similarity">Similarity: {similarity}</span>
  </summary>
  <div class="snapshot-images">
    <div class="snapshot-img">
      <div class="img-label">Baseline</div>
      <img src="{baseline}" />
    </div>
    <div class="snapshot-img">
      <div class="img-label">New</div>
      <img src="{comparison}" />
    </div>
  </div>
</details>
"#,
                status_class = status_class,
                open_attr = open_attr,
                name = html_escape(&c.name),
                category = html_escape(&c.category),
                similarity = similarity_pct,
                baseline = baseline_uri,
                comparison = comparison_uri,
            ));
        }
    }

    let has_failures = results
        .iter()
        .any(|r| matches!(r.status, StepStatus::Fail(_)));
    let overall_status = if has_failures { "FAIL" } else { "PASS" };
    let overall_class = if has_failures { "fail" } else { "pass" };

    let html = format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>Full Tests Report</title>
<style>
  * {{ margin: 0; padding: 0; box-sizing: border-box; }}
  body {{ font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, monospace;
         background: #1a1a2e; color: #e0e0e0; padding: 2rem; }}
  h1 {{ color: #fff; margin-bottom: 0.5rem; }}
  h2 {{ color: #ccc; margin: 2rem 0 1rem; border-bottom: 1px solid #333; padding-bottom: 0.5rem; }}
  .overall {{ font-size: 1.2rem; margin-bottom: 2rem; }}
  .overall .status {{ font-weight: bold; padding: 0.2rem 0.6rem; border-radius: 4px; }}
  .overall .pass {{ background: #1b4332; color: #52b788; }}
  .overall .fail {{ background: #461220; color: #e5383b; }}
  table {{ width: 100%; border-collapse: collapse; margin-bottom: 2rem; }}
  th {{ text-align: left; padding: 0.6rem 1rem; background: #16213e; color: #aaa;
       border-bottom: 2px solid #333; }}
  td {{ padding: 0.5rem 1rem; border-bottom: 1px solid #2a2a3e; }}
  tr.pass td.status {{ color: #52b788; font-weight: bold; }}
  tr.fail td.status {{ color: #e5383b; font-weight: bold; }}
  tr.skip td.status {{ color: #f4a261; font-weight: bold; }}
  tr.fail {{ background: #1c0f13; }}
  .error-detail {{ padding: 0.4rem 1rem; color: #e5383b; font-size: 0.85rem;
                   background: #1c0f13; border-left: 3px solid #e5383b; margin: 0.2rem 0; white-space: pre-wrap; word-break: break-all; }}
  .muted {{ color: #666; font-style: italic; }}
  .snapshot-card {{ border: 1px solid #333; border-radius: 8px; margin-bottom: 1.5rem;
                    overflow: hidden; }}
  .snapshot-card.fail {{ border-color: #e5383b; }}
  .snapshot-card.pass {{ border-color: #52b788; }}
  .snapshot-header {{ display: flex; gap: 1rem; align-items: center; padding: 0.8rem 1rem;
                      background: #16213e; cursor: pointer; list-style: none; }}
  .snapshot-header::-webkit-details-marker {{ display: none; }}
  .snapshot-header::before {{ content: "\25B6"; font-size: 0.7rem; color: #888; transition: transform 0.2s; }}
  details[open] > .snapshot-header::before {{ transform: rotate(90deg); }}
  .snapshot-name {{ font-weight: bold; flex: 1; }}
  .snapshot-category {{ color: #888; font-size: 0.85rem; }}
  .snapshot-similarity {{ font-size: 0.85rem; }}
  .snapshot-card.fail .snapshot-similarity {{ color: #e5383b; }}
  .snapshot-card.pass .snapshot-similarity {{ color: #52b788; }}
  .snapshot-images {{ display: flex; gap: 0; }}
  .snapshot-img {{ flex: 1; text-align: center; padding: 0.5rem; background: #111; }}
  .snapshot-img img {{ max-width: 100%; height: auto; border: 1px solid #333; }}
  .img-label {{ font-size: 0.75rem; color: #888; margin-bottom: 0.3rem; text-transform: uppercase; }}
  .timestamp {{ color: #555; font-size: 0.85rem; margin-top: 2rem; }}
</style>
</head>
<body>
<h1>Full Tests Report</h1>
<div class="overall">
  Total: {total_duration} &mdash; <span class="status {overall_class}">{overall_status}</span>
</div>

<h2>Test Steps</h2>
<table>
<tr><th>Step</th><th>Duration</th><th>Status</th></tr>
{steps_html}
</table>

<h2>Snapshot Comparisons</h2>
{snapshots_html}

<p class="timestamp">Generated: {timestamp}</p>
</body>
</html>"##,
        total_duration = format_duration(total_duration),
        overall_status = overall_status,
        overall_class = overall_class,
        steps_html = steps_html,
        snapshots_html = snapshots_html,
        timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
    );

    // Write report
    let report_dir = Path::new(".bin/reports");
    fs::create_dir_all(report_dir)?;
    let report_path = report_dir.join("full-tests-report.html");
    fs::write(&report_path, html)?;

    let abs_path = fs::canonicalize(&report_path)?;
    ui::print_message(
        MessageType::Success,
        &format!("Report saved to {}", abs_path.display()),
    );

    open_in_browser(&abs_path);
    Ok(())
}

fn open_in_browser(path: &Path) {
    let path_str = path.to_string_lossy();
    let result = if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(&*path_str).status()
    } else if cfg!(target_os = "linux") {
        std::process::Command::new("xdg-open")
            .arg(&*path_str)
            .status()
    } else if cfg!(target_os = "windows") {
        std::process::Command::new("cmd")
            .args(["/C", "start", &path_str])
            .status()
    } else {
        return;
    };

    if let Ok(status) = result {
        if status.success() {
            ui::print_message(MessageType::Info, "Report opened in browser");
        }
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
