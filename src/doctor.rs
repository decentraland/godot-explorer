use crate::platform::{
    check_android_sdk, check_development_dependencies, check_ios_development, check_required_tools,
    get_install_command, get_platform_info, suggest_install,
};
use crate::ui::{print_divider, print_message, print_section, MessageType};
use std::env;
use std::path::Path;

/// Run system health check
pub fn run_doctor() -> anyhow::Result<()> {
    print_section("System Health Check");

    // Platform info
    let platform = get_platform_info();
    print_message(
        MessageType::Info,
        &format!("Platform: {}", platform.display_name),
    );
    print_divider();

    // Check required tools
    print_section("Required Tools");
    let mut all_tools_ok = true;

    for (tool, available, description) in check_required_tools() {
        if available {
            print_message(MessageType::Success, &format!("{} - {}", tool, description));
        } else {
            print_message(
                MessageType::Error,
                &format!("{} - {} (NOT FOUND)", tool, description),
            );
            suggest_install(tool);
            all_tools_ok = false;
        }
    }

    // Check development dependencies
    print_divider();
    print_section("Development Dependencies");
    let mut missing_deps = false;

    for (dep, available, description) in check_development_dependencies() {
        if available {
            print_message(MessageType::Success, &format!("{} - {}", dep, description));
        } else {
            print_message(
                MessageType::Error,
                &format!("{} - {} (NOT FOUND)", dep, description),
            );
            missing_deps = true;
        }
    }

    if missing_deps {
        if let Some(install_cmd) = get_install_command() {
            print_message(
                MessageType::Info,
                "To install all missing dependencies, run:",
            );
            println!("\n{}\n", install_cmd);
        }
    }

    // Check Rust targets
    print_divider();
    print_section("Rust Targets");
    check_rust_targets();

    // Check Godot installation
    print_divider();
    print_section("Godot Engine");
    check_godot_installation();

    // Check Android development
    print_divider();
    print_section("Android Development");
    check_android_setup();

    // Check iOS development (macOS only)
    if platform.os == "macos" {
        print_divider();
        print_section("iOS Development");
        check_ios_setup();
    }

    // Check environment variables
    print_divider();
    print_section("Environment Variables");
    check_environment_variables();

    // Summary
    print_divider();
    print_section("Summary");
    if all_tools_ok && !missing_deps {
        print_message(
            MessageType::Success,
            "All required tools and dependencies are installed!",
        );
    } else {
        print_message(
            MessageType::Warning,
            "Some tools or dependencies are missing. Please install them before proceeding.",
        );

        if missing_deps && get_install_command().is_some() {
            print_message(
                MessageType::Info,
                "Run the command shown above to install missing dependencies.",
            );
        }
    }

    Ok(())
}

fn check_rust_targets() {
    // Check if common targets are installed
    let targets_to_check = vec![
        ("x86_64-pc-windows-gnu", "Windows 64-bit"),
        ("x86_64-apple-darwin", "macOS Intel"),
        ("aarch64-apple-darwin", "macOS Apple Silicon"),
        ("x86_64-unknown-linux-gnu", "Linux 64-bit"),
        ("aarch64-linux-android", "Android ARM64"),
        ("aarch64-apple-ios", "iOS ARM64"),
    ];

    for (target, description) in targets_to_check {
        let output = std::process::Command::new("rustup")
            .args(&["target", "list", "--installed"])
            .output();

        match output {
            Ok(output) => {
                let installed = String::from_utf8_lossy(&output.stdout);
                if installed.contains(target) {
                    print_message(
                        MessageType::Success,
                        &format!("{} ({})", target, description),
                    );
                } else {
                    print_message(
                        MessageType::Info,
                        &format!("{} ({}) - Not installed", target, description),
                    );
                    println!("  To install: rustup target add {}", target);
                }
            }
            Err(_) => {
                print_message(MessageType::Error, "Failed to check Rust targets");
                break;
            }
        }
    }
}

fn check_godot_installation() {
    let godot_path = Path::new(".bin/godot/godot4_bin");
    if godot_path.exists() {
        print_message(MessageType::Success, "Godot binary found");
    } else {
        print_message(MessageType::Warning, "Godot binary not found");
        println!("  Run: cargo run -- install");
    }

    // Check export templates
    if let Some(templates_path) = crate::install_dependency::godot_export_templates_path() {
        if Path::new(&templates_path).exists() {
            print_message(MessageType::Success, "Godot export templates found");
        } else {
            print_message(MessageType::Info, "Godot export templates not installed");
            println!("  Run: cargo run -- install --platforms <platform>");
        }
    }
}

fn check_android_setup() {
    match check_android_sdk() {
        Ok(ndk_path) => {
            print_message(
                MessageType::Success,
                &format!("Android NDK found: {}", ndk_path),
            );

            // Check if Android target is installed
            let output = std::process::Command::new("rustup")
                .args(&["target", "list", "--installed"])
                .output();

            if let Ok(output) = output {
                let installed = String::from_utf8_lossy(&output.stdout);
                if installed.contains("aarch64-linux-android") {
                    print_message(MessageType::Success, "Android Rust target installed");
                } else {
                    print_message(MessageType::Warning, "Android Rust target not installed");
                    println!("  Run: rustup target add aarch64-linux-android");
                }
            }

            // Check Android dependencies
            let android_deps_path = Path::new(".bin/android_deps");
            if android_deps_path.exists() {
                print_message(MessageType::Success, "Android dependencies downloaded");
            } else {
                print_message(MessageType::Info, "Android dependencies not downloaded");
                println!("  Run: cargo run -- install --platforms android");
            }
        }
        Err(msg) => {
            print_message(MessageType::Warning, &msg);
            suggest_install("android-ndk");
        }
    }
}

fn check_ios_setup() {
    match check_ios_development() {
        Ok(_) => {
            print_message(MessageType::Success, "Xcode Command Line Tools found");

            // Check if iOS target is installed
            let output = std::process::Command::new("rustup")
                .args(&["target", "list", "--installed"])
                .output();

            if let Ok(output) = output {
                let installed = String::from_utf8_lossy(&output.stdout);
                if installed.contains("aarch64-apple-ios") {
                    print_message(MessageType::Success, "iOS Rust target installed");
                } else {
                    print_message(MessageType::Warning, "iOS Rust target not installed");
                    println!("  Run: rustup target add aarch64-apple-ios");
                }
            }
        }
        Err(msg) => {
            print_message(MessageType::Warning, &msg);
        }
    }
}

fn check_environment_variables() {
    let vars_to_check = vec![
        ("ANDROID_HOME", "Android SDK location"),
        ("ANDROID_SDK", "Android SDK location (alternative)"),
        ("ANDROID_NDK_HOME", "Android NDK location"),
        ("ANDROID_NDK", "Android NDK location (alternative)"),
        ("HOME", "User home directory"),
    ];

    for (var, description) in vars_to_check {
        match env::var(var) {
            Ok(value) => {
                print_message(
                    MessageType::Success,
                    &format!("{}: {} ({})", var, value, description),
                );
            }
            Err(_) => {
                print_message(
                    MessageType::Info,
                    &format!("{}: Not set ({})", var, description),
                );
            }
        }
    }
}
