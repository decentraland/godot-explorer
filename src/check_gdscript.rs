use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::ExitStatus;

use anyhow::{bail, Context, Result};
use walkdir::WalkDir;
use xtaskops::ops::cmd;

use crate::{
    consts::GODOT_PROJECT_FOLDER,
    path::get_godot_path,
    ui::{print_message, print_section, MessageType},
};

/// Run headless Godot, streaming its output through to our own stdout while
/// keeping a copy so the caller can look for a success marker in it.
///
/// stderr is folded into stdout so the copy holds everything the CI log shows,
/// in the order it was written. `program` is a parameter rather than a call to
/// [`get_godot_path`] so tests can drive this with a stand-in for Godot.
fn run_headless(program: String, args: Vec<String>) -> Result<(String, ExitStatus)> {
    let reader = cmd(program, args)
        .stderr_to_stdout()
        // Without this, the last `read` below fails instead of reporting EOF.
        .unchecked()
        .reader()?;

    let mut child_output = BufReader::new(&reader);
    let mut log = String::new();
    let mut line = Vec::new();
    let stdout = io::stdout();
    let mut sink = stdout.lock();
    loop {
        line.clear();
        if child_output.read_until(b'\n', &mut line)? == 0 {
            break;
        }
        let text = String::from_utf8_lossy(&line);
        write!(sink, "{text}")?;
        sink.flush()?;
        log.push_str(&text);
    }

    let status = reader
        .try_wait()?
        .context("headless Godot reached EOF without exiting")?
        .status;
    Ok((log, status))
}

/// Decide whether a headless run passed, working around a Godot 4.6.2 crash on
/// the way out.
///
/// Every headless test here runs as a `SceneTree` script over the real project,
/// so Godot boots all 15 autoloads (network, threads, `CanvasItem`s) and then
/// races them during teardown: it leaks RIDs and ObjectDB instances and dies
/// with SIGSEGV — sometimes surfacing as SIGABRT from its own crash handler —
/// *after* the test has already printed its result. Between 2026-08-15 and
/// 2026-09-10 that turned the Linux build red 36 times with nothing wrong in the
/// commit; of 32 such crashes with logs still retained, 30 came after the
/// success marker.
///
/// The real fix is to stop booting the autoload stack for these tests. Until
/// then, trust the marker over the exit status — but only for a death by
/// signal, since a genuinely failing test prints `[<name>] FAIL: n case(s)` and
/// quits with code 1 without ever printing the marker.
fn headless_result(label: &str, log: &str, status: ExitStatus, success_marker: &str) -> Result<()> {
    // A script that did not compile still reaches its final print, and a counter nothing
    // incremented still reads zero, so the marker alone can report a test whose every
    // assertion errored as a pass.
    for fatal in ["Compile Error:", "Failed to load script"] {
        if log.contains(fatal) {
            print_message(
                MessageType::Error,
                &format!("{label}: script did not compile — any PASS it printed is meaningless"),
            );
            return Err(anyhow::anyhow!("{label} failed to compile"));
        }
    }

    // The marker, not the exit status, is what says the test ran: Godot exits 0 after a
    // script fails to parse, so trusting the status alone reports a test that never ran
    // as a pass.
    if log.contains(success_marker) {
        if status.success() {
            return Ok(());
        }

        // `code()` is `None` only when a signal killed the process, which cannot
        // happen on Windows, so this stays false there.
        if status.code().is_none() {
            print_message(
                MessageType::Warning,
                &format!(
                    "{label} passed but Godot crashed on the way out ({status}) — \
                     treating as success (engine teardown crash, not a test failure)"
                ),
            );
            return Ok(());
        }
    }

    Err(anyhow::anyhow!("{label} failed ({status})"))
}

pub fn check_gdscript() -> Result<()> {
    print_section("GDScript Validation");

    let godot_bin = get_godot_path();
    print_message(MessageType::Info, &format!("Using Godot: {}", godot_bin));
    print_message(
        MessageType::Info,
        "Running script validation on all .gd files...",
    );

    let (log, status) = run_headless(
        godot_bin,
        vec![
            "--headless".to_owned(),
            "--path".to_owned(),
            GODOT_PROJECT_FOLDER.to_owned(),
            "res://src/test/validate_all_scripts.tscn".to_owned(),
            "--quit".to_owned(),
        ],
    )?;

    match headless_result(
        "GDScript validation",
        &log,
        status,
        "All scripts validated successfully!",
    ) {
        Ok(()) => {
            print_message(
                MessageType::Success,
                "All GDScript files validated successfully!",
            );
            Ok(())
        }
        Err(err) => {
            print_message(
                MessageType::Error,
                "GDScript validation failed. See errors above.",
            );
            Err(err)
        }
    }
}

/// Floor for [`discover_tests`]: the suite count that must still be found.
const MIN_TESTS: usize = 16;

/// Every headless GDScript test in the project, discovered rather than listed.
///
/// A test is a `.gd` under `godot/src/test/` that announces itself as `[<stem>] PASS`;
/// that marker is already the contract [`headless_result`] checks, so declaring the
/// suite anywhere else only creates a second place to forget. A sibling `.tscn` wins:
/// `--script` compiles before autoloads exist, so a test reaching one must run as a
/// scene or every call against it silently becomes a no-op.
fn discover_tests() -> Result<Vec<(String, PathBuf)>> {
    let root = Path::new(GODOT_PROJECT_FOLDER).join("src/test");
    let mut found = Vec::new();

    for entry in WalkDir::new(&root).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("gd") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        if !fs::read_to_string(path)?.contains(&format!("[{stem}] PASS")) {
            continue;
        }

        let scene = path.with_extension("tscn");
        found.push((
            stem.to_owned(),
            if scene.exists() {
                scene
            } else {
                path.to_owned()
            },
        ));
    }

    found.sort();
    // Discovery is by marker, so renaming a test without updating its own print drops it
    // from CI silently. The floor turns that into a red build; raise it when a suite lands.
    if found.len() < MIN_TESTS {
        bail!(
            "discovered only {} GDScript tests, expected at least {MIN_TESTS} — \
             did a test lose its `[<stem>] PASS` marker?",
            found.len()
        );
    }
    Ok(found)
}

/// Runs them all, reporting every failure rather than stopping at the first: one red
/// run should say everything that is broken.
pub fn test_gdscript() -> Result<()> {
    print_section("GDScript Tests");

    let tests = discover_tests()?;
    if tests.is_empty() {
        bail!("no GDScript tests found under {GODOT_PROJECT_FOLDER}/src/test");
    }

    let godot_bin = get_godot_path();
    let mut failed = Vec::new();

    for (stem, path) in &tests {
        let res = path.strip_prefix(GODOT_PROJECT_FOLDER).unwrap_or(path);
        let res = format!("res://{}", res.to_string_lossy());
        print_message(MessageType::Info, &format!("Running {stem}..."));

        let mut args = vec![
            "--headless".to_owned(),
            "--path".to_owned(),
            GODOT_PROJECT_FOLDER.to_owned(),
        ];
        if res.ends_with(".tscn") {
            args.push(res);
            args.push("--quit".to_owned());
        } else {
            args.push("--script".to_owned());
            args.push(res);
        }

        let (log, status) = run_headless(godot_bin.clone(), args)?;
        if headless_result(stem, &log, status, &format!("[{stem}] PASS")).is_err() {
            print_message(MessageType::Error, &format!("{stem} FAILED"));
            failed.push(stem.clone());
        }
    }

    if !failed.is_empty() {
        bail!(
            "{} of {} GDScript tests failed: {}",
            failed.len(),
            tests.len(),
            failed.join(", ")
        );
    }
    print_message(
        MessageType::Success,
        &format!("All {} GDScript tests passed!", tests.len()),
    );
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    /// Drives the real pipeline against a stand-in for Godot, covering the
    /// decision table [`headless_result`] exists for. The interesting case is
    /// the second: a test that reported success and *then* died from a signal
    /// must stay green, while every other kind of red must stay red.
    #[test]
    fn exit_crash_after_the_marker_is_the_only_failure_forgiven() {
        // (child shell, expected to pass, what it stands for)
        let cases = [
            (r#"echo "[t] PASS""#, true, "clean pass"),
            (
                r#"echo "SCRIPT ERROR: Parse Error""#,
                false,
                "exited 0 but never announced the marker — the script did not run",
            ),
            (
                r#"echo "[t] PASS"; kill -SEGV $$"#,
                true,
                "passed, then the engine crashed tearing down autoloads",
            ),
            (
                r#"echo "[t] FAIL: 3 case(s)" >&2; exit 1"#,
                false,
                "real test failure",
            ),
            (
                // A failing test crashes on the way out just as often as a
                // passing one, so the signal alone must not excuse it.
                r#"echo "[t] FAIL: 3 case(s)" >&2; kill -SEGV $$"#,
                false,
                "real test failure that also crashed on exit",
            ),
            (r#"kill -SEGV $$"#, false, "crashed before any verdict"),
            (
                r#"echo "Godot Engine v4.6.2"; exit 2"#,
                false,
                "branch missing the subcommand / bad invocation",
            ),
        ];

        for (script, should_pass, what) in cases {
            let (log, status) = run_headless(
                "/bin/sh".to_owned(),
                vec!["-c".to_owned(), script.to_owned()],
            )
            .expect("running the stand-in should not fail");
            let verdict = headless_result("t", &log, status, "[t] PASS");
            assert_eq!(
                verdict.is_ok(),
                should_pass,
                "{what}: expected pass={should_pass}, got {verdict:?} (status {status}, log {log:?})"
            );
        }
    }
}
