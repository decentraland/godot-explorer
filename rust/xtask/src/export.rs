use std::{fs, path::Path, io};

use crate::install_dependency::{self, get_godot_editor_path};

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

pub fn export() -> Result<(), anyhow::Error> {
    let program = format!(
        "./../.bin/godot/{}",
        install_dependency::get_godot_executable_path().unwrap()
    );

    // Make exports directory
    let export_dir = "./../exports";
    if std::path::Path::new(export_dir).exists() {
        fs::remove_dir_all(export_dir)?;
    }
    fs::create_dir(export_dir)?;

    // Do imports and one project open
    let args = vec!["-e", "--path", "./../godot", "--headless", "--quit"];
    std::process::Command::new(program.as_str())
        .args(&args)
        .status()
        .expect("Failed to run Godot");

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
    std::process::Command::new(program.as_str())
        .args(&args)
        .status()
        .expect("Failed to run Godot");

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
            let program = format!("./../.bin/godot/{}", get_godot_editor_path().unwrap());  
            copy_dir_all(program, "./../exports/DecentralandGodotClient.app")?;
            
            let frameworks_dir = "./../exports/DecentralandGodotClient.app/Contents/Frameworks";
            if !std::path::Path::new(frameworks_dir).exists() {
                fs::create_dir(frameworks_dir)?;
            }
            
            std::fs::copy(
                "./../godot/lib/libdecentraland_godot_lib.dylib",
                "./../exports/DecentralandGodotClient.app/Contents/Frameworks/libdecentraland_godot_lib.dylib",
            )?;
            std::fs::copy(
                "./../exports/decentraland.godot.client.pck",
                "./../exports/DecentralandGodotClient.app/Contents/Resources/Godot.pck",
            )?;
            std::fs::remove_file(
                "./../exports/decentraland.godot.client.pck"
            )?;
        }
        _ => {}
    };

    Ok(())
}
