use std::{collections::HashMap, fs, io, path::Path, process::ExitStatus};
use zip::ZipArchive;

use crate::{
    consts::{
        BIN_FOLDER, EXPORTS_FOLDER, GODOT4_EXPORT_TEMPLATES_BASE_URL, GODOT_CURRENT_VERSION,
        GODOT_PLATFORM_FILES, GODOT_PROJECT_FOLDER,
    },
    copy_files::copy_ffmpeg_libraries,
    install_dependency::{
        download_and_extract_zip, godot_export_templates_path, set_executable_permission,
    },
    path::{adjust_canonicalization, get_godot_path},
    platform::validate_platform_for_target,
    ui::{create_spinner, print_message, print_section, MessageType},
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
            "quest" => {
                // Quest is essentially Android with specific settings
                "quest".to_string()
            }
            "linux" | "win64" | "macos" => t.to_string(),
            _ => {
                return Err(anyhow::anyhow!(
                    "Unsupported provided target: {}. Supported targets: ios, android, quest, linux, win64, macos.",
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

pub fn export(target: Option<&str>, format: &str, release: bool) -> Result<(), anyhow::Error> {
    print_section("Exporting Project");

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

    // Validate platform requirements
    validate_platform_for_target(&target)?;

    print_message(MessageType::Info, &format!("Target platform: {}", target));

    // Extract Android template if needed
    if target == "android" || target == "quest" {
        extract_android_template()?;
    }

    // Determine output file name
    let output_file_name = match target.as_str() {
        "linux" => "decentraland.godot.client.x86_64",
        "win64" => "decentraland.godot.client.exe",
        "macos" => "decentraland.godot.client.dmg",
        "ios" => "decentraland-godot-client.ipa",
        "android" => {
            if format == "aab" {
                "decentraland.godot.client.aab"
            } else {
                "decentraland.godot.client.apk"
            }
        }
        "quest" => {
            if format == "aab" {
                "meta-quest.aab"
            } else {
                "meta-quest.apk"
            }
        }
        _ => return Err(anyhow::anyhow!("Unexpected final target: {}", target)),
    };

    let output_rel_path = format!("{EXPORTS_FOLDER}{output_file_name}");
    if std::path::Path::new(output_rel_path.as_str()).exists() {
        fs::remove_file(output_rel_path.as_str())?;
    }

    // Adjust the output path parameter for Godot command line
    // This should reflect the correct relative path from the Godot project directory
    let output_path_godot_param = format!("./../exports/{output_file_name}");

    // For Android/Quest AAB format, we need to update export_presets.cfg
    let export_presets_backup = if (target == "android" || target == "quest") && format == "aab" {
        update_export_presets_for_aab()?
    } else {
        None
    };

    let export_mode = if release { "--export-release" } else { "--export-debug" };
    
    let args = vec![
        "-e",
        "--rendering-driver",
        "opengl3",
        "--headless",
        export_mode,
        target.as_str(),
        output_path_godot_param.as_str(),
    ];

    print_message(MessageType::Step, "Running Godot export...");
    let spinner = create_spinner("Exporting project...");

    let mut export_command = std::process::Command::new(program.as_str());
    export_command.args(&args);
    export_command.current_dir(adjust_canonicalization(
        std::fs::canonicalize(GODOT_PROJECT_FOLDER).unwrap(),
    ));

    // Set Android keystore environment variables if exporting for Android/Quest
    if (target == "android" || target == "quest") && release {
        let keystore_path = format!("{}release.keystore", BIN_FOLDER);
        let keystore_abs_path = std::fs::canonicalize(&keystore_path)
            .unwrap_or_else(|_| Path::new(&keystore_path).to_path_buf());
        
        if !Path::new(&keystore_path).exists() {
            print_message(MessageType::Warning, "Release keystore not found. Generate it with: cargo run -- generate-keystore --type release");
        } else {
            print_message(MessageType::Info, &format!("Using keystore: {}", keystore_abs_path.display()));
        }
        
        export_command.env("GODOT_ANDROID_KEYSTORE_RELEASE_PATH", keystore_abs_path);
        export_command.env("GODOT_ANDROID_KEYSTORE_RELEASE_USER", "androidreleasekey");
        export_command.env("GODOT_ANDROID_KEYSTORE_RELEASE_PASSWORD", "android");
    }

    let export_status = export_command
        .status()
        .expect("Failed to run Godot");

    spinner.finish();
    
    // Restore export presets if we backed them up
    if let Some(backup_content) = export_presets_backup {
        restore_export_presets(backup_content)?;
    }

    if !std::path::Path::new(output_rel_path.as_str()).exists() && target != "ios" {
        print_message(MessageType::Error, "Export failed. Common issues:");
        print_message(
            MessageType::Info,
            "- Missing export templates (run: cargo run -- install --platforms <platform>)",
        );
        print_message(
            MessageType::Info,
            "- Invalid export preset in project.godot",
        );
        print_message(
            MessageType::Info,
            "- Missing platform-specific dependencies",
        );
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

    print_message(
        MessageType::Success,
        &format!("Export completed: {}", output_rel_path),
    );

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
    let dest_path = godot_export_templates_path().expect("Failed to get template path");

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

fn update_export_presets_for_aab() -> Result<Option<String>, anyhow::Error> {
    let export_presets_path = format!("{}/export_presets.cfg", GODOT_PROJECT_FOLDER);
    
    // Read current content
    let original_content = fs::read_to_string(&export_presets_path)?;
    
    // Update for AAB format
    let updated_content = original_content
        .replace("gradle_build/export_format=0", "gradle_build/export_format=1")
        .replace("architectures/x86_64=true", "architectures/x86_64=false")
        .replace("package/signed=true", "package/signed=false");
    
    // Write updated content
    fs::write(&export_presets_path, &updated_content)?;
    
    Ok(Some(original_content))
}

fn restore_export_presets(original_content: String) -> Result<(), anyhow::Error> {
    let export_presets_path = format!("{}/export_presets.cfg", GODOT_PROJECT_FOLDER);
    fs::write(&export_presets_path, original_content)?;
    Ok(())
}

fn extract_android_template() -> Result<(), anyhow::Error> {
    let android_build_dir = format!("{}/android/", GODOT_PROJECT_FOLDER);
    let android_build_path = Path::new(&android_build_dir);
    
    // Check if already extracted
    if android_build_path.exists() {
        print_message(MessageType::Info, "Android template already extracted");
        return Ok(());
    }
    
    print_message(MessageType::Step, "Extracting Android template...");
    
    // Get the template path
    let templates_path = godot_export_templates_path()
        .ok_or_else(|| anyhow::anyhow!("Could not determine export templates path"))?;
    
    let android_source_zip = format!("{}/android_source.zip", templates_path);
    let android_source_path = Path::new(&android_source_zip);
    
    if !android_source_path.exists() {
        return Err(anyhow::anyhow!(
            "Android template not found at: {}. Run 'cargo run -- install --platforms android' first", 
            android_source_zip
        ));
    }
    
    // Create directories
    fs::create_dir_all(&android_build_dir)?;
    let build_dir = format!("{}/build", android_build_dir);
    fs::create_dir_all(&build_dir)?;
    
    // Extract the template
    let file = fs::File::open(android_source_path)?;
    let mut archive = ZipArchive::new(file)?;
    
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = Path::new(&build_dir).join(file.mangled_name());
        
        if file.is_dir() {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut outfile = fs::File::create(&outpath)?;
            io::copy(&mut file, &mut outfile)?;
        }
    }
    
    // Create version file
    let version_file = format!("{}/.build_version", android_build_dir);
    fs::write(&version_file, format!("{}.stable", GODOT_CURRENT_VERSION))?;
    
    // Create .gdignore file
    let gdignore_file = format!("{}/build/.gdignore", android_build_dir);
    fs::write(&gdignore_file, "")?;
    
    print_message(MessageType::Success, "Android template extracted successfully");
    
    Ok(())
}
