use std::{
    collections::HashMap,
    fs,
    io::{BufRead, BufReader},
};

use crate::{
    consts::{BIN_FOLDER, GODOT_PROJECT_FOLDER, RUST_LIB_PROJECT_FOLDER},
    copy_files::{copy_if_modified, copy_library},
    install_dependency,
    path::adjust_canonicalization,
};

#[allow(clippy::too_many_arguments)]
pub fn run(
    editor: bool,
    release_mode: bool,
    itest: bool,
    only_build: bool,
    link_libs: bool,
    scene_tests: bool,
    extras: Vec<String>,
    with_build_envs: Option<HashMap<String, String>>,
) -> Result<(), anyhow::Error> {
    let with_build_envs = match with_build_envs {
        Some(vars) => vars,
        None => HashMap::new(),
    };

    let program = adjust_canonicalization(
        std::fs::canonicalize(format!(
            "{}godot/{}",
            BIN_FOLDER,
            install_dependency::get_godot_executable_path().unwrap()
        ))
        .expect("Did you executed `cargo run -- install`?"),
    );

    println!("extras: {:?}", extras);

    std::env::set_var("GODOT4_BIN", program.clone());

    let mut args = vec!["--path", GODOT_PROJECT_FOLDER];
    if editor {
        args.push("-e");
    }

    #[allow(unused_mut)]
    let mut build_args = if release_mode {
        vec!["build", "--release"]
    } else {
        vec!["build"]
    };

    #[cfg(target_os = "macos")]
    {
        build_args.extend(&["--no-default-features", "-F", "use_deno"]);
    }

    let build_cwd =
        adjust_canonicalization(std::fs::canonicalize(RUST_LIB_PROJECT_FOLDER).unwrap());

    println!("cargo build at {build_cwd} args: {:?}", build_args);

    let build_status = std::process::Command::new("cargo")
        .current_dir(build_cwd)
        .args(build_args)
        .envs(with_build_envs)
        .status()
        .expect("Failed to run Godot");

    if !build_status.success() {
        return Err(anyhow::anyhow!(
            "cargo build exited with non-zero status: {}",
            build_status
        ));
    }

    match copy_library(!release_mode, link_libs) {
        Ok(_) => Ok(()),
        Err(e) => Err(anyhow::anyhow!("copy the library failed: {}", e)),
    }?;

    if only_build {
        return Ok(());
    }

    if itest {
        args.push("--headless");
        args.push("--verbose");
        args.push("--test");
    }

    if !extras.is_empty() {
        for extra in &extras {
            args.push(extra.as_str());
        }
    }

    if itest || scene_tests {
        let program = std::process::Command::new(program.as_str())
            .args(&args)
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("Failed to run Godot");

        let output = program.stdout.expect("Failed to get stdout of Godot");
        let reader = BufReader::new(output);
        let mut test_ok = (false, false, String::new()); // (found, ok)

        for line in reader.lines() {
            let line = line.expect("Failed to read line from stdout");
            println!("{}", line);

            // You can check if the line contains the desired string
            if scene_tests {
                if line.contains("All test of all scene passed") {
                    test_ok.0 = true;
                    test_ok.1 = true;
                    test_ok.2 = line;
                } else if line.contains("Some tests fail or some scenes couldn't be tested") {
                    test_ok.0 = true;
                    test_ok.1 = false;
                    test_ok.2 = line;
                }
            } else if line.contains("test-exiting with code ") {
                test_ok.0 = true;
                test_ok.1 = line.contains("test-exiting with code 0");
                test_ok.2 = line;
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
    } else {
        println!("Running Godot with args: {:?}", args);
        let status = std::process::Command::new(program.as_str())
            .args(&args)
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
}

pub fn build_web() -> Result<(), anyhow::Error> { 
    let build_cwd =
        adjust_canonicalization(std::fs::canonicalize(RUST_LIB_PROJECT_FOLDER).unwrap());
    let build_status = std::process::Command::new("cargo")
        .current_dir(build_cwd)
        .args(vec!["build", "-Zbuild-std"])
        .status()
        .expect("Failed to build lib");

    if !build_status.success() {
        return Err(anyhow::anyhow!(
            "cargo build exited with non-zero status: {}",
            build_status
        ));
    }

    let file_name = "decentraland_godot_lib.wasm";
    let source_folder: &str = "target/wasm32-unknown-emscripten/debug/";
    let source_folder = format!("{RUST_LIB_PROJECT_FOLDER}{source_folder}");
    let source_file =
        adjust_canonicalization(fs::canonicalize(source_folder)?.join(file_name.clone()));

    let lib_folder = format!("{GODOT_PROJECT_FOLDER}lib/");
    let destination_file =
        adjust_canonicalization(fs::canonicalize(lib_folder.as_str())?.join(file_name));
    copy_if_modified(source_file, destination_file, false)?;

    Ok(())
}
