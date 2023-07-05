use std::fs;

use crate::install_dependency::{self};

pub fn export() -> Result<(), anyhow::Error> {
    let program = format!(
        "./../.bin/godot/{}",
        install_dependency::get_godot_executable_path().unwrap()
    );

    // Make exports directory
    let export_dir = "./../exports";
    if !std::path::Path::new(export_dir).exists() {
        fs::create_dir(export_dir)?;
    }
    let lib_dir = "./../exports/lib";
    if !std::path::Path::new(lib_dir).exists() {
        fs::create_dir(lib_dir)?;
    }

    // Do imports and one project open
    let args = vec!["-e", "--path", "./../godot", "--headless", "--quit"];

    let status = std::process::Command::new(program.as_str())
        .args(&args)
        .status()
        .expect("Failed to run Godot");

    if !status.success() {
        return Err(anyhow::anyhow!(
            "(pre-import) Godot exited with non-zero status: {}",
            status
        ));
    }

    // Export .pck
    let args = vec![
        "-e",
        "--path",
        "./../godot",
        "--headless",
        "--export-pack",
        "linux",
        "./../exports/decentraland.godot.client.pck",
        "--quit",
    ];
    let status = std::process::Command::new(program.as_str())
        .args(&args)
        .status()
        .expect("Failed to run Godot");

    if !status.success() {
        return Err(anyhow::anyhow!(
            "(export-pack) Godot exited with non-zero status: {}",
            status
        ));
    }

    // check platform
    match std::env::consts::OS {
        "linux" => {
            std::fs::copy(
                "./../godot/lib/libdecentraland_godot_lib.so",
                "./../exports/libdecentraland_godot_lib.so",
            )?;
            std::fs::copy(program, "./../exports/decentraland.godot.client")?;
        }
        "windows" => {
            std::fs::copy(program, "./../exports/decentraland.godot.client.exe")?;
            std::fs::copy(
                "./../godot/lib/decentraland_godot_lib.dll",
                "./../exports/decentraland_godot_lib.dll",
            )?;
        }
        "macos" => {
            std::fs::copy(program, "./../exports/decentraland.godot.client")?;
            std::fs::copy(
                "./../godot/lib/libdecentraland_godot_lib.dylib",
                "./../exports/libdecentraland_godot_lib.dylib",
            )?;
        }
        _ => {}
    };

    Ok(())
}
