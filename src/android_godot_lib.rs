use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;

use zip::write::FileOptions;
use zip::{ZipArchive, ZipWriter};

use crate::consts::{BIN_FOLDER, GODOT_PROJECT_FOLDER};
use crate::ui::{create_spinner, print_message, print_section, MessageType};

const CONFIG_FILE: &str = ".bin/godot_engine_config.json";

#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct GodotEngineConfig {
    pub godot_engine_path: Option<String>,
}

impl GodotEngineConfig {
    pub fn load() -> Self {
        let config_path = Path::new(CONFIG_FILE);
        if config_path.exists() {
            if let Ok(content) = fs::read_to_string(config_path) {
                if let Ok(config) = serde_json::from_str(&content) {
                    return config;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> anyhow::Result<()> {
        fs::create_dir_all(BIN_FOLDER)?;
        let content = serde_json::to_string_pretty(self)?;
        fs::write(CONFIG_FILE, content)?;
        Ok(())
    }
}

/// Expand ~ to home directory
fn expand_path(path: &str) -> String {
    if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(&path[2..]).to_string_lossy().to_string();
        }
    }
    path.to_string()
}

/// Get the Godot engine path, prompting user if not configured or invalid
fn get_godot_engine_path(validate_android: bool) -> anyhow::Result<String> {
    let mut config = GodotEngineConfig::load();

    // Check if we have a saved path and it's still valid
    if let Some(ref path) = config.godot_engine_path {
        let is_valid = if validate_android {
            // For Android, check if the android platform exists
            Path::new(&format!("{}/platform/android", path)).exists()
        } else {
            // For iOS, check if the bin folder exists
            Path::new(&format!("{}/bin", path)).exists()
        };

        if is_valid {
            return Ok(path.clone());
        } else {
            print_message(
                MessageType::Warning,
                &format!("Saved Godot engine path no longer valid: {}", path),
            );
        }
    }

    // Prompt user for path
    print_message(
        MessageType::Info,
        "Please provide the path to your Godot engine repository.",
    );
    print_message(
        MessageType::Info,
        "This should be the root of your godotengine checkout (e.g., ~/github/godotengine)",
    );

    print!("Godot engine path: ");
    std::io::stdout().flush()?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let input = input.trim();

    if input.is_empty() {
        return Err(anyhow::anyhow!("Godot engine path is required"));
    }

    let expanded_path = expand_path(input);

    // Validate path
    if !Path::new(&expanded_path).exists() {
        return Err(anyhow::anyhow!("Path does not exist: {}", expanded_path));
    }

    // Save the path
    config.godot_engine_path = Some(expanded_path.clone());
    config.save()?;

    print_message(
        MessageType::Success,
        &format!("Godot engine path saved: {}", expanded_path),
    );

    Ok(expanded_path)
}

/// Check if Android template exists
fn android_template_exists() -> bool {
    Path::new(&format!(
        "{}/android/build/libs/debug/godot-lib.template_debug.aar",
        GODOT_PROJECT_FOLDER
    ))
    .exists()
}

/// Update the Godot Android library in the AAR
pub fn update_libgodot_android(release: bool) -> anyhow::Result<()> {
    let build_type = if release { "release" } else { "debug" };

    print_section(&format!("Updating Godot Android Library ({})", build_type));

    // Get Godot engine path (required for this command)
    let godot_engine_path = get_godot_engine_path(true)?;

    // Check if Android template exists
    if !android_template_exists() {
        print_message(
            MessageType::Warning,
            "Android template not found. Run the following first:",
        );
        print_message(
            MessageType::Info,
            "  cargo run -- install --targets android",
        );
        print_message(
            MessageType::Info,
            "  cargo run -- export --target android --format apk  (to extract template)",
        );
        return Err(anyhow::anyhow!("Android template not found"));
    }

    // Determine source and target paths
    let source_so = format!(
        "{}/platform/android/java/lib/libs/{}/arm64-v8a/libgodot_android.so",
        godot_engine_path, build_type
    );
    let target_aar = format!(
        "{}/android/build/libs/{}/godot-lib.template_{}.aar",
        GODOT_PROJECT_FOLDER, build_type, build_type
    );

    // Check source file exists
    if !Path::new(&source_so).exists() {
        print_message(
            MessageType::Error,
            &format!("Source library not found: {}", source_so),
        );
        print_message(MessageType::Info, "Build it first with:");
        print_message(
            MessageType::Info,
            &format!(
                "  cd {} && scons platform=android target=template_{} arch=arm64",
                godot_engine_path, build_type
            ),
        );
        return Err(anyhow::anyhow!("Source library not found"));
    }

    // Check target AAR exists
    if !Path::new(&target_aar).exists() {
        print_message(
            MessageType::Error,
            &format!("Target AAR not found: {}", target_aar),
        );
        print_message(
            MessageType::Info,
            "Run 'cargo run -- install --targets android' to download templates.",
        );
        return Err(anyhow::anyhow!("Target AAR not found"));
    }

    // Create backup if doesn't exist
    let backup_aar = format!("{}.backup", target_aar);
    if !Path::new(&backup_aar).exists() {
        print_message(MessageType::Step, "Creating backup...");
        fs::copy(&target_aar, &backup_aar)?;
    }

    let spinner = create_spinner("Extracting AAR...");

    // Read the source .so file
    let mut source_data = Vec::new();
    File::open(&source_so)?.read_to_end(&mut source_data)?;

    // Extract AAR, replace .so, and recreate
    let aar_file = File::open(&target_aar)?;
    let mut archive = ZipArchive::new(aar_file)?;

    // Create a temp file for the new AAR
    let temp_aar = format!("{}.tmp", target_aar);
    let temp_file = File::create(&temp_aar)?;
    let mut zip_writer = ZipWriter::new(temp_file);

    spinner.finish();

    let spinner = create_spinner("Replacing library in AAR...");

    // Copy all files from original AAR, replacing the .so
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name().to_string();

        let options = FileOptions::default()
            .compression_method(file.compression())
            .unix_permissions(file.unix_mode().unwrap_or(0o644));

        if name == "jni/arm64-v8a/libgodot_android.so" {
            // Replace with our new .so
            zip_writer.start_file(&name, options)?;
            zip_writer.write_all(&source_data)?;
        } else if file.is_dir() {
            zip_writer.add_directory(&name, options)?;
        } else {
            // Copy as-is
            let mut data = Vec::new();
            file.read_to_end(&mut data)?;
            zip_writer.start_file(&name, options)?;
            zip_writer.write_all(&data)?;
        }
    }

    zip_writer.finish()?;
    drop(archive);

    spinner.finish();

    // Replace original with new
    fs::rename(&temp_aar, &target_aar)?;

    // Show sizes
    let source_size = fs::metadata(&source_so)?.len();
    let aar_size = fs::metadata(&target_aar)?.len();

    print_message(
        MessageType::Success,
        &format!(
            "Updated libgodot_android.so ({:.1} MB) in AAR ({:.1} MB)",
            source_size as f64 / 1024.0 / 1024.0,
            aar_size as f64 / 1024.0 / 1024.0
        ),
    );

    println!();
    print_message(MessageType::Info, "Next steps:");
    println!("  1. Build your project: cargo run -- build --target android");
    println!("  2. Export APK: cargo run -- export --target android --format apk");

    Ok(())
}
