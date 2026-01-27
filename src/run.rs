use std::{collections::HashMap, io::BufRead, path::PathBuf, thread, time::Duration};

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

/// Configuration for ADB device detection retry logic
const ADB_MAX_RETRIES: u32 = 3;
const ADB_RETRY_DELAY_SECS: u64 = 2;

pub fn run(
    editor: bool,
    itest: bool,
    extras: Vec<String>,
    scene_tests: bool,
    client_tests: bool,
    use_tuned_glibc: bool,
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

    if itest || scene_tests || client_tests {
        run_tests(&program, &args, scene_tests, client_tests, use_tuned_glibc)
    } else {
        run_godot(&program, &args, use_tuned_glibc)
    }
}

pub fn build(
    release_mode: bool,
    production_mode: bool,
    extra_build_args: Vec<&str>,
    with_build_envs: Option<HashMap<String, String>>,
    target: Option<&str>,
) -> anyhow::Result<()> {
    // Validate flag combinations and determine profile
    let profile = get_build_profile(release_mode, production_mode)?;

    let target = get_target_os(target)?;

    // Validate platform requirements
    validate_platform_for_target(&target)?;

    print_build_status(&target, "starting");

    // For Android, use direct cargo build with proper environment setup
    if target == "android" {
        // For now, disable it for Android builds
        let android_build_args = extra_build_args.clone();

        build_with_cargo_ndk(&profile, android_build_args)?;
    } else if target == "ios" {
        // For now, disable it for iOS builds
        let ios_build_args = extra_build_args.clone();

        let (build_args, with_build_envs) = prepare_build_args_envs(
            &profile,
            ios_build_args,
            with_build_envs.unwrap_or_default(),
            &target,
        )?;

        let build_cwd = std::fs::canonicalize(RUST_LIB_PROJECT_FOLDER)?;
        run_cargo_build(&build_cwd, &build_args, &with_build_envs)?;
    } else {
        let (build_args, with_build_envs) = prepare_build_args_envs(
            &profile,
            extra_build_args,
            with_build_envs.unwrap_or_default(),
            &target,
        )?;

        let build_cwd = std::fs::canonicalize(RUST_LIB_PROJECT_FOLDER)?;
        run_cargo_build(&build_cwd, &build_args, &with_build_envs)?;
    }

    copy_library(&target, profile)?;

    print_build_status(&target, "success");

    Ok(())
}

/// Determines the cargo build profile based on release and production/staging flags.
///
/// Returns:
/// - "dev" when neither --release nor --prod/--staging is set
/// - "dev-release" when --release is set but --prod/--staging is not
/// - "release" when both --release and --prod/--staging are set
/// - Error when --prod/--staging is set without --release
fn get_build_profile(release_mode: bool, production_mode: bool) -> anyhow::Result<&'static str> {
    match (release_mode, production_mode) {
        (false, false) => Ok("dev"),
        (true, false) => Ok("dev-release"),
        (true, true) => Ok("release"),
        (false, true) => Err(anyhow::anyhow!(
            "--prod/--staging flag requires --release flag. Use: cargo run -- build -r --prod (or --staging)"
        )),
    }
}

/// Prepares the build arguments and environment variables based on the target and mode.
fn prepare_build_args_envs(
    profile: &str,
    extra_build_args: Vec<&str>,
    mut with_build_envs: HashMap<String, String>,
    target: &String,
) -> anyhow::Result<(Vec<String>, HashMap<String, String>)> {
    let mut build_args = vec!["build"];

    // Add profile argument based on the determined profile
    match profile {
        "dev" => {} // Default profile, no argument needed
        "release" => {
            build_args.push("--release");
        }
        "dev-release" => {
            build_args.push("--profile");
            build_args.push("dev-release");
        }
        _ => unreachable!("Invalid profile: {}", profile),
    }

    build_args.extend(extra_build_args);

    // Set PROTOC environment variable to use locally installed protoc
    let protoc_path = BinPaths::protoc_bin();
    if protoc_path.exists() {
        if let Ok(canonical_path) = std::fs::canonicalize(&protoc_path) {
            with_build_envs.insert(
                "PROTOC".to_string(),
                canonical_path.to_string_lossy().to_string(),
            );
        }
    }

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

/// Sets up environment variables needed for building on Android.
fn setup_android_env(with_build_envs: &mut HashMap<String, String>) -> anyhow::Result<()> {
    // Try to find Android NDK using the platform detection from check_android_sdk
    match crate::platform::check_android_sdk() {
        Ok(ndk_path) => {
            // Use AndroidBuildEnv struct to configure environment
            let android_env = AndroidBuildEnv::new(ndk_path.clone());
            android_env.apply_to_env(with_build_envs);

            // Also set ANDROID_NDK and ANDROID_NDK_HOME
            with_build_envs.insert("ANDROID_NDK".to_string(), ndk_path.clone());
            with_build_envs.insert("ANDROID_NDK_HOME".to_string(), ndk_path);
        }
        Err(_) => {
            // Fallback to old behavior if platform detection fails
            let android_ndk = std::env::var("ANDROID_NDK").ok();
            let android_sdk = std::env::var("ANDROID_SDK").ok();

            let android_ndk_path = android_ndk.unwrap_or_else(|| {
                if let Some(android_sdk_path) = android_sdk {
                    get_android_ndk_path(&android_sdk_path)
                        .to_string_lossy()
                        .to_string()
                } else {
                    // This will likely fail, but we'll get a better error message later
                    String::new()
                }
            });

            if !android_ndk_path.is_empty() {
                // Use AndroidBuildEnv struct to configure environment
                let android_env = AndroidBuildEnv::new(android_ndk_path.clone());
                android_env.apply_to_env(with_build_envs);

                // Also set ANDROID_NDK and ANDROID_NDK_HOME
                with_build_envs.insert("ANDROID_NDK".to_string(), android_ndk_path.clone());
                with_build_envs.insert("ANDROID_NDK_HOME".to_string(), android_ndk_path);
            }
        }
    }

    Ok(())
}

// Removed check_cargo_ndk_available as we're not using cargo-ndk anymore

/// Validates Android SDK/NDK setup and returns the NDK path
fn validate_android_ndk() -> anyhow::Result<String> {
    // Use the centralized platform detection
    match crate::platform::check_android_sdk() {
        Ok(ndk_path) => Ok(ndk_path),
        Err(_) => {
            let ndk_version = ANDROID_NDK_VERSION;

            // Provide OS-specific help
            let os_specific_help = match std::env::consts::OS {
                "windows" => {
                    "Common Android SDK locations on Windows:\n\
                    - %USERPROFILE%\\AppData\\Local\\Android\\Sdk\n\
                    - %USERPROFILE%\\Android\\Sdk\n\
                    - C:\\Android\\Sdk\n\
                    - C:\\Program Files\\Android\\android-sdk"
                }
                "macos" => {
                    "Common Android SDK locations on macOS:\n\
                    - ~/Library/Android/sdk\n\
                    - ~/Android/Sdk\n\
                    - /usr/local/share/android-sdk (Homebrew)\n\
                    - /Applications/Android Studio.app/Contents/sdk"
                }
                _ => {
                    "Common Android SDK locations on Linux:\n\
                    - ~/Android/Sdk\n\
                    - /opt/android-sdk\n\
                    - ~/.android/sdk"
                }
            };

            Err(anyhow::anyhow!(
                "Android NDK not found!\n\n\
                Please install Android NDK version {} and set one of these environment variables:\n\
                - ANDROID_NDK_HOME (preferred)\n\
                - ANDROID_NDK\n\
                - ANDROID_HOME or ANDROID_SDK (NDK will be searched in <path>/ndk/{})\n\n\
                {}\n\n\
                You can install the NDK using Android Studio SDK Manager or download it from:\n\
                https://developer.android.com/ndk/downloads",
                ndk_version,
                ndk_version,
                os_specific_help
            ))
        }
    }
}

/// Builds for Android using direct cargo build (not cargo-ndk due to libc++ linking issues)
fn build_with_cargo_ndk(profile: &str, extra_build_args: Vec<&str>) -> anyhow::Result<()> {
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
            Please run: cargo run -- install --targets android\n\n"
        ));
    }

    // Setup environment similar to android-build.sh
    let mut envs = HashMap::new();

    // Set PROTOC environment variable to use locally installed protoc
    let protoc_path = BinPaths::protoc_bin();
    if protoc_path.exists() {
        if let Ok(canonical_path) = std::fs::canonicalize(&protoc_path) {
            envs.insert(
                "PROTOC".to_string(),
                canonical_path.to_string_lossy().to_string(),
            );
        }
    }

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

    // Add profile argument based on the determined profile
    match profile {
        "dev" => {} // Default profile, no argument needed
        "release" => {
            args.push("--release");
        }
        "dev-release" => {
            args.push("--profile");
            args.push("dev-release");
        }
        _ => unreachable!("Invalid profile: {}", profile),
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
fn run_godot(program: &str, args: &[&str], use_tuned_glibc: bool) -> anyhow::Result<()> {
    print_message(MessageType::Step, "Starting Godot...");

    let mut cmd = std::process::Command::new(program);
    cmd.args(args);

    // Apply tuned glibc malloc settings on Linux for better memory release
    #[cfg(target_os = "linux")]
    if use_tuned_glibc {
        print_message(MessageType::Info, "Using tuned glibc malloc settings");
        // Use mmap for allocations >= 128KB (released immediately on free)
        cmd.env("MALLOC_MMAP_THRESHOLD_", "131072");
        // Trim heap when free memory exceeds 128KB
        cmd.env("MALLOC_TRIM_THRESHOLD_", "131072");
        // Limit arenas to reduce fragmentation (default is 8 * num_cpus)
        cmd.env("MALLOC_ARENA_MAX", "2");
    }

    #[cfg(not(target_os = "linux"))]
    if use_tuned_glibc {
        print_message(
            MessageType::Warning,
            "--use-tuned-glibc is only supported on Linux",
        );
    }

    let status = cmd.status().expect("Failed to get the status of Godot");

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
fn run_tests(
    program: &str,
    args: &[&str],
    scene_tests: bool,
    client_tests: bool,
    use_tuned_glibc: bool,
) -> anyhow::Result<()> {
    // Prepare arguments for client tests
    let mut final_args = args.to_vec();

    if client_tests {
        final_args.push("--client-test");
        final_args.push("avatar_outline"); // Run avatar outline tests by default
    }

    let mut cmd = std::process::Command::new(program);
    cmd.args(&final_args);
    cmd.stdout(std::process::Stdio::piped());

    // Apply tuned glibc malloc settings on Linux
    #[cfg(target_os = "linux")]
    if use_tuned_glibc {
        cmd.env("MALLOC_MMAP_THRESHOLD_", "131072");
        cmd.env("MALLOC_TRIM_THRESHOLD_", "131072");
        cmd.env("MALLOC_ARENA_MAX", "2");
    }

    let child = cmd.spawn().expect("Failed to run Godot");

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
        } else if client_tests {
            if line.contains("All client tests passed!") {
                test_ok = (true, true, line);
            } else if line.contains("Visual tests failed!") {
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

/// Restart ADB server to fix potential connection issues
fn restart_adb_server() -> anyhow::Result<()> {
    print_message(MessageType::Info, "Restarting ADB server...");

    // Kill the server
    let _ = std::process::Command::new("adb")
        .args(["kill-server"])
        .output();

    // Small delay to let it fully shut down
    thread::sleep(Duration::from_millis(500));

    // Start the server
    let start_result = std::process::Command::new("adb")
        .args(["start-server"])
        .output()?;

    if !start_result.status.success() {
        return Err(anyhow::anyhow!("Failed to restart ADB server"));
    }

    // Give it a moment to initialize
    thread::sleep(Duration::from_secs(1));

    print_message(MessageType::Success, "ADB server restarted");
    Ok(())
}

/// Get list of connected Android devices with retry logic
/// Returns a tuple of (device_id, all_device_lines) on success
fn get_connected_android_device() -> anyhow::Result<(String, Vec<String>)> {
    let mut last_error = None;

    for attempt in 1..=ADB_MAX_RETRIES {
        let spinner = create_spinner(&format!(
            "Checking for connected Android devices (attempt {}/{})...",
            attempt, ADB_MAX_RETRIES
        ));

        let devices_output = std::process::Command::new("adb")
            .args(["devices", "-l"])
            .output();

        spinner.finish();

        match devices_output {
            Ok(output) => {
                let devices_str = String::from_utf8_lossy(&output.stdout);
                let device_lines: Vec<String> = devices_str
                    .lines()
                    .skip(1) // Skip "List of devices attached" header
                    .filter(|line| !line.is_empty() && line.contains("device"))
                    .map(|s| s.to_string())
                    .collect();

                if !device_lines.is_empty() {
                    // Extract device ID from the first device line
                    let device_id = device_lines[0]
                        .split_whitespace()
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("Failed to parse device ID"))?
                        .to_string();

                    return Ok((device_id, device_lines));
                }

                last_error = Some(anyhow::anyhow!(
                    "No Android devices found. Please connect a device and enable USB debugging."
                ));
            }
            Err(e) => {
                last_error = Some(anyhow::anyhow!("Failed to run adb devices: {}", e));
            }
        }

        // If not the last attempt, try restarting ADB and wait before retrying
        if attempt < ADB_MAX_RETRIES {
            print_message(
                MessageType::Warning,
                &format!(
                    "No devices found, retrying in {} seconds...",
                    ADB_RETRY_DELAY_SECS
                ),
            );

            // Try restarting ADB server on second attempt
            if attempt == 2 {
                if let Err(e) = restart_adb_server() {
                    print_message(
                        MessageType::Warning,
                        &format!("Failed to restart ADB server: {}", e),
                    );
                }
            } else {
                thread::sleep(Duration::from_secs(ADB_RETRY_DELAY_SECS));
            }
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Failed to detect Android devices")))
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

    // Check for connected devices with retry logic
    let (device_id, device_lines) = get_connected_android_device()?;

    if device_lines.len() > 1 {
        print_message(
            MessageType::Warning,
            &format!(
                "Multiple devices found ({}), using first device: {}",
                device_lines.len(),
                device_id
            ),
        );
    } else {
        print_message(MessageType::Info, &format!("Using device: {}", device_id));
    }

    // Install APK
    let spinner = create_spinner("Installing APK...");
    let install_status = std::process::Command::new("adb")
        .args(["-s", &device_id, "install", "-r", &apk_path])
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
            "-s",
            &device_id,
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
        .args([
            "-s",
            &device_id,
            "logcat",
            "-s",
            "godot:V",
            "GodotApp:V",
            "dclgodot:V",
        ])
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
pub fn hotreload_android(extras: Vec<String>) -> anyhow::Result<()> {
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

    // Check for connected devices with retry logic
    let (device_id, device_lines) = get_connected_android_device()?;

    print_message(
        MessageType::Info,
        &format!(
            "Found {} connected device(s), using: {}",
            device_lines.len(),
            device_id
        ),
    );

    // Get the .so file path from the canonical location (copy_library puts it here)
    let so_path = format!(
        "{}target/libdclgodot_android/libdclgodot.so",
        RUST_LIB_PROJECT_FOLDER
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
