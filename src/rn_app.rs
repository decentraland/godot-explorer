//! The React Native app in `rn/` is the shipping mobile app; the Godot export is one of its
//! inputs. These helpers turn an export into the RN app (Android APK/AAB, iOS staged project or
//! device build) and run it without the engine on simulators and emulators.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use crate::{
    consts::EXPORTS_FOLDER,
    install_dependency::godot_export_templates_path,
    ui::{print_message, MessageType},
};

pub const RN_FOLDER: &str = "./rn/";
/// Godot's own Android export, the input of `rn/scripts/sync-godot-android.sh`.
pub const GODOT_ANDROID_EXPORT_DIR: &str = "./exports/godot/";
pub const ANDROID_APK_NAME: &str = "decentraland.godot.client.apk";
pub const ANDROID_AAB_NAME: &str = "decentraland.godot.client.aab";
/// Where the iOS export stages the RN project (without node_modules/Pods) for the TestFlight build.
pub const IOS_STAGED_RN_DIR: &str = "./exports/rn/";
/// The engine built with `embedded_host=yes`, published next to `ios.zip` by the engine fork.
pub const IOS_HOST_TEMPLATE: &str = "ios_host_release.a";
const IOS_SIMULATOR_NAME: &str = "DclRn iPhone 17 Pro";

/// Fails early, before a long Godot export, when the tools the RN build needs are missing.
pub fn check_tools(target: Option<&str>) -> anyhow::Result<()> {
    let tools: &[&str] = match target {
        Some("android") => &["npm"],
        Some("ios") => &["npm", "pod"],
        _ => &[],
    };
    for tool in tools {
        if Command::new(tool).arg("--version").output().is_err() {
            return Err(anyhow::anyhow!(
                "`{tool}` not found: the React Native app (rn/) needs Node 22 (npm) and, for iOS, \
                 CocoaPods. Install them or pass --godot-only to export Godot alone."
            ));
        }
    }
    Ok(())
}

fn run_checked(command: &mut Command, what: &str) -> anyhow::Result<()> {
    print_message(MessageType::Step, what);
    let status = command.status()?;
    if !status.success() {
        return Err(anyhow::anyhow!("{what} failed with {status}"));
    }
    Ok(())
}

fn rn_path(relative: &str) -> PathBuf {
    Path::new(RN_FOLDER).join(relative)
}

/// `npm ci` when rn/node_modules is missing or older than package-lock.json.
pub fn ensure_node_modules() -> anyhow::Result<()> {
    let installed = rn_path("node_modules/.package-lock.json");
    let lock = rn_path("package-lock.json");
    let stale = match (fs::metadata(&installed), fs::metadata(&lock)) {
        (Ok(installed), Ok(lock)) => installed.modified()? < lock.modified()?,
        _ => true,
    };
    if stale {
        run_checked(
            Command::new("npm").arg("ci").current_dir(RN_FOLDER),
            "Installing React Native dependencies (npm ci)",
        )?;
    }
    Ok(())
}

/// Builds the RN Android app. With Godot, `rn/scripts/sync-godot-android.sh` first copies the
/// engine, plugins, native libs and project data out of Godot's export (`release` picks the
/// template). An `aab` is left unsigned for CI's upload-key signing; an `apk` is signed with the
/// local keystore (CI re-signs it). The result lands at the path the Godot export used to.
pub fn build_android(format: &str, release: bool, with_godot: bool) -> anyhow::Result<PathBuf> {
    if with_godot {
        let godot_apk = Path::new(GODOT_ANDROID_EXPORT_DIR).join(ANDROID_APK_NAME);
        run_checked(
            Command::new("bash")
                .arg(rn_path("scripts/sync-godot-android.sh"))
                .env("GODOT_APK", fs::canonicalize(&godot_apk)?)
                .env("GODOT_BUILD", if release { "release" } else { "debug" }),
            "Copying the Godot export into rn/android",
        )?;
    }
    ensure_node_modules()?;

    let aab = format == "aab";
    let mut gradle = Command::new("./gradlew");
    gradle
        .arg(if aab {
            ":app:bundleRelease"
        } else {
            ":app:assembleRelease"
        })
        .current_dir(rn_path("android"));
    if !with_godot {
        gradle.arg("-PdclNoGodot=true");
    }
    if aab {
        gradle.env("DCL_ANDROID_UNSIGNED", "1");
    } else {
        let keystore_type = if release { "release" } else { "debug" };
        let keystore = fs::canonicalize(crate::keystore::generate_keystore(keystore_type)?)?;
        let (alias, password) = crate::keystore::get_keystore_credentials(keystore_type);
        gradle
            .env("DCL_ANDROID_KEYSTORE", keystore)
            .env("DCL_ANDROID_KEY_ALIAS", alias)
            .env("DCL_ANDROID_KEYSTORE_PASSWORD", password);
    }
    run_checked(
        &mut gradle,
        "Building the React Native Android app (Gradle)",
    )?;

    let (built, name) = if aab {
        (
            "android/app/build/outputs/bundle/release/app-release.aab",
            ANDROID_AAB_NAME,
        )
    } else {
        (
            "android/app/build/outputs/apk/release/app-release.apk",
            ANDROID_APK_NAME,
        )
    };
    fs::create_dir_all(EXPORTS_FOLDER)?;
    let output = Path::new(EXPORTS_FOLDER).join(name);
    fs::copy(rn_path(built), &output)?;
    print_message(
        MessageType::Success,
        &format!("React Native app: {}", output.display()),
    );
    Ok(output)
}

/// The engine built with `embedded_host=yes`: `GODOT_HOST_LIB`, else the `ios_host_release.a`
/// template installed next to the other export templates.
pub fn resolve_godot_host_lib() -> anyhow::Result<PathBuf> {
    if let Ok(path) = std::env::var("GODOT_HOST_LIB") {
        let path = PathBuf::from(path);
        if path.exists() {
            return Ok(path);
        }
        return Err(anyhow::anyhow!(
            "GODOT_HOST_LIB points at a missing file: {}",
            path.display()
        ));
    }
    if let Some(templates) = godot_export_templates_path() {
        let path = Path::new(&templates).join(IOS_HOST_TEMPLATE);
        if path.exists() {
            return Ok(path);
        }
    }
    Err(anyhow::anyhow!(
        "No embedded_host Godot engine for iOS. Run `cargo run -- install --targets ios` (it fetches \
         {IOS_HOST_TEMPLATE} when the engine release has it) or set GODOT_HOST_LIB to a \
         libgodot.ios.template_release.arm64.a built with \
         `scons platform=ios target=template_release embedded_host=yes`."
    ))
}

/// Copies the Godot export into rn/ios/Godot and stages the RN project under exports/rn, which
/// the TestFlight pipeline builds (it runs `npm ci` and `pod install` itself).
pub fn prepare_ios() -> anyhow::Result<()> {
    let host_lib = resolve_godot_host_lib()?;
    run_checked(
        Command::new("bash")
            .arg(rn_path("scripts/sync-godot-ios.sh"))
            .env("GODOT_HOST_LIB", host_lib),
        "Copying the Godot export into rn/ios",
    )?;
    fs::create_dir_all(IOS_STAGED_RN_DIR)?;
    run_checked(
        Command::new("rsync").args([
            "-a",
            "--delete",
            "--exclude=node_modules",
            "--exclude=ios/Pods",
            "--exclude=ios/build",
            "--exclude=android",
            "--exclude=.bundle",
            RN_FOLDER,
            IOS_STAGED_RN_DIR,
        ]),
        "Staging the React Native project in exports/rn",
    )
}

fn pod_install() -> anyhow::Result<()> {
    let lock = fs::read_to_string(rn_path("ios/Podfile.lock")).unwrap_or_default();
    let manifest = fs::read_to_string(rn_path("ios/Pods/Manifest.lock")).unwrap_or_default();
    if lock.is_empty() || lock != manifest {
        run_checked(
            Command::new("pod")
                .arg("install")
                .current_dir(rn_path("ios")),
            "Installing CocoaPods",
        )?;
    }
    Ok(())
}

/// Builds the RN iOS app with the bundled JS (Release configuration) for a device or the
/// simulator; returns the .app path.
pub fn build_ios(simulator: bool) -> anyhow::Result<PathBuf> {
    ensure_node_modules()?;
    pod_install()?;
    let derived_data = Path::new(EXPORTS_FOLDER).join("build-rn");
    let mut xcodebuild = Command::new("xcodebuild");
    xcodebuild.args([
        "-workspace",
        "ios/DclRn.xcworkspace",
        "-scheme",
        "DclRn",
        "-configuration",
        "Release",
        "-derivedDataPath",
    ]);
    xcodebuild.arg(fs::canonicalize(EXPORTS_FOLDER)?.join("build-rn"));
    if simulator {
        xcodebuild.args([
            "-sdk",
            "iphonesimulator",
            "-destination",
            "generic/platform=iOS Simulator",
        ]);
    } else {
        xcodebuild.args([
            "-destination",
            "generic/platform=iOS",
            "-allowProvisioningUpdates",
        ]);
    }
    xcodebuild.arg("build").current_dir(RN_FOLDER);
    run_checked(
        &mut xcodebuild,
        "Building the React Native iOS app (xcodebuild)",
    )?;

    let products = if simulator {
        "Release-iphonesimulator"
    } else {
        "Release-iphoneos"
    };
    let app = derived_data
        .join("Build/Products")
        .join(products)
        .join("DclRn.app");
    if !app.exists() {
        return Err(anyhow::anyhow!(".app not found at {}", app.display()));
    }
    Ok(app)
}

fn simulator_udid() -> anyhow::Result<String> {
    let listing = Command::new("xcrun")
        .args(["simctl", "list", "devices", "available"])
        .output()?;
    let listing = String::from_utf8_lossy(&listing.stdout);
    if let Some(line) = listing.lines().find(|l| l.contains(IOS_SIMULATOR_NAME)) {
        if let Some(udid) = line.split(['(', ')']).nth(1) {
            return Ok(udid.to_string());
        }
    }
    let created = Command::new("xcrun")
        .args([
            "simctl",
            "create",
            IOS_SIMULATOR_NAME,
            "com.apple.CoreSimulator.SimDeviceType.iPhone-17-Pro",
        ])
        .output()?;
    if !created.status.success() {
        return Err(anyhow::anyhow!(
            "simctl create failed: {}",
            String::from_utf8_lossy(&created.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&created.stdout).trim().to_string())
}

/// Builds the app without Godot (the simulator always uses the mock bridge), installs it on an
/// iOS simulator and launches it.
pub fn run_ios_simulator() -> anyhow::Result<()> {
    let app = build_ios(true)?;
    let udid = simulator_udid()?;
    let _ = Command::new("xcrun")
        .args(["simctl", "boot", &udid])
        .status();
    let _ = Command::new("open").args(["-a", "Simulator"]).status();
    run_checked(
        Command::new("xcrun").args(["simctl", "bootstatus", &udid, "-b"]),
        "Waiting for the simulator to boot",
    )?;
    run_checked(
        Command::new("xcrun")
            .args(["simctl", "install", &udid])
            .arg(&app),
        "Installing on the simulator",
    )?;
    run_checked(
        Command::new("xcrun").args([
            "simctl",
            "launch",
            "--terminate-running-process",
            &udid,
            crate::run::IOS_BUNDLE_ID,
        ]),
        "Launching on the simulator",
    )
}
