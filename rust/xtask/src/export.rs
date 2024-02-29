use std::{fs, io, path::Path};

use crate::{
    consts::{BIN_FOLDER, EXPORTS_FOLDER, GODOT4_EXPORT_TEMPLATES_BASE_URL, GODOT_PROJECT_FOLDER},
    copy_files::copy_ffmpeg_libraries,
    install_dependency::{self, download_and_extract_zip, set_executable_permission},
    path::adjust_canonicalization,
};

#[allow(dead_code)]
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
    let program = adjust_canonicalization(
        std::fs::canonicalize(format!(
            "{}godot/{}",
            BIN_FOLDER,
            install_dependency::get_godot_executable_path().unwrap()
        ))
        .unwrap(),
    );

    // Make exports directory
    if std::path::Path::new(EXPORTS_FOLDER).exists() {
        fs::remove_dir_all(EXPORTS_FOLDER)?;
    }
    std::thread::sleep(std::time::Duration::from_secs(1));
    fs::create_dir(EXPORTS_FOLDER)?;
    std::thread::sleep(std::time::Duration::from_secs(1));

    // Do imports and one project open
    let args = vec![
        "-e",
        "--headless",
        "--rendering-driver",
        "opengl3",
        "--quit-after",
        "1000",
    ];
    let status1 = std::process::Command::new(program.as_str())
        .args(&args)
        .current_dir(adjust_canonicalization(
            std::fs::canonicalize(GODOT_PROJECT_FOLDER).unwrap(),
        ))
        .status()
        .expect("Failed to run Godot");

    let output_file_name = match std::env::consts::OS {
        "linux" => "decentraland.godot.client.x86_64",
        "windows" => "decentraland.godot.client.exe",
        "macos" => "decentraland.godot.client.dmg",
        _ => {
            return Err(anyhow::anyhow!(
                "Unsupported platform: {}",
                std::env::consts::OS
            ));
        }
    };

    let target = match std::env::consts::OS {
        "linux" => "linux",
        "windows" => "win64",
        "macos" => "macos",
        _ => {
            return Err(anyhow::anyhow!(
                "Unsupported platform: {}",
                std::env::consts::OS
            ));
        }
    };

    let output_rel_path = format!("{EXPORTS_FOLDER}{output_file_name}");
    if std::path::Path::new(output_rel_path.as_str()).exists() {
        fs::remove_file(output_rel_path.as_str())?;
    }

    // See this exports path differ from EXPORT_FOLDER because it's relative to godot project dir
    let output_path_godot_param = format!("../exports/{output_file_name}");
    let args = vec![
        "-e",
        "--rendering-driver",
        "opengl3",
        "--headless",
        "--export-release",
        target,
        output_path_godot_param.as_str(),
    ];

    println!("Running the export build with command: {:?}", args);

    let status2 = std::process::Command::new(program.as_str())
        .args(&args)
        .current_dir(adjust_canonicalization(
            std::fs::canonicalize(GODOT_PROJECT_FOLDER).unwrap(),
        ))
        .status()
        .expect("Failed to run Godot");

    if !std::path::Path::new(output_rel_path.as_str()).exists() {
        return Err(anyhow::anyhow!(
            "Output file was not generated. pre-import godot status: {:?}, project-export godot status: {:?}",
            status1,
            status2
        ));
    }

    if std::env::consts::OS == "linux" {
        set_executable_permission(Path::new(output_rel_path.as_str()))?;
    }

    copy_ffmpeg_libraries(EXPORTS_FOLDER.to_string(), false)?;

    Ok(())
}

pub fn prepare_templates() -> Result<(), anyhow::Error> {
    let dest_path = format!("{BIN_FOLDER}godot/templates");
    download_and_extract_zip(GODOT4_EXPORT_TEMPLATES_BASE_URL, dest_path.as_str())?;

    Ok(())
}
