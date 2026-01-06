use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;

use crate::android_godot_lib::GodotEngineConfig;
use crate::consts::{EXPORTS_FOLDER, GODOT_PROJECT_FOLDER, RUST_LIB_PROJECT_FOLDER, IOS_EXPORT_NAME};
use crate::export::import_assets;
use crate::path::get_godot_path;
use crate::ui::{create_spinner, print_message, print_section, MessageType};

/// Check if Xcode project exists
fn xcode_project_exists() -> bool {
    Path::new(&format!("{}/{}.xcodeproj", EXPORTS_FOLDER, IOS_EXPORT_NAME)).exists()
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

/// Get the Godot engine path for iOS, prompting user if not configured
fn get_godot_engine_path_for_ios() -> anyhow::Result<Option<String>> {
    let mut config = GodotEngineConfig::load();

    // Check if we have a saved path and it's still valid for iOS
    if let Some(ref path) = config.godot_engine_path {
        let godot_lib = format!("{}/bin/libgodot.ios.template_debug.arm64.a", path);
        if Path::new(&godot_lib).exists() {
            return Ok(Some(path.clone()));
        } else {
            print_message(
                MessageType::Warning,
                &format!(
                    "iOS library not found at saved path. Build it with:\n  cd {} && scons platform=ios target=template_debug arch=arm64",
                    path
                ),
            );
        }
    }

    // Prompt user for path
    print_message(
        MessageType::Info,
        "To update the Godot engine library, please provide the path to your Godot engine repository.",
    );
    print_message(
        MessageType::Info,
        "This should contain bin/libgodot.ios.template_debug.arm64.a",
    );
    print_message(
        MessageType::Info,
        "Leave empty to skip Godot engine update.",
    );

    print!("Godot engine path: ");
    std::io::stdout().flush()?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let input = input.trim();

    if input.is_empty() {
        return Ok(None);
    }

    let expanded_path = expand_path(input);

    // Validate path
    let godot_lib = format!("{}/bin/libgodot.ios.template_debug.arm64.a", expanded_path);
    if !Path::new(&godot_lib).exists() {
        print_message(
            MessageType::Warning,
            &format!("Godot library not found at: {}", godot_lib),
        );
        print_message(
            MessageType::Info,
            "Build it with: cd <godot-engine> && scons platform=ios target=template_debug arch=arm64",
        );
        return Ok(None);
    }

    // Save the path
    config.godot_engine_path = Some(expanded_path.clone());
    config.save()?;

    print_message(
        MessageType::Success,
        &format!("Godot engine path saved: {}", expanded_path),
    );

    Ok(Some(expanded_path))
}

/// Update Godot engine library
fn update_godot_lib(godot_engine_path: &str) -> anyhow::Result<()> {
    let source = format!(
        "{}/bin/libgodot.ios.template_debug.arm64.a",
        godot_engine_path
    );
    let target = format!(
        "{}/{}.xcframework/ios-arm64/libgodot.a",
        EXPORTS_FOLDER, IOS_EXPORT_NAME
    );

    if !Path::new(&source).exists() {
        print_message(MessageType::Warning, "Godot library not found, skipping");
        return Ok(());
    }

    let spinner = create_spinner("Updating Godot engine library...");
    fs::copy(&source, &target)?;
    spinner.finish();

    let size = fs::metadata(&source)?.len();
    print_message(
        MessageType::Success,
        &format!("libgodot.a ({:.1} MB)", size as f64 / 1024.0 / 1024.0),
    );

    Ok(())
}

/// Update dcl-godot-ios plugin
fn update_plugin() -> anyhow::Result<()> {
    let source = "plugins/dcl-godot-ios/bin/dcl_godot_ios-device.release_debug.a";
    let target = format!(
        "{}/{}/dylibs/ios/plugins/dcl_godot_ios/dcl_godot_ios.xcframework/ios-arm64/dcl_godot_ios-device.release_debug.a",
        EXPORTS_FOLDER, IOS_EXPORT_NAME
    );

    if !Path::new(source).exists() {
        print_message(
            MessageType::Warning,
            "Plugin library not found. Build it with:",
        );
        print_message(
            MessageType::Info,
            "  cd plugins/dcl-godot-ios && ./scripts/build.sh",
        );
        return Ok(());
    }

    let spinner = create_spinner("Updating dcl-godot-ios plugin...");
    fs::copy(source, &target)?;
    spinner.finish();

    let size = fs::metadata(source)?.len();
    print_message(
        MessageType::Success,
        &format!("dcl_godot_ios plugin ({:.1} KB)", size as f64 / 1024.0),
    );

    Ok(())
}

/// Update Rust library (libdclgodot)
fn update_rust_lib() -> anyhow::Result<()> {
    let source = format!(
        "{}target/libdclgodot_ios/libdclgodot.dylib",
        RUST_LIB_PROJECT_FOLDER
    );
    let target = format!(
        "{}/{}/lib/target/libdclgodot_ios/libdclgodot.framework/libdclgodot",
        EXPORTS_FOLDER, IOS_EXPORT_NAME
    );

    if !Path::new(&source).exists() {
        print_message(
            MessageType::Warning,
            "Rust library not found. Build it with:",
        );
        print_message(MessageType::Info, "  cargo run -- build --target ios");
        return Ok(());
    }

    // Check if target directory exists
    let target_dir = Path::new(&target).parent().unwrap();
    if !target_dir.exists() {
        print_message(
            MessageType::Warning,
            "Target framework not found. Export the Xcode project first with:",
        );
        print_message(MessageType::Info, "  cargo run -- export --target ios");
        return Ok(());
    }

    let spinner = create_spinner("Updating Rust library (libdclgodot)...");
    fs::copy(&source, &target)?;
    spinner.finish();

    // Fix the install name to use @rpath instead of absolute path
    let output = Command::new("otool").args(["-D", &target]).output()?;
    let install_name = String::from_utf8_lossy(&output.stdout);
    let current_name = install_name.lines().last().unwrap_or("");

    if !current_name.starts_with("@rpath") {
        print_message(MessageType::Step, "Fixing install name...");
        let status = Command::new("install_name_tool")
            .args(["-id", "@rpath/libdclgodot.framework/libdclgodot", &target])
            .status()?;

        if !status.success() {
            print_message(MessageType::Warning, "Failed to fix install name");
        }
    }

    let size = fs::metadata(&source)?.len();
    print_message(
        MessageType::Success,
        &format!("libdclgodot ({:.1} MB)", size as f64 / 1024.0 / 1024.0),
    );

    Ok(())
}

/// Update PCK file by re-exporting
fn update_pck() -> anyhow::Result<()> {
    let pck_path = format!("{}/{}.pck", EXPORTS_FOLDER, IOS_EXPORT_NAME);
    let program = get_godot_path();

    // Remove old PCK
    if Path::new(&pck_path).exists() {
        fs::remove_file(&pck_path)?;
    }

    let spinner = create_spinner("Re-exporting PCK file...");

    // Export just the PCK using --export-pack
    let status = Command::new(&program)
        .args([
            "--headless",
            "--export-pack",
            "ios",
            &format!("../exports/{}.pck", IOS_EXPORT_NAME),
        ])
        .current_dir(GODOT_PROJECT_FOLDER)
        .status();

    spinner.finish();

    match status {
        Ok(s) if s.success() && Path::new(&pck_path).exists() => {
            let size = fs::metadata(&pck_path)?.len();
            print_message(
                MessageType::Success,
                &format!("{}.pck ({:.1} MB)", IOS_EXPORT_NAME, size as f64 / 1024.0 / 1024.0),
            );
        }
        _ => {
            print_message(
                MessageType::Warning,
                "PCK export may have failed, trying with import first...",
            );
            // Fallback: import assets first
            import_assets();

            let status = Command::new(&program)
                .args([
                    "--headless",
                    "--export-pack",
                    "ios",
                    &format!("../exports/{}.pck", IOS_EXPORT_NAME),
                ])
                .current_dir(GODOT_PROJECT_FOLDER)
                .status()?;

            if status.success() && Path::new(&pck_path).exists() {
                let size = fs::metadata(&pck_path)?.len();
                print_message(
                    MessageType::Success,
                    &format!("{}.pck ({:.1} MB)", IOS_EXPORT_NAME, size as f64 / 1024.0 / 1024.0),
                );
            } else {
                print_message(MessageType::Error, "Failed to generate PCK");
            }
        }
    }

    Ok(())
}

/// Main entry point for update-ios-xcode command
pub fn update_ios_xcode(
    update_godot: bool,
    update_plugin: bool,
    update_lib: bool,
    update_pck: bool,
) -> anyhow::Result<()> {
    print_section("Updating iOS Xcode Project");

    // Check platform
    if std::env::consts::OS != "macos" {
        return Err(anyhow::anyhow!("This command is only supported on macOS"));
    }

    // Check if Xcode project exists, if not run export first
    if !xcode_project_exists() {
        print_message(
            MessageType::Warning,
            "Xcode project not found. Running export first...",
        );
        crate::export::export(Some("ios"), "ipa", false)?;

        if !xcode_project_exists() {
            return Err(anyhow::anyhow!(
                "Failed to create Xcode project. Check export logs."
            ));
        }
    }

    // Determine what to update (default: all)
    let update_all = !update_godot && !update_plugin && !update_lib && !update_pck;

    // Update Godot library
    if update_all || update_godot {
        if let Some(godot_path) = get_godot_engine_path_for_ios()? {
            update_godot_lib(&godot_path)?;
        }
    }

    // Update plugin
    if update_all || update_plugin {
        self::update_plugin()?;
    }

    // Update Rust library
    if update_all || update_lib {
        update_rust_lib()?;
    }

    // Update PCK
    if update_all || update_pck {
        self::update_pck()?;
    }

    println!();
    print_message(MessageType::Success, "Xcode project updated!");
    println!();
    print_message(MessageType::Info, "Next steps:");
    println!("  1. Open Xcode: open exports/{}.xcodeproj", IOS_EXPORT_NAME);
    println!("  2. Build and run on device (Cmd+R)");

    Ok(())
}
