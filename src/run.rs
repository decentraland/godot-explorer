use std::{collections::HashMap, io::BufRead, path::PathBuf};

use crate::{
    consts::{GODOT_PROJECT_FOLDER, RUST_LIB_PROJECT_FOLDER}, copy_files::copy_library, export::get_target_os, path::{adjust_canonicalization, get_godot_path}
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

    let (build_args, with_build_envs) = prepare_build_args_envs(
        release_mode,
        extra_build_args,
        with_build_envs.unwrap_or_default(),
        &target,
    )?;

    let build_cwd = adjust_canonicalization(std::fs::canonicalize(RUST_LIB_PROJECT_FOLDER)?);
    run_cargo_build(&PathBuf::from(build_cwd), &build_args, &with_build_envs)?;

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
        // Set RUSTY_V8_MIRROR for iOS and Android
        with_build_envs.insert(
            "RUSTY_V8_MIRROR".to_string(),
            "https://github.com/leanmendoza/rusty_v8/releases/download".to_string(),
        );

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
