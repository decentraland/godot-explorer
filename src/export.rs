use std::{collections::HashMap, fs, io, path::Path, process::ExitStatus};

use crate::{
    consts::{
        EXPORTS_FOLDER, GODOT4_EXPORT_TEMPLATES_BASE_URL, GODOT_CURRENT_VERSION,
        GODOT_PLATFORM_FILES, GODOT_PROJECT_FOLDER,
    },
    copy_files::copy_ffmpeg_libraries,
    install_dependency::{download_and_extract_zip, get_template_path, set_executable_permission},
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

pub fn get_target_os(target: Option<&str>) -> anyhow::Result<String> {
    let target = if let Some(t) = target {
        let t = t.to_lowercase();
        match t.as_str() {
            "ios" => {
                // iOS can only be compiled from macOS
                if std::env::consts::OS != "macos" {
                    return Err(anyhow::anyhow!(
                        "iOS builds are only supported on macOS hosts"
                    ));
                }
                "ios".to_string()
            }
            "android" => {
                // Android can usually be built from multiple platforms assuming you have the correct export templates
                "android".to_string()
            }
            "linux" | "win64" | "macos" => t.to_string(),
            _ => {
                return Err(anyhow::anyhow!(
                    "Unsupported provided target: {}. Supported targets: ios, android, linux, win64, macos.",
                    t
                ));
            }
        }
    } else {
        // Fallback to host OS
        match std::env::consts::OS {
            "linux" => "linux".to_string(),
            "windows" => "win64".to_string(),
            "macos" => "macos".to_string(),
            _ => {
                return Err(anyhow::anyhow!(
                    "Unsupported platform for exporting: {}",
                    std::env::consts::OS
                ));
            }
        }
    };

    Ok(target)
}

pub fn import_assets() -> ExitStatus {
    let program = get_godot_path();

    // Do imports and one project open
    let args = vec![
        "--editor",
        "--import",
        "--headless",
        "--rendering-driver",
        "opengl3",
        "--quit-after",
        "1000",
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

pub fn export(target: Option<&str>) -> Result<(), anyhow::Error> {
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

    // Determine final target if not specified
    let target = get_target_os(target)?;

    // Determine output file name
    let output_file_name = match target.as_str() {
        "linux" => "decentraland.godot.client.x86_64",
        "win64" => "decentraland.godot.client.exe",
        "macos" => "decentraland.godot.client.dmg",
        "ios" => "decentraland-godot-client.ipa",
        "android" => "decentraland.godot.client.apk",
        _ => return Err(anyhow::anyhow!("Unexpected final target: {}", target)),
    };

    let output_rel_path = format!("{EXPORTS_FOLDER}{output_file_name}");
    if std::path::Path::new(output_rel_path.as_str()).exists() {
        fs::remove_file(output_rel_path.as_str())?;
    }

    // Adjust the output path parameter for Godot command line
    // This should reflect the correct relative path from the Godot project directory
    let output_path_godot_param = format!("./../exports/{output_file_name}");

    let args = vec![
        "-e",
        "--rendering-driver",
        "opengl3",
        "--headless",
        "--export-debug",
        target.as_str(),
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

    if !std::path::Path::new(output_rel_path.as_str()).exists() && target != "ios" {
        return Err(anyhow::anyhow!(
            "Output file was not generated. pre-import godot status: {:?}, project-export godot status: {:?}",
            import_assets_status,
            export_status
        ));
    }

    // Set executable permission on Linux
    if target == "linux" {
        set_executable_permission(Path::new(output_rel_path.as_str()))?;
    }

    copy_ffmpeg_libraries(&target, EXPORTS_FOLDER.to_string(), false)?;

    Ok(())
}

pub fn prepare_templates(platforms: &[String]) -> Result<(), anyhow::Error> {
    // Convert GODOT_PLATFORM_FILES into a HashMap
    let file_map: HashMap<&str, Vec<&str>> = GODOT_PLATFORM_FILES
        .iter()
        .map(|(platform, files)| (*platform, files.to_vec()))
        .collect();

    // If no specific templates are provided, default to all templates
    let templates = if platforms.is_empty() {
        println!("No specific templates provided, downloading all templates.");
        println!(
            "For downloading for a specific platform use: `cargo run -- install --platform linux`"
        );
        file_map
            .keys()
            .map(|&k| k.to_string())
            .collect::<Vec<String>>()
    } else {
        platforms.to_vec()
    };

    // Process each template and download the associated files
    let dest_path = get_template_path().expect("Failed to get template path");

    for template in templates {
        if let Some(files) = file_map.get(template.as_str()) {
            for file in files {
                println!("Downloading file for {}: {}", template, file);

                let url = format!("{}{}.zip", GODOT4_EXPORT_TEMPLATES_BASE_URL, file);
                download_and_extract_zip(
                    url.as_str(),
                    dest_path.as_str(),
                    Some(format!(
                        "{GODOT_CURRENT_VERSION}.{file}.export-templates.zip"
                    )),
                )?;
            }
        } else {
            println!("No files mapped for template: {}", template);
        }
    }

    Ok(())
}
