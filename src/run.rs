use std::{collections::HashMap, io::BufRead, path::PathBuf};

use cargo_metadata::MetadataCommand;

use crate::{
    consts::{ANDROID_NDK_VERSION, GODOT_PROJECT_FOLDER, RUST_LIB_PROJECT_FOLDER, EXPORTS_FOLDER},
    copy_files::copy_library,
    export::get_target_os,
    path::{adjust_canonicalization, get_godot_path},
    platform::validate_platform_for_target,
    ui::{print_build_status, print_message, create_spinner, MessageType},
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

        // Check if user already specified features
        let has_features = android_build_args.iter().any(|&arg| arg == "--features");
        let has_no_default_features = android_build_args
            .iter()
            .any(|&arg| arg == "--no-default-features");

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

        // Check if user already specified features
        let has_features = ios_build_args.iter().any(|&arg| arg == "--features");
        let has_no_default_features = ios_build_args
            .iter()
            .any(|&arg| arg == "--no-default-features");

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

        let build_cwd = adjust_canonicalization(std::fs::canonicalize(RUST_LIB_PROJECT_FOLDER)?);
        run_cargo_build(&PathBuf::from(build_cwd), &build_args, &with_build_envs)?;
    } else {
        let (build_args, with_build_envs) = prepare_build_args_envs(
            release_mode,
            extra_build_args,
            with_build_envs.unwrap_or_default(),
            &target,
        )?;

        let build_cwd = adjust_canonicalization(std::fs::canonicalize(RUST_LIB_PROJECT_FOLDER)?);
        run_cargo_build(&PathBuf::from(build_cwd), &build_args, &with_build_envs)?;
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
    with_build_envs.insert(
        "RUSTY_V8_SRC_BINDING_PATH".to_string(),
        binding_file_path.to_string_lossy().to_string(),
    );

    // Ensure the target directory exists.
    if !target_dir.exists() {
        std::fs::create_dir_all(&target_dir)?;
    }

    // Download the binding file if it does not already exist.
    if !binding_file_path.exists() {
        let status = std::process::Command::new("curl")
            .args(&[
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
fn setup_ffmpeg_env(with_build_envs: &mut HashMap<String, String>, target: &str) -> anyhow::Result<()> {
    // Skip for mobile platforms
    if target == "android" || target == "ios" {
        return Ok(());
    }
    
    let local_ffmpeg_path = format!("{}ffmpeg", crate::consts::BIN_FOLDER);
    if std::path::Path::new(&local_ffmpeg_path).exists() {
        // Get absolute path for FFmpeg
        let absolute_ffmpeg_path = std::fs::canonicalize(&local_ffmpeg_path)?;
        let absolute_ffmpeg_str = absolute_ffmpeg_path.to_string_lossy();
        
        // Set PKG_CONFIG_PATH to help find our local FFmpeg
        let pkg_config_path = format!("{}/lib/pkgconfig", absolute_ffmpeg_str);
        if let Some(existing_path) = with_build_envs.get("PKG_CONFIG_PATH") {
            with_build_envs.insert("PKG_CONFIG_PATH".to_string(), format!("{}:{}", pkg_config_path, existing_path));
        } else {
            with_build_envs.insert("PKG_CONFIG_PATH".to_string(), pkg_config_path.clone());
        }
        
        // Also add lib directory to LD_LIBRARY_PATH for runtime
        let lib_path = format!("{}/lib", absolute_ffmpeg_str);
        if let Some(existing_path) = with_build_envs.get("LD_LIBRARY_PATH") {
            with_build_envs.insert("LD_LIBRARY_PATH".to_string(), format!("{}:{}", lib_path, existing_path));
        } else {
            with_build_envs.insert("LD_LIBRARY_PATH".to_string(), lib_path);
        }
        
        // Set FFMPEG_DIR for ffmpeg-sys-next
        with_build_envs.insert("FFMPEG_DIR".to_string(), absolute_ffmpeg_str.to_string());
        
        // Also set PKG_CONFIG_ALLOW_SYSTEM_LIBS and PKG_CONFIG_ALLOW_SYSTEM_CFLAGS
        with_build_envs.insert("PKG_CONFIG_ALLOW_SYSTEM_LIBS".to_string(), "1".to_string());
        with_build_envs.insert("PKG_CONFIG_ALLOW_SYSTEM_CFLAGS".to_string(), "1".to_string());
        
        print_message(
            MessageType::Info,
            &format!("Using local FFmpeg 6.1 from: {}", absolute_ffmpeg_str)
        );
        print_message(
            MessageType::Info,
            &format!("PKG_CONFIG_PATH set to: {}", pkg_config_path)
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
            format!("{}/ndk/{}", android_sdk_path, ANDROID_NDK_VERSION)
        } else {
            let home = std::env::var("HOME").expect("HOME environment not set");
            format!("{}/Android/Sdk/ndk/{}", home, ANDROID_NDK_VERSION)
        }
    });

    with_build_envs.insert("ANDROID_NDK".to_string(), android_ndk_path.clone());
    with_build_envs.insert("ANDROID_NDK_HOME".to_string(), android_ndk_path.clone());

    let target_cc = format!(
        "{}/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android35-clang",
        android_ndk_path
    );
    let target_cxx = format!(
        "{}/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android35-clang++",
        android_ndk_path
    );
    let target_ar = format!(
        "{}/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar",
        android_ndk_path
    );
    let cargo_target_linker = format!(
        "{}/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android35-clang",
        android_ndk_path
    );

    with_build_envs.insert("TARGET_CC".to_string(), target_cc);
    with_build_envs.insert("TARGET_CXX".to_string(), target_cxx);
    with_build_envs.insert("TARGET_AR".to_string(), target_ar);
    with_build_envs.insert(
        "CARGO_FFMPEG_SYS_DISABLE_SIZE_T_IS_USIZE".to_string(),
        "1".to_string(),
    );
    with_build_envs.insert(
        "CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER".to_string(),
        cargo_target_linker,
    );
    with_build_envs.insert(
        "CARGO_PROFILE_RELEASE_BUILD_OVERRIDE_DEBUG".to_string(),
        "true".to_string(),
    );

    let cxxflags = "-v --target=aarch64-linux-android";
    let rustflags = format!(
        "-L{}/toolchains/llvm/prebuilt/linux-x86_64/lib/aarch64-unknown-linux-musl",
        android_ndk_path
    );

    with_build_envs.insert("CXXFLAGS".to_string(), cxxflags.to_string());
    with_build_envs.insert("RUSTFLAGS".to_string(), rustflags);

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
    let android_deps_path = std::env::current_dir()?.join(".bin/android_deps");
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

    // Set up Android toolchain paths
    let target_cc = format!(
        "{}/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android35-clang",
        ndk_path
    );
    let target_cxx = format!(
        "{}/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android35-clang++",
        ndk_path
    );
    let target_ar = format!(
        "{}/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar",
        ndk_path
    );
    let cargo_target_linker = format!(
        "{}/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android35-clang",
        ndk_path
    );

    envs.insert("TARGET_CC".to_string(), target_cc);
    envs.insert("TARGET_CXX".to_string(), target_cxx);
    envs.insert("TARGET_AR".to_string(), target_ar);
    envs.insert(
        "CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER".to_string(),
        cargo_target_linker,
    );
    envs.insert(
        "CARGO_FFMPEG_SYS_DISABLE_SIZE_T_IS_USIZE".to_string(),
        "1".to_string(),
    );
    envs.insert(
        "CARGO_PROFILE_RELEASE_BUILD_OVERRIDE_DEBUG".to_string(),
        "true".to_string(),
    );

    let cxxflags = "-v --target=aarch64-linux-android";
    let rustflags = format!(
        "-L{}/toolchains/llvm/prebuilt/linux-x86_64/lib/aarch64-unknown-linux-musl",
        ndk_path
    );

    envs.insert("CXXFLAGS".to_string(), cxxflags.to_string());
    envs.insert("RUSTFLAGS".to_string(), rustflags);

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

    let build_status = std::process::Command::new("cargo")
        .current_dir(cwd)
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
        _ => Err(anyhow::anyhow!("Unsupported platform for device deployment: {}", platform)),
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
    let adb_check = std::process::Command::new("which")
        .arg("adb")
        .output();
        
    if adb_check.is_err() || !adb_check.unwrap().status.success() {
        return Err(anyhow::anyhow!(
            "adb not found. Please install Android SDK and ensure adb is in your PATH"
        ));
    }
    
    // Check for connected devices
    let spinner = create_spinner("Checking for connected Android devices...");
    let devices_output = std::process::Command::new("adb")
        .args(&["devices", "-l"])
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
        .args(&["install", "-r", &apk_path])
        .status()?;
    spinner.finish();
    
    if !install_status.success() {
        return Err(anyhow::anyhow!("Failed to install APK"));
    }
    
    print_message(MessageType::Success, "APK installed successfully");
    
    // Launch the app
    let spinner = create_spinner("Launching application...");
    let launch_status = std::process::Command::new("adb")
        .args(&[
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
        .args(&["logcat", "-s", "godot:V", "GodotApp:V", "dclgodot:V"])
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
        .args(&["-c", "-t", "1"])
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
        .args(&[
            "--bundle",
            &ipa_path,
            "--justlaunch",
            "--debug",
        ])
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
        .args(&["devicectl", "device", "list"])
        .output()?;
        
    let devices_str = String::from_utf8_lossy(&devices_output.stdout);
    print_message(MessageType::Info, &format!("Available devices:\n{}", devices_str));
    
    Err(anyhow::anyhow!(
        "Full iOS deployment requires ios-deploy. Please install it with: brew install ios-deploy"
    ))
}
