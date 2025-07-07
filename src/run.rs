use std::{collections::HashMap, io::BufRead, path::PathBuf};

use cargo_metadata::MetadataCommand;

use crate::{
    consts::{ANDROID_NDK_VERSION, EXPORTS_FOLDER, GODOT_PROJECT_FOLDER, RUST_LIB_PROJECT_FOLDER},
    copy_files::copy_library,
    export::get_target_os,
    helpers::{get_android_ndk_path, AndroidBuildEnv, BinPaths},
    path::{adjust_canonicalization, get_godot_path},
    platform::validate_platform_for_target,
    ui::{create_spinner, print_build_status, print_message, MessageType},
};

pub fn run(
    editor: bool,
    itest: bool,
    extras: Vec<String>,
    scene_tests: bool,
) -> anyhow::Result<()> {
    let program = get_godot_path();
    println!("extras: {:?}", extras);

    std::env::set_var("GODOT4_BIN", program.clone());

    let mut args = vec!["--path", GODOT_PROJECT_FOLDER];
    if editor {
        args.push("-e");
    }

    if itest {
        args.push("--headless");
        args.push("--rendering-driver");
        args.push("vulkan");
        args.push("--verbose");
        args.push("--test-runner");
    }

    for extra in &extras {
        args.push(extra);
    }

    if itest || scene_tests {
        run_tests(&program, &args, scene_tests)
    } else {
        run_godot(&program, &args)
    }
}

/// Check if build args already have feature specifications
fn has_feature_args(build_args: &[&str]) -> (bool, bool) {
    let has_features = build_args.iter().any(|&arg| arg == "--features");
    let has_no_default_features = build_args.iter().any(|&arg| arg == "--no-default-features");
    (has_features, has_no_default_features)
}

pub fn build(
    release_mode: bool,
    extra_build_args: Vec<&str>,
    with_build_envs: Option<HashMap<String, String>>,
    target: Option<&str>,
) -> anyhow::Result<()> {
    let target = get_target_os(target)?;

    // Validate platform requirements
    validate_platform_for_target(&target)?;

    print_build_status(&target, "starting");

    // For Android, use direct cargo build with proper environment setup
    if target == "android" {
        // TODO: FFMPEG feature is going to be implemented for mobile platforms
        // For now, disable it for Android builds
        let mut android_build_args = extra_build_args.clone();

        let (has_features, has_no_default_features) = has_feature_args(&android_build_args);
        if !has_no_default_features && !has_features {
            // If user didn't specify features, disable ffmpeg by default
            android_build_args.push("--no-default-features");
            android_build_args.push("--features");
            android_build_args.push("android");
        }

        build_with_cargo_ndk(release_mode, android_build_args)?;
    } else if target == "ios" {
        // TODO: FFMPEG feature is going to be implemented for mobile platforms
        // For now, disable it for iOS builds
        let mut ios_build_args = extra_build_args.clone();

        let (has_features, has_no_default_features) = has_feature_args(&ios_build_args);
        if !has_no_default_features && !has_features {
            // If user didn't specify features, disable ffmpeg by default
            ios_build_args.push("--no-default-features");
            ios_build_args.push("--features");
            ios_build_args.push("ios");
        }

        let (build_args, with_build_envs) = prepare_build_args_envs(
            release_mode,
            ios_build_args,
            with_build_envs.unwrap_or_default(),
            &target,
        )?;

        let build_cwd = std::fs::canonicalize(RUST_LIB_PROJECT_FOLDER)?;
        run_cargo_build(&build_cwd, &build_args, &with_build_envs)?;
    } else {
        let (build_args, with_build_envs) = prepare_build_args_envs(
            release_mode,
            extra_build_args,
            with_build_envs.unwrap_or_default(),
            &target,
        )?;

        let build_cwd = std::fs::canonicalize(RUST_LIB_PROJECT_FOLDER)?;
        run_cargo_build(&build_cwd, &build_args, &with_build_envs)?;
    }

    copy_library(&target, !release_mode)?;

    print_build_status(&target, "success");

    Ok(())
}

/// Prepares the build arguments and environment variables based on the target and mode.
fn prepare_build_args_envs(
    release_mode: bool,
    extra_build_args: Vec<&str>,
    mut with_build_envs: HashMap<String, String>,
    target: &String,
) -> anyhow::Result<(Vec<String>, HashMap<String, String>)> {
    let mut build_args = vec!["build"];
    if release_mode {
        build_args.push("--release");
    }

    build_args.extend(extra_build_args);

    // On Windows, try to auto-set LIBCLANG_PATH if not already set
    #[cfg(windows)]
    {
        if std::env::var("LIBCLANG_PATH").is_err() {
            if let Some(libclang_path) = crate::platform::find_libclang_path() {
                print_message(
                    MessageType::Info,
                    &format!("Auto-detected LIBCLANG_PATH: {}", libclang_path),
                );
                with_build_envs.insert("LIBCLANG_PATH".to_string(), libclang_path);
            }
        }
    }

    if target == "ios" || target == "android" {
        setup_v8_bindings(&mut with_build_envs, target)?;

        match target.as_str() {
            "ios" => {
                // Add target, but let user control features
                build_args.push("--target");
                build_args.push("aarch64-apple-ios");
            }
            "android" => {
                // Add target, but let user control features
                build_args.push("--target");
                build_args.push("aarch64-linux-android");
                setup_android_env(&mut with_build_envs)?;
            }
            _ => {}
        }
    } else {
        // For desktop platforms, set up FFmpeg paths if local installation exists
        setup_ffmpeg_env(&mut with_build_envs, target)?;
    }

    let build_args: Vec<String> = build_args.iter().map(|&s| s.to_string()).collect();

    Ok((build_args, with_build_envs))
}

fn get_v8_version_using_metadata() -> anyhow::Result<String> {
    // Use the Cargo.toml in the "lib" folder.
    let mut cmd = MetadataCommand::new();
    cmd.manifest_path("lib/Cargo.toml");
    let metadata = cmd.exec()?;

    // Find the package corresponding to "lib/Cargo.toml"
    let lib_package = metadata
        .packages
        .iter()
        .find(|pkg| pkg.manifest_path.ends_with("lib/Cargo.toml"))
        .ok_or_else(|| anyhow::anyhow!("lib package not found in cargo metadata"))?;

    // Iterate through its dependencies to locate "v8"
    for dependency in &lib_package.dependencies {
        if dependency.name == "v8" {
            // Convert the VersionReq to a String and remove a leading '^', if present.
            let version_req = dependency.req.to_string();
            let version = version_req.strip_prefix('^').unwrap_or(&version_req);
            return Ok(version.to_string());
        }
    }
    Err(anyhow::anyhow!("v8 dependency not found in cargo metadata"))
}

/// Sets up V8 bindings by configuring environment variables and downloading the binding file if needed.
/// This function is used for both iOS and Android targets.
fn setup_v8_bindings(
    with_build_envs: &mut HashMap<String, String>,
    target: &String,
) -> anyhow::Result<()> {
    // Set the RUSTY_V8_MIRROR environment variable.
    with_build_envs.insert(
        "RUSTY_V8_MIRROR".to_string(),
        "https://github.com/dclexplorer/rusty_v8/releases/download".to_string(),
    );

    // Choose the binding file name based on the target.
    let v8_binding_file_name = if target == "android" {
        "src_binding_debug_aarch64-linux-android.rs"
    } else {
        // Adjust the file name for iOS as needed.
        "src_binding_debug_aarch64-apple-ios.rs"
    };
    let version = get_v8_version_using_metadata()?;

    let rusty_v8_mirror = with_build_envs
        .get("RUSTY_V8_MIRROR")
        .expect("RUSTY_V8_MIRROR should be set");
    let v8_binding_url = format!("{}/v{}/{}", rusty_v8_mirror, version, v8_binding_file_name);

    //println!("v8 binding url: {}", v8_binding_url);

    // Determine the absolute path for the binding file inside the target directory.
    let current_dir = std::env::current_dir()?;
    let target_dir = current_dir.join("target");
    let binding_file_path = target_dir.join(v8_binding_file_name);

    // Set the RUSTY_V8_SRC_BINDING_PATH environment variable.
    // Adjust path for Windows to remove extended path prefix
    let binding_path_str = if cfg!(windows) {
        crate::path::adjust_canonicalization(&binding_file_path)
    } else {
        binding_file_path.to_string_lossy().to_string()
    };

    with_build_envs.insert("RUSTY_V8_SRC_BINDING_PATH".to_string(), binding_path_str);

    // Ensure the target directory exists.
    if !target_dir.exists() {
        std::fs::create_dir_all(&target_dir)?;
    }

    // Download the binding file if it does not already exist.
    if !binding_file_path.exists() {
        let status = std::process::Command::new("curl")
            .args([
                "-L",
                "-o",
                binding_file_path.to_str().unwrap(),
                &v8_binding_url,
            ])
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!(
                "Failed to download V8 binding file from {}",
                v8_binding_url
            ));
        }
    }
    Ok(())
}

/// Sets up environment variables for FFmpeg if local installation exists
fn setup_ffmpeg_env(
    with_build_envs: &mut HashMap<String, String>,
    target: &str,
) -> anyhow::Result<()> {
    // Skip for mobile platforms
    if target == "android" || target == "ios" {
        return Ok(());
    }

    let local_ffmpeg_path = BinPaths::ffmpeg();
    if local_ffmpeg_path.exists() {
        // Get absolute path for FFmpeg and adjust for Windows UNC paths
        let absolute_ffmpeg_path = std::fs::canonicalize(&local_ffmpeg_path)?;
        let absolute_ffmpeg_str = crate::path::adjust_canonicalization(absolute_ffmpeg_path);

        // Set PKG_CONFIG_PATH to help find our local FFmpeg
        let pkg_config_path = format!("{}/lib/pkgconfig", absolute_ffmpeg_str);
        if let Some(existing_path) = with_build_envs.get("PKG_CONFIG_PATH") {
            with_build_envs.insert(
                "PKG_CONFIG_PATH".to_string(),
                format!("{}:{}", pkg_config_path, existing_path),
            );
        } else {
            with_build_envs.insert("PKG_CONFIG_PATH".to_string(), pkg_config_path.clone());
        }

        // Also add lib directory to LD_LIBRARY_PATH for runtime
        let lib_path = format!("{}/lib", absolute_ffmpeg_str);
        if let Some(existing_path) = with_build_envs.get("LD_LIBRARY_PATH") {
            with_build_envs.insert(
                "LD_LIBRARY_PATH".to_string(),
                format!("{}:{}", lib_path, existing_path),
            );
        } else {
            with_build_envs.insert("LD_LIBRARY_PATH".to_string(), lib_path);
        }

        // Set FFMPEG_DIR for ffmpeg-sys-next
        with_build_envs.insert("FFMPEG_DIR".to_string(), absolute_ffmpeg_str.clone());

        // Also set PKG_CONFIG_ALLOW_SYSTEM_LIBS and PKG_CONFIG_ALLOW_SYSTEM_CFLAGS
        with_build_envs.insert("PKG_CONFIG_ALLOW_SYSTEM_LIBS".to_string(), "1".to_string());
        with_build_envs.insert(
            "PKG_CONFIG_ALLOW_SYSTEM_CFLAGS".to_string(),
            "1".to_string(),
        );

        print_message(
            MessageType::Info,
            &format!("Using local FFmpeg 6.1 from: {}", absolute_ffmpeg_str),
        );
        print_message(
            MessageType::Info,
            &format!("PKG_CONFIG_PATH set to: {}", pkg_config_path),
        );
    }

    Ok(())
}

/// Sets up environment variables needed for building on Android.
fn setup_android_env(with_build_envs: &mut HashMap<String, String>) -> anyhow::Result<()> {
    let android_ndk = std::env::var("ANDROID_NDK").ok();
    let android_sdk = std::env::var("ANDROID_SDK").ok();

    let android_ndk_path = android_ndk.unwrap_or_else(|| {
        if let Some(android_sdk_path) = android_sdk {
            get_android_ndk_path(&android_sdk_path)
                .to_string_lossy()
                .to_string()
        } else {
            let home = std::env::var("HOME").expect("HOME environment not set");
            let android_sdk = format!("{}/Android/Sdk", home);
            get_android_ndk_path(&android_sdk)
                .to_string_lossy()
                .to_string()
        }
    });

    // Use AndroidBuildEnv struct to configure environment
    let android_env = AndroidBuildEnv::new(android_ndk_path.clone());
    android_env.apply_to_env(with_build_envs);

    // Also set ANDROID_NDK and ANDROID_NDK_HOME
    with_build_envs.insert("ANDROID_NDK".to_string(), android_ndk_path.clone());
    with_build_envs.insert("ANDROID_NDK_HOME".to_string(), android_ndk_path);

    Ok(())
}

// Removed check_cargo_ndk_available as we're not using cargo-ndk anymore

/// Validates Android SDK/NDK setup and returns the NDK path
fn validate_android_ndk() -> anyhow::Result<String> {
    // Check ANDROID_NDK_HOME first
    if let Ok(ndk_home) = std::env::var("ANDROID_NDK_HOME") {
        if std::path::Path::new(&ndk_home).exists() {
            return Ok(ndk_home);
        } else {
            return Err(anyhow::anyhow!(
                "ANDROID_NDK_HOME is set to '{}' but the directory doesn't exist",
                ndk_home
            ));
        }
    }

    // Check ANDROID_NDK
    if let Ok(ndk) = std::env::var("ANDROID_NDK") {
        if std::path::Path::new(&ndk).exists() {
            return Ok(ndk);
        } else {
            return Err(anyhow::anyhow!(
                "ANDROID_NDK is set to '{}' but the directory doesn't exist",
                ndk
            ));
        }
    }

    // Check standard paths
    let ndk_version = ANDROID_NDK_VERSION;
    let possible_paths = vec![
        (std::env::var("ANDROID_SDK").ok(), "ndk/{}"),
        (std::env::var("ANDROID_HOME").ok(), "ndk/{}"),
        (std::env::var("HOME").ok(), "Android/Sdk/ndk/{}"),
    ];

    for (base_path, ndk_subpath) in possible_paths {
        if let Some(base) = base_path {
            let ndk_path = format!("{}/{}", base, ndk_subpath.replace("{}", ndk_version));
            if std::path::Path::new(&ndk_path).exists() {
                return Ok(ndk_path);
            }
        }
    }

    Err(anyhow::anyhow!(
        "Android NDK not found!\n\n\
        Please install Android NDK version {} and set one of these environment variables:\n\
        - ANDROID_NDK_HOME (preferred)\n\
        - ANDROID_NDK\n\
        - ANDROID_HOME or ANDROID_SDK (NDK will be searched in <path>/ndk/{})\n\n\
        You can install the NDK using Android Studio SDK Manager or download it from:\n\
        https://developer.android.com/ndk/downloads",
        ndk_version,
        ndk_version
    ))
}

/// Builds for Android using direct cargo build (not cargo-ndk due to libc++ linking issues)
fn build_with_cargo_ndk(release_mode: bool, extra_build_args: Vec<&str>) -> anyhow::Result<()> {
    print_message(MessageType::Step, "Building Android target...");

    // Validate Android NDK is properly installed
    let ndk_path = validate_android_ndk()?;
    print_message(
        MessageType::Success,
        &format!("Using Android NDK: {}", ndk_path),
    );

    // Check if Android dependencies are installed
    let android_deps_path = BinPaths::android_deps();
    if !android_deps_path.exists() {
        return Err(anyhow::anyhow!(
            "Android dependencies not found!\n\n\
            Please run: cargo run -- install --targets android\n\n\
            This will download the required FFmpeg libraries for Android."
        ));
    }

    // Setup environment similar to android-build.sh
    let mut envs = HashMap::new();
    setup_v8_bindings(&mut envs, &"android".to_string())?;

    // Use AndroidBuildEnv struct to configure environment
    let android_env = AndroidBuildEnv::new(ndk_path.clone());
    android_env.apply_to_env(&mut envs);

    // Critical: Disable custom libcxx as per android-build.sh
    envs.insert("GN_ARGS".to_string(), "use_custom_libcxx=false".to_string());

    // Set ANDROID_NDK_HOME
    envs.insert("ANDROID_NDK_HOME".to_string(), ndk_path.clone());
    envs.insert("ANDROID_NDK".to_string(), ndk_path);

    let build_cwd = adjust_canonicalization(std::fs::canonicalize(RUST_LIB_PROJECT_FOLDER)?);

    // Use cargo build directly instead of cargo-ndk
    let mut args = vec!["build"];

    if release_mode {
        args.push("--release");
    }

    args.extend(&["--target", "aarch64-linux-android"]);

    // Let user control features via command line
    args.extend(extra_build_args);

    print_message(
        MessageType::Info,
        &format!("Running: cargo {}", args.join(" ")),
    );

    let build_status = std::process::Command::new("cargo")
        .current_dir(&build_cwd)
        .args(&args)
        .envs(&envs)
        .env("RUST_BACKTRACE", "full")
        .status()
        .expect("Failed to run cargo build");

    if !build_status.success() {
        return Err(anyhow::anyhow!(
            "cargo build exited with non-zero status: {}",
            build_status
        ));
    }

    Ok(())
}

/// Runs `cargo build` with the provided arguments and environment.
fn run_cargo_build(
    cwd: &PathBuf,
    build_args: &[String],
    envs: &HashMap<String, String>,
) -> anyhow::Result<()> {
    print_message(
        MessageType::Info,
        &format!("Running: cargo {}", build_args.join(" ")),
    );

    // On Windows, we need to use a path without the extended prefix
    // to avoid issues with build scripts that use OUT_DIR
    let working_dir = if cfg!(windows) {
        // Try to use a relative path if we're already in the correct directory
        let current_dir = std::env::current_dir()?;
        if current_dir == *cwd {
            PathBuf::from(".")
        } else {
            // Convert to string and back to remove extended prefix
            let cwd_str = crate::path::adjust_canonicalization(cwd);
            PathBuf::from(cwd_str)
        }
    } else {
        cwd.clone()
    };

    let build_status = std::process::Command::new("cargo")
        .current_dir(&working_dir)
        .args(build_args)
        .envs(envs)
        .status()
        .expect("Failed to run cargo build");

    if !build_status.success() {
        return Err(anyhow::anyhow!(
            "cargo build exited with non-zero status: {}",
            build_status
        ));
    }

    Ok(())
}

/// Runs Godot with the provided arguments and checks for successful exit.
fn run_godot(program: &str, args: &[&str]) -> anyhow::Result<()> {
    print_message(MessageType::Step, "Starting Godot...");
    let status = std::process::Command::new(program)
        .args(args)
        .status()
        .expect("Failed to get the status of Godot");

    if !status.success() {
        Err(anyhow::anyhow!(
            "Godot exited with non-zero status: {}",
            status
        ))
    } else {
        Ok(())
    }
}

/// Runs tests using Godot and checks the output to determine pass/fail.
fn run_tests(program: &str, args: &[&str], scene_tests: bool) -> anyhow::Result<()> {
    let child = std::process::Command::new(program)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to run Godot");

    let output = child.stdout.expect("Failed to get stdout of Godot");
    let reader = std::io::BufReader::new(output);

    let mut test_ok = (false, false, String::new()); // (found, ok, line)

    for line in reader.lines() {
        let line = line.expect("Failed to read line from stdout");
        println!("{}", line);

        if scene_tests {
            if line.contains("All test of all scene passed") {
                test_ok = (true, true, line);
            } else if line.contains("Some tests fail or some scenes couldn't be tested") {
                test_ok = (true, false, line);
            }
        } else if line.contains("test-exiting with code ") {
            test_ok = (true, line.contains("test-exiting with code 0"), line);
        }
    }

    if test_ok.0 {
        if test_ok.1 {
            Ok(())
        } else {
            Err(anyhow::anyhow!("test failed: {}", test_ok.2))
        }
    } else {
        Err(anyhow::anyhow!("test not run"))
    }
}

/// Deploy and run the application on a connected device
pub fn deploy_and_run_on_device(platform: &str, release: bool) -> anyhow::Result<()> {
    match platform {
        "android" => deploy_and_run_android(release),
        "ios" => deploy_and_run_ios(release),
        _ => Err(anyhow::anyhow!(
            "Unsupported platform for device deployment: {}",
            platform
        )),
    }
}

/// Deploy and run on Android device using adb
fn deploy_and_run_android(_release: bool) -> anyhow::Result<()> {
    // The APK name is always the same regardless of release/debug mode
    let apk_name = "decentraland.godot.client.apk";
    let apk_path = format!("{}/{}", EXPORTS_FOLDER, apk_name);

    // Check if APK exists
    if !std::path::Path::new(&apk_path).exists() {
        return Err(anyhow::anyhow!("APK not found at: {}", apk_path));
    }

    // Check if adb is available
    let adb_check = std::process::Command::new("which").arg("adb").output();

    if adb_check.is_err() || !adb_check.unwrap().status.success() {
        return Err(anyhow::anyhow!(
            "adb not found. Please install Android SDK and ensure adb is in your PATH"
        ));
    }

    // Check for connected devices
    let spinner = create_spinner("Checking for connected Android devices...");
    let devices_output = std::process::Command::new("adb")
        .args(["devices", "-l"])
        .output()?;
    spinner.finish();

    let devices_str = String::from_utf8_lossy(&devices_output.stdout);
    let device_lines: Vec<&str> = devices_str
        .lines()
        .skip(1) // Skip "List of devices attached" header
        .filter(|line| !line.is_empty() && line.contains("device"))
        .collect();

    if device_lines.is_empty() {
        return Err(anyhow::anyhow!(
            "No Android devices found. Please connect a device and enable USB debugging."
        ));
    }

    print_message(
        MessageType::Info,
        &format!("Found {} connected device(s)", device_lines.len()),
    );

    // Install APK
    let spinner = create_spinner("Installing APK...");
    let install_status = std::process::Command::new("adb")
        .args(["install", "-r", &apk_path])
        .status()?;
    spinner.finish();

    if !install_status.success() {
        return Err(anyhow::anyhow!("Failed to install APK"));
    }

    print_message(MessageType::Success, "APK installed successfully");

    // Launch the app
    let spinner = create_spinner("Launching application...");
    let launch_status = std::process::Command::new("adb")
        .args([
            "shell",
            "am",
            "start",
            "-n",
            "org.decentraland.godotexplorer/com.godot.game.GodotApp",
        ])
        .status()?;
    spinner.finish();

    if !launch_status.success() {
        return Err(anyhow::anyhow!("Failed to launch application"));
    }

    print_message(MessageType::Success, "Application launched on device");

    // Show logs
    print_message(MessageType::Info, "Showing device logs (Ctrl+C to stop):");
    let _log_status = std::process::Command::new("adb")
        .args(["logcat", "-s", "godot:V", "GodotApp:V", "dclgodot:V"])
        .status()?;

    Ok(())
}

/// Deploy and run on iOS device using ios-deploy or xcrun
fn deploy_and_run_ios(_release: bool) -> anyhow::Result<()> {
    // Check platform
    if std::env::consts::OS != "macos" {
        return Err(anyhow::anyhow!("iOS deployment is only supported on macOS"));
    }

    // The IPA name is always the same regardless of release/debug mode
    let ipa_name = "decentraland-godot-client.ipa";
    let ipa_path = format!("{}/{}", EXPORTS_FOLDER, ipa_name);

    // For iOS, we typically export as .xcarchive or use Xcode project
    // The actual implementation depends on how Godot exports iOS projects

    // Check if ios-deploy is available
    let ios_deploy_check = std::process::Command::new("which")
        .arg("ios-deploy")
        .output();

    if ios_deploy_check.is_err() || !ios_deploy_check.unwrap().status.success() {
        print_message(
            MessageType::Warning,
            "ios-deploy not found. Install with: brew install ios-deploy",
        );

        // Try xcrun as fallback
        return deploy_ios_with_xcrun(_release);
    }

    // Check for connected devices
    let spinner = create_spinner("Checking for connected iOS devices...");
    let devices_output = std::process::Command::new("ios-deploy")
        .args(["-c", "-t", "1"])
        .output()?;
    spinner.finish();

    let devices_str = String::from_utf8_lossy(&devices_output.stdout);
    if devices_str.contains("No devices found") {
        return Err(anyhow::anyhow!(
            "No iOS devices found. Please connect a device and trust this computer."
        ));
    }

    // Install and run
    let spinner = create_spinner("Installing and launching on iOS device...");
    let deploy_status = std::process::Command::new("ios-deploy")
        .args(["--bundle", &ipa_path, "--justlaunch", "--debug"])
        .status()?;
    spinner.finish();

    if !deploy_status.success() {
        return Err(anyhow::anyhow!("Failed to deploy to iOS device"));
    }

    print_message(MessageType::Success, "Application launched on iOS device");
    Ok(())
}

/// Fallback iOS deployment using xcrun
fn deploy_ios_with_xcrun(_release: bool) -> anyhow::Result<()> {
    // This is a simplified version - actual implementation would need
    // to handle Xcode project properly
    print_message(
        MessageType::Info,
        "Using xcrun for iOS deployment (limited functionality)",
    );

    // List devices
    let devices_output = std::process::Command::new("xcrun")
        .args(["devicectl", "device", "list"])
        .output()?;

    let devices_str = String::from_utf8_lossy(&devices_output.stdout);
    print_message(
        MessageType::Info,
        &format!("Available devices:\n{}", devices_str),
    );

    Err(anyhow::anyhow!(
        "Full iOS deployment requires ios-deploy. Please install it with: brew install ios-deploy"
    ))
}

/// Hot reload Android .so file by pushing it directly to the device
pub fn hotreload_android(release: bool, extras: Vec<String>) -> anyhow::Result<()> {
    print_message(MessageType::Step, "Hot reloading Android library...");

    // Check if adb is available
    if !std::process::Command::new("which")
        .arg("adb")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return Err(anyhow::anyhow!(
            "adb not found. Please install Android SDK and ensure adb is in your PATH"
        ));
    }

    // Check for connected devices
    let spinner = create_spinner("Checking for connected Android devices...");
    let devices_output = std::process::Command::new("adb")
        .args(["devices", "-l"])
        .output()?;
    spinner.finish();

    let devices_str = String::from_utf8_lossy(&devices_output.stdout);
    let device_lines: Vec<&str> = devices_str
        .lines()
        .skip(1) // Skip "List of devices attached" header
        .filter(|line| !line.is_empty() && line.contains("device"))
        .collect();

    if device_lines.is_empty() {
        return Err(anyhow::anyhow!(
            "No Android devices found. Please connect a device and enable USB debugging."
        ));
    }

    print_message(
        MessageType::Info,
        &format!("Found {} connected device(s)", device_lines.len()),
    );

    // Get the .so file path
    let build_mode = if release { "release" } else { "debug" };
    let so_path = format!(
        "{}target/aarch64-linux-android/{}/libdclgodot.so",
        RUST_LIB_PROJECT_FOLDER, build_mode
    );

    // Check if .so file exists
    if !std::path::Path::new(&so_path).exists() {
        return Err(anyhow::anyhow!(
            ".so file not found at: {}. Please build for Android first.",
            so_path
        ));
    }

    // Push the .so file to the device
    let package_name = "org.decentraland.godotexplorer";

    print_message(
        MessageType::Warning,
        "Note: Android push (like using --only-lib) requires an app built with android:debuggable=\"true\"",
    );

    let spinner = create_spinner("Pushing .so file to device...");

    // First, push to temp location
    let temp_path = "/data/local/tmp/libdclgodot.so";
    let push_to_temp = std::process::Command::new("adb")
        .args(["push", &so_path, temp_path])
        .status()?;

    if !push_to_temp.success() {
        spinner.finish();
        return Err(anyhow::anyhow!("Failed to push .so file to device"));
    }

    spinner.finish();

    // Try using run-as (requires debuggable app)
    print_message(
        MessageType::Step,
        "Attempting to copy library using run-as...",
    );
    let run_as_status = std::process::Command::new("adb")
        .args([
            "shell",
            &format!(
                "run-as {} sh -c 'cp {} /data/data/{}/libdclgodot.so && chmod 755 /data/data/{}/libdclgodot.so'",
                package_name, temp_path, package_name, package_name
            ),
        ])
        .output()?;

    let run_as_success = run_as_status.status.success();

    if run_as_success {
        print_message(
            MessageType::Success,
            "Library copied to app data directory!",
        );
        print_message(
            MessageType::Info,
            "Note: The app will need to be configured to load from this location",
        );
    } else {
        // Clean up temp file
        std::process::Command::new("adb")
            .args(["shell", "rm", temp_path])
            .status()
            .ok();

        return Err(anyhow::anyhow!(
            "Hotreload requires a debug build of the app (android:debuggable=\"true\").\n\n\
            For now, please use the normal deployment: cargo run -- run --target android"
        ));
    }

    // Clean up temp file
    std::process::Command::new("adb")
        .args(["shell", "rm", temp_path])
        .status()
        .ok();

    print_message(MessageType::Success, "Library pushed successfully!");

    // Restart the app to load the new library
    print_message(MessageType::Step, "Restarting application...");

    // Stop the app
    std::process::Command::new("adb")
        .args(["shell", "am", "force-stop", package_name])
        .status()?;

    // Start the app with extras
    let activity = format!("{}/com.godot.game.GodotApp", package_name);
    let mut start_args = vec![
        "shell".to_string(),
        "am".to_string(),
        "start".to_string(),
        "-n".to_string(),
        activity,
    ];

    // Add extras as intent parameters
    if !extras.is_empty() {
        // Convert Godot command line args to Android intent extras
        // For example: --skip-lobby becomes -e skip-lobby true
        for extra in &extras {
            if let Some(arg) = extra.strip_prefix("--") {
                start_args.push("-e".to_string());
                start_args.push(arg.to_string());
                start_args.push("true".to_string());
            } else if extra.starts_with('-') {
                // Single dash arguments
                if let Some(arg) = extra.strip_prefix('-') {
                    start_args.push("-e".to_string());
                    start_args.push(arg.to_string());
                    start_args.push("true".to_string());
                }
            }
        }

        print_message(
            MessageType::Info,
            &format!("Launching with extras: {:?}", extras),
        );
    }

    let start_status = std::process::Command::new("adb")
        .args(&start_args)
        .status()?;

    if !start_status.success() {
        return Err(anyhow::anyhow!("Failed to restart application"));
    }

    print_message(
        MessageType::Success,
        "Application restarted with new library!",
    );

    // Show logs
    print_message(MessageType::Info, "Showing device logs (Ctrl+C to stop):");
    std::process::Command::new("adb")
        .args(["logcat", "-s", "godot:V", "GodotApp:V", "dclgodot:V"])
        .status()?;

    Ok(())
}
