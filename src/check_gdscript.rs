use std::io::{self, BufRead, BufReader, Write};
use std::process::ExitStatus;

use anyhow::{Context, Result};
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
    if status.success() {
        return Ok(());
    }

    // `code()` is `None` only when a signal killed the process, which cannot
    // happen on Windows, so this stays false there.
    if status.code().is_none() && log.contains(success_marker) {
        print_message(
            MessageType::Warning,
            &format!(
                "{label} passed but Godot crashed on the way out ({status}) — \
                 treating as success (engine teardown crash, not a test failure)"
            ),
        );
        return Ok(());
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

/// Runs a set of headless GDScript unit tests, each a `SceneTree` script that
/// exits non-zero on failure. `scripts` are paths under `godot/`, given in full so
/// tests are not confined to one directory.
///
/// Each script announces itself as `[<file stem>] PASS`; see [`headless_result`]
/// for why that marker, rather than the exit status, decides the outcome.
fn run_script_tests(section: &str, kind: &str, scripts: &[&str]) -> Result<()> {
    print_section(section);

    let godot_bin = get_godot_path();
    for script in scripts {
        let name = script.rsplit('/').next().unwrap_or(script);
        let stem = name.strip_suffix(".gd").unwrap_or(name);
        print_message(MessageType::Info, &format!("Running {name}..."));

        let (log, status) = run_headless(
            godot_bin.clone(),
            vec![
                "--headless".to_owned(),
                "--path".to_owned(),
                GODOT_PROJECT_FOLDER.to_owned(),
                "--script".to_owned(),
                format!("res://{script}"),
            ],
        )?;

        if let Err(err) = headless_result(name, &log, status, &format!("[{stem}] PASS")) {
            print_message(MessageType::Error, &format!("{name} FAILED"));
            return Err(err.context(format!("{kind} test failed: {name}")));
        }
    }

    print_message(MessageType::Success, &format!("All {kind} tests passed!"));
    Ok(())
}

pub fn test_avatar() -> Result<()> {
    run_script_tests(
        "Avatar Regression Tests",
        "avatar regression",
        &[
            "src/test/avatar/test_avatar_locomotion_grounded.gd",
            "src/test/avatar/test_avatar_state_machine_graph.gd",
            "src/test/avatar/test_avatar_autoplay_stomp.gd",
            "src/test/avatar/test_avatar_anim_throttle.gd",
        ],
    )
}

pub fn test_i18n() -> Result<()> {
    run_script_tests(
        "Localization Tests",
        "localization",
        &["src/test/i18n/test_translation_key.gd"],
    )
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
