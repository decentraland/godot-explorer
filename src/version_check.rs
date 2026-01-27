use crate::ui::{print_message, print_section, MessageType};
use std::fs;
use std::path::Path;

/// Check that version codes across Cargo.toml and export_presets.cfg match
pub fn run_version_check() -> anyhow::Result<()> {
    print_section("Version Consistency Check");

    // Parse Cargo.toml version
    let cargo_toml_path = Path::new("lib/Cargo.toml");
    let cargo_toml_content =
        fs::read_to_string(cargo_toml_path).expect("Failed to read Cargo.toml");

    let cargo_version = cargo_toml_content
        .lines()
        .find(|line| line.starts_with("version = "))
        .expect("Failed to find version in Cargo.toml")
        .split('=')
        .nth(1)
        .expect("Failed to parse version")
        .trim()
        .trim_matches('"');

    print_message(
        MessageType::Info,
        &format!("Cargo.toml version: {}", cargo_version),
    );

    // Extract the minor version number (e.g., "0.32.0" -> 32)
    let version_parts: Vec<&str> = cargo_version.split('.').collect();
    let expected_version_code = version_parts
        .get(1)
        .expect("Version should have at least 2 parts")
        .parse::<u32>()
        .expect("Failed to parse version code");

    print_message(
        MessageType::Info,
        &format!("Expected version code: {}", expected_version_code),
    );

    // Parse export_presets.cfg
    let export_presets_path = Path::new("godot/export_presets.cfg");

    let export_presets_content =
        fs::read_to_string(export_presets_path).expect("Failed to read export_presets.cfg");

    // Find all version/code entries (Android and iOS)
    let mut android_version_code = None;
    let mut ios_version = None;

    let mut in_android_preset = false;
    let mut in_ios_preset = false;

    for line in export_presets_content.lines() {
        if line.contains("name=\"android\"") {
            in_android_preset = true;
            in_ios_preset = false;
        } else if line.contains("name=\"ios\"") {
            in_ios_preset = true;
            in_android_preset = false;
        } else if line.starts_with("[preset.") && !line.contains(".options]") {
            // Reset flags on new preset (but not on [preset.X.options])
            in_android_preset = false;
            in_ios_preset = false;
        }

        if in_android_preset && line.starts_with("version/code=") && android_version_code.is_none()
        {
            android_version_code = Some(
                line.split('=')
                    .nth(1)
                    .expect("Failed to parse Android version/code")
                    .trim()
                    .parse::<u32>()
                    .expect("Failed to parse Android version/code as u32"),
            );
        } else if in_ios_preset && line.starts_with("application/version=") && ios_version.is_none()
        {
            ios_version = Some(
                line.split('=')
                    .nth(1)
                    .expect("Failed to parse iOS application/version")
                    .trim()
                    .trim_matches('"')
                    .parse::<u32>()
                    .expect("Failed to parse iOS application/version as u32"),
            );
        }
    }

    let android_version_code = android_version_code.expect("Failed to find Android version/code");
    let ios_version = ios_version.expect("Failed to find iOS application/version");

    print_message(
        MessageType::Info,
        &format!("Android version/code: {}", android_version_code),
    );
    print_message(
        MessageType::Info,
        &format!("iOS application/version: {}", ios_version),
    );

    // Compare versions
    let mut all_match = true;

    if android_version_code != expected_version_code {
        print_message(
            MessageType::Error,
            &format!(
                "Android version/code ({}) does not match Cargo.toml version code ({})",
                android_version_code, expected_version_code
            ),
        );
        all_match = false;
    }

    if ios_version != expected_version_code {
        print_message(
            MessageType::Error,
            &format!(
                "iOS application/version ({}) does not match Cargo.toml version code ({})",
                ios_version, expected_version_code
            ),
        );
        all_match = false;
    }

    if all_match {
        print_message(
            MessageType::Success,
            &format!(
                "✓ All versions match: Cargo.toml={}, Android={}, iOS={}",
                expected_version_code, android_version_code, ios_version
            ),
        );
        Ok(())
    } else {
        Err(anyhow::anyhow!("Version mismatch detected"))
    }
}
