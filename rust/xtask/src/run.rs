use std::io::{BufRead, BufReader};

use crate::{
    consts::{BIN_FOLDER, GODOT_PROJECT_FOLDER, RUST_LIB_PROJECT_FOLDER},
    install_dependency,
    path::adjust_canonicalization, copy_files::copy_library,
};

pub fn run(
    editor: bool,
    release_mode: bool,
    itest: bool,
    only_build: bool,
    link_libs: bool,
) -> Result<(), anyhow::Error> {
    let program = adjust_canonicalization(
        std::fs::canonicalize(format!(
            "{}godot/{}",
            BIN_FOLDER,
            install_dependency::get_godot_executable_path().unwrap()
        ))
        .expect("Did you executed `cargo run -- install`?"),
    );

    std::env::set_var("GODOT4_BIN", program.clone());

    let mut args = vec!["--path", GODOT_PROJECT_FOLDER];
    if editor {
        args.push("-e");
    }

    let build_args = if release_mode {
        vec!["build", "--release"]
    } else {
        vec!["build"]
    };

    let build_cwd =
        adjust_canonicalization(std::fs::canonicalize(RUST_LIB_PROJECT_FOLDER).unwrap());

    println!("cargo build at {build_cwd} args: {:?}", build_args);

    let build_status = std::process::Command::new("cargo")
        .current_dir(build_cwd)
        .args(build_args)
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

    if itest {
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
            if line.contains("test-exiting with code ") {
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
