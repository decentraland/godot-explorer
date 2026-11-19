//! `cargo run -- maestro`: builds the React Native app without the engine (mock bridge), installs
//! it on the connected Android device and runs the Maestro flows in `rn/.maestro/flows`.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};

use crate::{
    consts::EXPORTS_FOLDER,
    rn_app::{self, ANDROID_APK_NAME},
    ui::{print_message, MessageType},
};

const FLOWS_FOLDER: &str = "./rn/.maestro/flows/";
const ANDROID_PACKAGE: &str = "org.decentraland.godotexplorer";

/// Every flow in file-name order, or the ones whose file name starts with one of `selected`
/// (`04`, `04-discover` and `04-discover.yaml` all pick the same file).
fn resolve_flows(selected: &[String]) -> anyhow::Result<Vec<PathBuf>> {
    let mut all: Vec<PathBuf> = fs::read_dir(FLOWS_FOLDER)?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "yaml"))
        .collect();
    all.sort();
    if selected.is_empty() {
        return Ok(all);
    }
    selected
        .iter()
        .map(|name| {
            all.iter()
                .find(|path| {
                    path.file_name()
                        .is_some_and(|file| file.to_string_lossy().starts_with(name.as_str()))
                })
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("No flow in {FLOWS_FOLDER} starts with `{name}`"))
        })
        .collect()
}

fn connected_device() -> anyhow::Result<String> {
    let output = Command::new("adb").arg("devices").output().map_err(|_| {
        anyhow::anyhow!(
            "adb not found: install the Android SDK platform tools and add them to PATH"
        )
    })?;
    let listing = String::from_utf8_lossy(&output.stdout);
    let devices: Vec<&str> = listing
        .lines()
        .skip(1)
        .filter_map(|line| line.strip_suffix("\tdevice"))
        .collect();
    match devices.as_slice() {
        [device] => Ok(device.to_string()),
        [] => Err(anyhow::anyhow!(
            "No Android device or emulator connected (see `adb devices`)"
        )),
        _ => Err(anyhow::anyhow!(
            "Several devices connected ({}): pick one with --device <id>",
            devices.join(", ")
        )),
    }
}

/// The mock APK has `versionCode` 1 and the local debug key, so it cannot replace a CI build:
/// that app is removed first. The flows clear the app's data anyway.
fn install(device: &str, apk: &Path) -> anyhow::Result<()> {
    print_message(MessageType::Step, "Installing the mock build");
    let install = || {
        Command::new("adb")
            .args(["-s", device, "install", "-r"])
            .arg(apk)
            .status()
    };
    if install()?.success() {
        return Ok(());
    }
    print_message(
        MessageType::Warning,
        "The installed app is another build: removing it and installing again",
    );
    Command::new("adb")
        .args(["-s", device, "uninstall", ANDROID_PACKAGE])
        .status()?;
    if !install()?.success() {
        return Err(anyhow::anyhow!("Failed to install {}", apk.display()));
    }
    Ok(())
}

pub fn run(selected: &[String], device: Option<&str>, skip_build: bool) -> anyhow::Result<()> {
    if Command::new("maestro").arg("--version").output().is_err() {
        return Err(anyhow::anyhow!(
            "`maestro` not found: install it with `curl -fsSL https://get.maestro.mobile.dev | bash`"
        ));
    }
    let flows = resolve_flows(selected)?;
    let device = match device {
        Some(device) => device.to_string(),
        None => connected_device()?,
    };
    print_message(MessageType::Info, &format!("Using device: {device}"));

    if !skip_build {
        rn_app::build_android("apk", true, false)?;
        install(&device, &Path::new(EXPORTS_FOLDER).join(ANDROID_APK_NAME))?;
    }

    let output = Path::new(EXPORTS_FOLDER).join("maestro");
    if output.exists() {
        fs::remove_dir_all(&output)?;
    }
    fs::create_dir_all(&output)?;

    let mut results = Vec::new();
    for flow in &flows {
        let name = flow.file_stem().unwrap_or_default().to_string_lossy();
        print_message(MessageType::Step, &format!("Flow {name}"));
        let started = Instant::now();
        let passed = Command::new("maestro")
            .args(["--device", &device, "test", "--format", "junit", "--output"])
            .arg(output.join(format!("{name}.xml")))
            .arg("--test-output-dir")
            .arg(output.join(name.as_ref()))
            .arg(flow)
            .status()?
            .success();
        results.push((name, passed, started.elapsed().as_secs()));
    }

    println!();
    for (name, passed, seconds) in &results {
        let mark = if *passed { "PASS" } else { "FAIL" };
        println!("{mark}  {name}  ({}m {:02}s)", seconds / 60, seconds % 60);
    }
    let failed = results.iter().filter(|(_, passed, _)| !passed).count();
    let summary = format!(
        "{} of {} flows passed; reports, logs and screenshots in {}",
        results.len() - failed,
        results.len(),
        output.display()
    );
    if failed > 0 {
        print_message(MessageType::Error, &summary);
        return Err(anyhow::anyhow!("{failed} Maestro flow(s) failed"));
    }
    print_message(MessageType::Success, &summary);
    Ok(())
}
