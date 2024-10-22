use std::{fs, io, path::Path, process::ExitStatus};

use crate::{
    consts::{
        BIN_FOLDER, EXPORTS_FOLDER, GODOT4_EXPORT_TEMPLATES_BASE_URL, GODOT_CURRENT_VERSION,
        GODOT_PROJECT_FOLDER,
    },
    copy_files::copy_ffmpeg_libraries,
    install_dependency::{download_and_extract_zip, set_executable_permission},
    path::{adjust_canonicalization, get_godot_path},
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

pub fn import_assets() -> ExitStatus
{
    let program = get_godot_path();

    // Do imports and one project open
    let args = vec![
        "--editor",
        "--import",
        "--headless",
        "--rendering-driver",
        "opengl3",
    ];

    println!("execute ${program} {:?}", args);
    std::process::Command::new(program.as_str())
        .args(&args)
        .current_dir(adjust_canonicalization(
            std::fs::canonicalize(GODOT_PROJECT_FOLDER).unwrap(),
        ))
        .status()
        .expect("Failed to run Godot")
}

pub fn export() -> Result<(), anyhow::Error> {
    let program = get_godot_path();

    // Make exports directory
    if std::path::Path::new(EXPORTS_FOLDER).exists() {
        fs::remove_dir_all(EXPORTS_FOLDER)?;
    }
    std::thread::sleep(std::time::Duration::from_secs(1));
    fs::create_dir(EXPORTS_FOLDER)?;
    std::thread::sleep(std::time::Duration::from_secs(1));

    // Do imports and one project open
    let import_assets_status = import_assets();

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
    let output_path_godot_param = format!("./../exports/{output_file_name}");
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

    let export_status = std::process::Command::new(program.as_str())
        .args(&args)
        .current_dir(adjust_canonicalization(
            std::fs::canonicalize(GODOT_PROJECT_FOLDER).unwrap(),
        ))
        .status()
        .expect("Failed to run Godot");

    if !std::path::Path::new(output_rel_path.as_str()).exists() {
        return Err(anyhow::anyhow!(
            "Output file was not generated. pre-import godot status: {:?}, project-export godot status: {:?}",
            import_assets_status,
            export_status
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
    download_and_extract_zip(
        GODOT4_EXPORT_TEMPLATES_BASE_URL,
        dest_path.as_str(),
        Some(format!("{GODOT_CURRENT_VERSION}.export-templates.zip")),
    )?;

    Ok(())
}
