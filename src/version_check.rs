use crate::ui::{print_message, print_section, MessageType};
use std::fs;
use std::path::Path;

/// Check that the marketing version (SemVer) is consistent across the places that ship it:
/// `lib/Cargo.toml` `version`, Android `version/name`, and iOS `application/short_version`.
///
/// The store BUILD NUMBER (Android `version/code` / iOS `application/version`) is intentionally
/// NOT checked here: it is stamped at export time by the `build_number` module (days-since-2020),
/// so the committed values are placeholders and would never match the marketing SemVer.
pub fn run_version_check() -> anyhow::Result<()> {
    print_section("Version Consistency Check");

    // Marketing SemVer source of truth: lib/Cargo.toml `version`.
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
        .trim_matches('"')
        .to_string();

    print_message(
        MessageType::Info,
        &format!("Cargo.toml version (marketing SemVer): {}", cargo_version),
    );

    // Parse export_presets.cfg marketing strings (Android version/name, iOS short_version).
    let export_presets_path = Path::new("godot/export_presets.cfg");
    let export_presets_content =
        fs::read_to_string(export_presets_path).expect("Failed to read export_presets.cfg");

    let mut android_version_name = None;
    let mut ios_short_version = None;

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

        if in_android_preset && line.starts_with("version/name=") && android_version_name.is_none()
        {
            android_version_name = Some(parse_quoted_value(line));
        } else if in_ios_preset
            && line.starts_with("application/short_version=")
            && ios_short_version.is_none()
        {
            ios_short_version = Some(parse_quoted_value(line));
        }
    }

    let android_version_name = android_version_name.expect("Failed to find Android version/name");
    let ios_short_version =
        ios_short_version.expect("Failed to find iOS application/short_version");

    print_message(
        MessageType::Info,
        &format!("Android version/name: {}", android_version_name),
    );
    print_message(
        MessageType::Info,
        &format!("iOS application/short_version: {}", ios_short_version),
    );

    // Compare marketing versions
    let mut all_match = true;

    if android_version_name != cargo_version {
        print_message(
            MessageType::Error,
            &format!(
                "Android version/name ({}) does not match Cargo.toml version ({})",
                android_version_name, cargo_version
            ),
        );
        all_match = false;
    }

    if ios_short_version != cargo_version {
        print_message(
            MessageType::Error,
            &format!(
                "iOS application/short_version ({}) does not match Cargo.toml version ({})",
                ios_short_version, cargo_version
            ),
        );
        all_match = false;
    }

    if all_match {
        print_message(
            MessageType::Success,
            &format!(
                "✓ Marketing version matches: Cargo.toml={}, Android version/name={}, iOS short_version={}",
                cargo_version, android_version_name, ios_short_version
            ),
        );
        Ok(())
    } else {
        Err(anyhow::anyhow!("Version mismatch detected"))
    }
}

/// Extract the value from a `key="value"` (or `key=value`) cfg line.
fn parse_quoted_value(line: &str) -> String {
    line.split('=')
        .nth(1)
        .expect("Failed to parse cfg value")
        .trim()
        .trim_matches('"')
        .to_string()
}
