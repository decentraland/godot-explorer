use std::{collections::HashMap, fs, io, path::Path, process::ExitStatus};
use zip::ZipArchive;

use crate::{
    consts::{
        EXPORTS_FOLDER, GODOT4_EXPORT_TEMPLATES_BASE_URL, GODOT_CURRENT_VERSION,
        GODOT_PLATFORM_FILES, GODOT_PROJECT_FOLDER,
    },
    helpers::get_exe_extension,
    install_dependency::{
        download_and_extract_zip, godot_export_templates_path, set_executable_permission,
    },
    path::{adjust_canonicalization, get_godot_path},
    platform::validate_platform_for_target,
    ui::{create_spinner, print_message, print_section, MessageType},
};

use walkdir::WalkDir;

/// Strips debug symbols from iOS static libraries (.a files) inside ios.zip to reduce size.
/// This extracts the zip, strips the libraries, and re-compresses it.
/// This is only run on macOS since iOS templates are only used there.
#[cfg(target_os = "macos")]
fn strip_ios_template_symbols(templates_path: &str) -> Result<(), anyhow::Error> {
    use zip::write::FileOptions;

    let ios_zip_path = Path::new(templates_path).join("ios.zip");

    if !ios_zip_path.exists() {
        print_message(
            MessageType::Warning,
            &format!("iOS template not found at: {}", ios_zip_path.display()),
        );
        return Ok(());
    }

    let size_before = fs::metadata(&ios_zip_path).map(|m| m.len()).unwrap_or(0);
    let size_before_mb = size_before as f64 / (1024.0 * 1024.0);

    // Check if already stripped (assume >1GB means not stripped)
    const MIN_UNSTRIPPED_SIZE_MB: f64 = 1024.0; // 1 GB
    if size_before_mb < MIN_UNSTRIPPED_SIZE_MB {
        print_message(
            MessageType::Info,
            &format!(
                "iOS template size is {:.1} MB (<{:.0} MB), appears already stripped. Skipping.",
                size_before_mb, MIN_UNSTRIPPED_SIZE_MB
            ),
        );
        return Ok(());
    }

    print_message(
        MessageType::Info,
        &format!("iOS template size before: {:.1} MB", size_before_mb),
    );

    // Create a temporary directory for extraction
    let temp_dir = Path::new(templates_path).join("ios_temp");
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir)?;
    }
    fs::create_dir_all(&temp_dir)?;

    // Extract the zip
    let spinner = create_spinner("Extracting iOS template...");
    let file = fs::File::open(&ios_zip_path)?;
    let mut archive = ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = temp_dir.join(file.mangled_name());

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
    spinner.finish_and_clear();

    // Strip the .a files
    let spinner = create_spinner("Stripping debug symbols...");
    let mut stripped_count = 0;

    for entry in WalkDir::new(&temp_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "a") && e.file_type().is_file())
    {
        let path = entry.path();

        // Run strip -S to remove debug symbols (keeps symbol table for linking)
        let status = std::process::Command::new("strip")
            .args(["-S", path.to_str().unwrap()])
            .status();

        match status {
            Ok(s) if s.success() => {
                stripped_count += 1;
            }
            Ok(s) => {
                print_message(
                    MessageType::Warning,
                    &format!(
                        "strip command failed for {}: exit code {:?}",
                        path.display(),
                        s.code()
                    ),
                );
            }
            Err(e) => {
                print_message(
                    MessageType::Warning,
                    &format!("Failed to run strip on {}: {}", path.display(), e),
                );
            }
        }
    }
    spinner.finish_and_clear();

    print_message(
        MessageType::Info,
        &format!("Stripped {} static libraries", stripped_count),
    );

    // Re-compress the zip
    let spinner = create_spinner("Re-compressing iOS template...");
    let new_zip_path = Path::new(templates_path).join("ios_new.zip");
    let new_zip_file = fs::File::create(&new_zip_path)?;
    let mut zip_writer = zip::ZipWriter::new(new_zip_file);

    let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    for entry in WalkDir::new(&temp_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        let relative_path = path.strip_prefix(&temp_dir)?;

        if relative_path.as_os_str().is_empty() {
            continue;
        }

        if path.is_dir() {
            let dir_name = format!("{}/", relative_path.display());
            zip_writer.add_directory(&dir_name, options)?;
        } else {
            let file_name = relative_path.to_string_lossy().to_string();
            zip_writer.start_file(&file_name, options)?;
            let mut file = fs::File::open(path)?;
            io::copy(&mut file, &mut zip_writer)?;
        }
    }

    zip_writer.finish()?;
    spinner.finish_and_clear();

    // Replace the original zip with the new one
    fs::remove_file(&ios_zip_path)?;
    fs::rename(&new_zip_path, &ios_zip_path)?;

    // Clean up temp directory
    fs::remove_dir_all(&temp_dir)?;

    let size_after = fs::metadata(&ios_zip_path).map(|m| m.len()).unwrap_or(0);
    let size_after_mb = size_after as f64 / (1024.0 * 1024.0);
    let saved_mb = (size_before as f64 - size_after as f64) / (1024.0 * 1024.0);

    print_message(
        MessageType::Success,
        &format!(
            "iOS template size after: {:.1} MB (saved {:.1} MB)",
            size_after_mb, saved_mb
        ),
    );

    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn strip_ios_template_symbols(_templates_path: &str) -> Result<(), anyhow::Error> {
    // iOS templates are only used on macOS, no-op on other platforms
    Ok(())
}

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

    // Make exports directory if it doesn't exist
    if !std::path::Path::new(EXPORTS_FOLDER).exists() {
        fs::create_dir(EXPORTS_FOLDER)?;
    }

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
    let exe_ext = get_exe_extension(&target);
    let output_file_name = match target.as_str() {
        "linux" => "decentraland.godot.client.x86_64".to_string(),
        "win64" => format!("decentraland.godot.client{}", exe_ext),
        "macos" => "decentraland.godot.client.dmg".to_string(),
        "ios" => "decentraland-godot-client.ipa".to_string(),
        "android" => {
            if format == "aab" {
                "decentraland.godot.client.aab".to_string()
            } else {
                "decentraland.godot.client.apk".to_string()
            }
        }
        "quest" => {
            if format == "aab" {
                "meta-quest.aab".to_string()
            } else {
                "meta-quest.apk".to_string()
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

    let export_mode = if release {
        "--export-release"
    } else {
        "--export-debug"
    };

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
    if target == "android" || target == "quest" {
        let keystore_type = if release { "release" } else { "debug" };

        // Generate keystore if it doesn't exist
        let keystore_path = crate::keystore::generate_keystore(keystore_type)?;
        let keystore_abs_path = std::fs::canonicalize(keystore_path)?;

        print_message(
            MessageType::Info,
            &format!("Using keystore: {}", keystore_abs_path.display()),
        );

        let (keystore_user, keystore_password) =
            crate::keystore::get_keystore_credentials(keystore_type);
        let env_prefix = format!("GODOT_ANDROID_KEYSTORE_{}", keystore_type.to_uppercase());

        export_command.env(format!("{}_PATH", env_prefix), keystore_abs_path);
        export_command.env(format!("{}_USER", env_prefix), keystore_user);
        export_command.env(format!("{}_PASSWORD", env_prefix), keystore_password);
    }

    let export_status = export_command.status().expect("Failed to run Godot");

    spinner.finish();

    // Restore export presets if we backed them up
    if let Some(backup_content) = export_presets_backup {
        restore_export_presets(backup_content)?;
    }

    if !std::path::Path::new(output_rel_path.as_str()).exists() && target != "ios" {
        print_message(MessageType::Error, "Export failed. Common issues:");
        print_message(
            MessageType::Info,
            "- Missing export templates (run: cargo run -- install --targets <platform>)",
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

    print_message(
        MessageType::Success,
        &format!("Export completed: {}", output_rel_path),
    );

    Ok(())
}

pub fn prepare_templates(platforms: &[String], no_strip: bool) -> Result<(), anyhow::Error> {
    // Convert GODOT_PLATFORM_FILES into a HashMap
    let file_map: HashMap<&str, Vec<&str>> = GODOT_PLATFORM_FILES
        .iter()
        .map(|(platform, files)| (*platform, files.to_vec()))
        .collect();

    // If no specific templates are provided, default to all templates
    let templates = if platforms.is_empty() {
        println!("No specific templates provided, downloading all templates.");
        println!(
            "For downloading for a specific platform use: `cargo run -- install --targets linux`"
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

    for template in &templates {
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

    // Strip iOS templates if downloaded (unless --no-strip is specified)
    if templates.iter().any(|t| t == "ios") && !no_strip {
        strip_ios_template_symbols(&dest_path)?;
    } else if templates.iter().any(|t| t == "ios") && no_strip {
        print_message(
            MessageType::Info,
            "Skipping iOS template stripping (--no-strip specified, debug symbols preserved for Sentry)",
        );
    }

    Ok(())
}

/// Strips debug symbols from already-installed iOS templates.
/// This can be run standalone via `cargo run -- strip-ios-templates`
pub fn strip_ios_templates() -> Result<(), anyhow::Error> {
    print_section("Stripping iOS Templates");

    let dest_path = godot_export_templates_path().expect("Failed to get template path");
    strip_ios_template_symbols(&dest_path)?;

    Ok(())
}

fn update_export_presets_for_aab() -> Result<Option<String>, anyhow::Error> {
    let export_presets_path = format!("{}/export_presets.cfg", GODOT_PROJECT_FOLDER);

    // Read current content
    let original_content = fs::read_to_string(&export_presets_path)?;

    // Update for AAB format
    let updated_content = original_content
        .replace(
            "gradle_build/export_format=0",
            "gradle_build/export_format=1",
        )
        .replace("architectures/x86_64=true", "architectures/x86_64=false")
        .replace("package/signed=true", "package/signed=false");

    // Write updated content
    fs::write(&export_presets_path, updated_content)?;

    Ok(Some(original_content))
}

fn restore_export_presets(original_content: String) -> Result<(), anyhow::Error> {
    let export_presets_path = format!("{}/export_presets.cfg", GODOT_PROJECT_FOLDER);
    fs::write(export_presets_path, original_content)?;
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
            "Android template not found at: {}. Run 'cargo run -- install --targets android' first",
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
    fs::write(version_file, format!("{}.stable", GODOT_CURRENT_VERSION))?;

    // Create .gdignore file
    let gdignore_file = format!("{}/build/.gdignore", android_build_dir);
    fs::write(gdignore_file, "")?;

    // Set executable permission on gradlew
    let gradlew_path = format!("{}/build/gradlew", android_build_dir);
    if Path::new(&gradlew_path).exists() {
        set_executable_permission(Path::new(&gradlew_path))?;
        print_message(MessageType::Info, "Set executable permission on gradlew");
    }

    print_message(
        MessageType::Success,
        "Android template extracted successfully",
    );

    Ok(())
}
