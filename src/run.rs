use std::{collections::HashMap, io::BufRead, path::PathBuf};

use cargo_metadata::MetadataCommand;

use crate::{
    consts::{GODOT_PROJECT_FOLDER, RUST_LIB_PROJECT_FOLDER},
    copy_files::copy_library,
    export::get_target_os,
    path::{adjust_canonicalization, get_godot_path},
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

    // For Android, use direct cargo build with proper environment setup
    if target == "android" {
        build_with_cargo_ndk(release_mode, extra_build_args)?;
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

    if target == "macos" {
        build_args.extend(&[
            "--no-default-features",
            "-F",
            "use_deno",
            "-F",
            "enable_inspector",
        ]);
    }

    build_args.extend(extra_build_args);

    if target == "ios" || target == "android" {
        setup_v8_bindings(&mut with_build_envs, target)?;

        match target.as_str() {
            "ios" => {
                build_args.extend(&[
                    "--no-default-features",
                    "-F",
                    "use_deno",
                    "-F",
                    "use_livekit",
                ]);
                build_args.push("--target");
                build_args.push("aarch64-apple-ios");
            }
            "android" => {
                build_args.extend(&[
                    "--no-default-features",
                    "-F",
                    "use_deno",
                    "-F",
                    "use_livekit",
                    "-F",
                    "use_ffmpeg",
                ]);
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

/// Sets up environment variables needed for building on Android.
fn setup_android_env(with_build_envs: &mut HashMap<String, String>) -> anyhow::Result<()> {
    let android_ndk = std::env::var("ANDROID_NDK").ok();
    let android_sdk = std::env::var("ANDROID_SDK").ok();

    let android_ndk_path = android_ndk.unwrap_or_else(|| {
        if let Some(android_sdk_path) = android_sdk {
            format!("{}/ndk/27.1.12297006", android_sdk_path)
        } else {
            let home = std::env::var("HOME").expect("HOME environment not set");
            format!("{}/Android/Sdk/ndk/27.1.12297006", home)
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
            println!("✓ Using Android NDK from ANDROID_NDK_HOME: {}", ndk_home);
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
            println!("✓ Using Android NDK from ANDROID_NDK: {}", ndk);
            return Ok(ndk);
        } else {
            return Err(anyhow::anyhow!(
                "ANDROID_NDK is set to '{}' but the directory doesn't exist",
                ndk
            ));
        }
    }

    // Check standard paths
    let ndk_version = "27.1.12297006";
    let possible_paths = vec![
        (std::env::var("ANDROID_SDK").ok(), "ndk/{}"),
        (std::env::var("ANDROID_HOME").ok(), "ndk/{}"),
        (std::env::var("HOME").ok(), "Android/Sdk/ndk/{}"),
    ];

    for (base_path, ndk_subpath) in possible_paths {
        if let Some(base) = base_path {
            let ndk_path = format!("{}/{}", base, ndk_subpath.replace("{}", ndk_version));
            if std::path::Path::new(&ndk_path).exists() {
                println!("✓ Found Android NDK at: {}", ndk_path);
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
        ndk_version, ndk_version
    ))
}

/// Builds for Android using direct cargo build (not cargo-ndk due to libc++ linking issues)
fn build_with_cargo_ndk(
    release_mode: bool,
    extra_build_args: Vec<&str>,
) -> anyhow::Result<()> {
    println!("Building Android target...");

    // Validate Android NDK is properly installed
    let ndk_path = validate_android_ndk()?;

    // Check if Android dependencies are installed
    let android_deps_path = std::env::current_dir()?
        .join(".bin/android_deps");
    if !android_deps_path.exists() {
        return Err(anyhow::anyhow!(
            "Android dependencies not found!\n\n\
            Please run: cargo run -- install --platforms android\n\n\
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

    args.extend(&[
        "--target",
        "aarch64-linux-android",
        "--no-default-features",
        "-F",
        "use_deno",
        "-F",
        "use_livekit",
        // Note: FFmpeg is intentionally disabled for now as in android-build.sh
    ]);

    args.extend(extra_build_args);

    println!("cargo build at {} args: {:?}", build_cwd, args);
    println!("Environment: GN_ARGS={}", envs.get("GN_ARGS").unwrap());

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
    println!("cargo build at {} args: {:?}", cwd.display(), build_args);

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
    println!("Running Godot with args: {:?}", args);
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
