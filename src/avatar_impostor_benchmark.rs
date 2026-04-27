use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};

use crate::consts::GODOT_PROJECT_FOLDER;
use crate::path::get_godot_path;
use crate::ui::{create_spinner, print_message, print_section, MessageType};

const IOS_BUNDLE_ID: &str = "org.decentraland.godotexplorer";
const BENCHMARK_TIMEOUT_SECS: u64 = 240;
const BENCHMARK_RESULT_END_MARKER: &str = "Delta FPS:";
const BENCHMARK_RESULT_START_MARKER: &str = "=== Avatar Impostor Benchmark ===";

pub fn run_avatar_impostor_benchmark(headless: bool, target: &str) -> anyhow::Result<()> {
    print_section("Avatar Impostor Benchmark");
    match target {
        "" | "host" | "macos" | "linux" => run_host(headless),
        "ios" => run_ios(),
        other => anyhow::bail!("Unsupported target: {}", other),
    }
}

fn output_dir_abs() -> anyhow::Result<PathBuf> {
    let dir = PathBuf::from("benchmark-results");
    fs::create_dir_all(&dir)?;
    Ok(fs::canonicalize(&dir)?)
}

fn run_host(headless: bool) -> anyhow::Result<()> {
    print_message(
        MessageType::Info,
        if headless {
            "Running headless"
        } else {
            "Running with window"
        },
    );

    let output_dir = output_dir_abs()?;
    let output_file = output_dir.join("avatar-impostor-benchmark.txt");
    if output_file.exists() {
        let _ = fs::remove_file(&output_file);
    }
    let output_str = output_file.to_str().unwrap();

    let program = get_godot_path();
    let mut args: Vec<&str> = vec![
        "--path",
        GODOT_PROJECT_FOLDER,
        "--avatar-impostor-benchmark",
        "--avatar-impostor-benchmark-output",
        output_str,
    ];
    if headless {
        args.insert(2, "--headless");
    } else {
        args.insert(2, "--rendering-driver");
        args.insert(3, "vulkan");
    }

    print_message(
        MessageType::Step,
        &format!("Launching Godot: {} {}", program, args.join(" ")),
    );

    let status = Command::new(&program).args(&args).status()?;

    if !status.success() {
        print_message(
            MessageType::Warning,
            &format!("Godot exited with status: {}", status),
        );
    }

    if !output_file.exists() {
        anyhow::bail!(
            "Benchmark did not produce results at {}",
            output_file.display()
        );
    }

    print_message(
        MessageType::Success,
        &format!("Results written to {}", output_file.display()),
    );
    println!();
    let contents = fs::read_to_string(&output_file)?;
    println!("{}", contents);

    Ok(())
}

fn run_ios() -> anyhow::Result<()> {
    if std::env::consts::OS != "macos" {
        anyhow::bail!("iOS benchmark is only supported on macOS");
    }

    if !command_exists("idevicesyslog") {
        anyhow::bail!(
            "`idevicesyslog` not found. Install with `brew install libimobiledevice`."
        );
    }

    let device_id = detect_ios_device()?;
    print_message(MessageType::Info, &format!("Using device: {}", device_id));

    let output_dir = output_dir_abs()?;
    let result_file = output_dir.join("avatar-impostor-benchmark-ios.txt");
    let raw_log_file = output_dir.join("avatar-impostor-benchmark-ios.raw.log");
    if result_file.exists() {
        let _ = fs::remove_file(&result_file);
    }
    if raw_log_file.exists() {
        let _ = fs::remove_file(&raw_log_file);
    }

    // Terminate any running instance so we capture a clean run.
    let _ = Command::new("xcrun")
        .args([
            "devicectl",
            "device",
            "process",
            "terminate",
            "--device",
            &device_id,
            "--bundle-identifier",
            IOS_BUNDLE_ID,
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    // Start idevicesyslog → raw log file.
    let log_handle = File::create(&raw_log_file)?;
    let mut syslog_proc = Command::new("idevicesyslog")
        .args(["-u", &device_id])
        .stdout(log_handle)
        .stderr(Stdio::null())
        .spawn()?;
    print_message(
        MessageType::Step,
        &format!(
            "idevicesyslog streaming to {} (pid {})",
            raw_log_file.display(),
            syslog_proc.id()
        ),
    );

    // Give syslog a moment to attach.
    sleep(Duration::from_secs(2));

    // Launch the app with the benchmark flag.
    let launch_status = Command::new("xcrun")
        .args([
            "devicectl",
            "device",
            "process",
            "launch",
            "--device",
            &device_id,
            IOS_BUNDLE_ID,
            "--avatar-impostor-benchmark",
        ])
        .status()?;
    if !launch_status.success() {
        let _ = syslog_proc.kill();
        anyhow::bail!("Failed to launch app on device (status {})", launch_status);
    }
    print_message(
        MessageType::Success,
        "App launched on iOS device with --avatar-impostor-benchmark",
    );

    // Poll the raw log for the result block.
    let spinner = create_spinner("Waiting for benchmark to finish (up to 4 min)...");
    let start = Instant::now();
    let mut found = false;
    while start.elapsed() < Duration::from_secs(BENCHMARK_TIMEOUT_SECS) {
        if let Ok(contents) = fs::read_to_string(&raw_log_file) {
            if contents.contains(BENCHMARK_RESULT_END_MARKER) {
                found = true;
                break;
            }
        }
        sleep(Duration::from_secs(2));
    }
    spinner.finish();

    // Give syslog a beat to flush, then stop it.
    sleep(Duration::from_secs(2));
    let _ = syslog_proc.kill();
    let _ = syslog_proc.wait();

    let raw_log = fs::read_to_string(&raw_log_file).unwrap_or_default();
    let extracted = extract_benchmark_block(&raw_log);

    if !found || extracted.is_none() {
        print_message(
            MessageType::Warning,
            &format!(
                "Did not find a complete benchmark block in {}.",
                raw_log_file.display()
            ),
        );
        anyhow::bail!(
            "Benchmark timed out after {}s. Raw log preserved at {}",
            BENCHMARK_TIMEOUT_SECS,
            raw_log_file.display()
        );
    }

    let block = extracted.unwrap();
    let mut f = File::create(&result_file)?;
    f.write_all(block.as_bytes())?;

    print_message(
        MessageType::Success,
        &format!("Results written to {}", result_file.display()),
    );
    println!();
    println!("{}", block);

    Ok(())
}

fn detect_ios_device() -> anyhow::Result<String> {
    let output = Command::new("xcrun")
        .args(["devicectl", "list", "devices"])
        .output()?;
    if !output.status.success() {
        anyhow::bail!("`xcrun devicectl list devices` failed");
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line.contains("connected") || line.contains("available") {
            for word in line.split_whitespace() {
                if word.len() == 36 && word.chars().filter(|c| *c == '-').count() == 4 {
                    return Ok(word.to_string());
                }
            }
        }
    }
    anyhow::bail!("No connected iOS device found")
}

fn command_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Extract the benchmark result block from the raw idevicesyslog stream.
/// idevicesyslog prefixes each line with timestamp and process info; we strip
/// that and keep only the lines belonging to the result block.
fn extract_benchmark_block(raw: &str) -> Option<String> {
    let mut lines: Vec<&str> = Vec::new();
    let mut in_block = false;

    for line in raw.lines() {
        let cleaned = strip_syslog_prefix(line);
        if cleaned.contains(BENCHMARK_RESULT_START_MARKER) {
            in_block = true;
            lines.clear();
            lines.push(BENCHMARK_RESULT_START_MARKER);
            continue;
        }
        if !in_block {
            continue;
        }
        // Push the cleaned content (already without prefix).
        lines.push(cleaned);
        if cleaned.contains(BENCHMARK_RESULT_END_MARKER) {
            return Some(lines.join("\n") + "\n");
        }
    }
    None
}

/// idevicesyslog format: `MMM dd HH:MM:SS hostname process[pid] <Level>: message`
/// We return the substring after the first `: ` (after the level marker).
fn strip_syslog_prefix(line: &str) -> &str {
    if let Some(idx) = line.find("> ") {
        // Look for a "<Level>:" then the actual content. Try the common pattern.
        let after = &line[idx + 2..];
        // Some lines have leading whitespace; trim
        return after.trim_start();
    }
    if let Some(idx) = line.find(">: ") {
        return &line[idx + 3..];
    }
    // Fallback: try to skip the timestamp+process prefix conservatively
    // by finding first "]: " (after pid).
    if let Some(idx) = line.find("]: ") {
        return &line[idx + 3..];
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_block_from_syslog() {
        let raw = "\
Apr 27 16:15:32 iPhone Decentraland[1234] <Notice>: setup\n\
Apr 27 16:15:34 iPhone Decentraland[1234] <Notice>: === Avatar Impostor Benchmark ===\n\
Apr 27 16:15:34 iPhone Decentraland[1234] <Notice>: Avatars: 100 (radius 5-50 m)\n\
Apr 27 16:15:34 iPhone Decentraland[1234] <Notice>: Impostors OFF: 22.0 fps\n\
Apr 27 16:15:34 iPhone Decentraland[1234] <Notice>: Impostors ON: 50.0 fps\n\
Apr 27 16:15:34 iPhone Decentraland[1234] <Notice>: Delta FPS: +127.3%\n\
Apr 27 16:15:35 iPhone Decentraland[1234] <Notice>: quitting\n";
        let block = extract_benchmark_block(raw).expect("block");
        assert!(block.contains("=== Avatar Impostor Benchmark ==="));
        assert!(block.contains("Delta FPS: +127.3%"));
        assert!(!block.contains("quitting"));
    }
}
