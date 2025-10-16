use crate::dependencies::BuildStatus;
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
            if let Some(tool_path) = crate::helpers::get_tool_path(tool) {
                if tool_path.starts_with(".bin") {
                    print_message(
                        MessageType::Success,
                        &format!(
                            "{} - {} (using local: {})",
                            tool,
                            description,
                            tool_path.display()
                        ),
                    );
                } else {
                    print_message(
                        MessageType::Success,
                        &format!("{} - {} (system)", tool, description),
                    );
                }
            } else {
                print_message(MessageType::Success, &format!("{} - {}", tool, description));
            }
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

    // Check build status
    print_divider();
    print_section("Build Status");
    check_build_status();

    // Check environment variables
    print_divider();
    print_section("Environment Variables");
    check_environment_variables();

    // Check version consistency
    print_divider();
    if let Err(e) = crate::version_check::run_version_check() {
        print_message(MessageType::Error, &format!("Version check failed: {}", e));
        all_tools_ok = false;
    }

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
            .args(["target", "list", "--installed"])
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
    if crate::helpers::is_tool_installed("godot") {
        print_message(MessageType::Success, "Godot binary found");
    } else {
        print_message(MessageType::Warning, "Godot binary not found");
        println!("  Run: cargo run -- install");
    }

    // Check export templates per platform
    print_message(MessageType::Info, "Export templates status:");
    if let Some(templates_path) = crate::install_dependency::godot_export_templates_path() {
        let platforms = vec![
            (
                "android",
                vec![
                    "android_debug.apk",
                    "android_release.apk",
                    "android_source.zip",
                ],
            ),
            ("ios", vec!["ios.zip"]),
            ("linux", vec!["linux_debug.x86_64", "linux_release.x86_64"]),
            ("macos", vec!["macos.zip"]),
            (
                "windows",
                vec!["windows_debug_x86_64.exe", "windows_release_x86_64.exe"],
            ),
        ];

        for (platform, files) in platforms {
            let mut all_found = true;
            for file in &files {
                let file_path = Path::new(&templates_path).join(file);
                if !file_path.exists() {
                    all_found = false;
                    break;
                }
            }

            if all_found {
                print_message(MessageType::Success, &format!("  {} - Installed", platform));
            } else {
                print_message(
                    MessageType::Info,
                    &format!("  {} - Not installed", platform),
                );
            }
        }

        print_message(
            MessageType::Info,
            "To install templates for specific platforms:",
        );
        println!("  Run: cargo run -- install --targets <platform1>,<platform2>,...");
        println!("  Available platforms: android, ios, linux, macos, windows");
    } else {
        print_message(
            MessageType::Warning,
            "Could not determine export templates path",
        );
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
                .args(["target", "list", "--installed"])
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
                println!("  Run: cargo run -- install --targets android");
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
                .args(["target", "list", "--installed"])
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

fn check_build_status() {
    let build_status = BuildStatus::check();
    let platform = get_platform_info();

    // Host platform
    let host_platform = &platform.os;
    if build_status.host_built {
        print_message(
            MessageType::Success,
            &format!("Host platform ({}) - Built", host_platform),
        );
    } else {
        print_message(
            MessageType::Warning,
            &format!("Host platform ({}) - Not built", host_platform),
        );
        println!("  Run: cargo run -- build");
    }

    // Android
    if build_status.android_built {
        print_message(MessageType::Success, "Android (ARM64) - Built");
    } else {
        print_message(MessageType::Info, "Android (ARM64) - Not built");
        println!("  Run: cargo run -- build --target android");
    }

    // iOS
    if build_status.ios_built {
        print_message(MessageType::Success, "iOS - Built");
    } else {
        print_message(MessageType::Info, "iOS - Not built");
        if platform.os == "macos" {
            println!("  Run: cargo run -- build --target ios");
        }
    }

    // Windows
    if build_status.windows_built {
        print_message(MessageType::Success, "Windows - Built");
    } else {
        print_message(MessageType::Info, "Windows - Not built");
        println!("  Run: cargo run -- build --target windows");
    }

    // macOS
    if build_status.macos_built {
        print_message(MessageType::Success, "macOS - Built");
    } else {
        print_message(MessageType::Info, "macOS - Not built");
        if platform.os == "macos" {
            println!("  Run: cargo run -- build --target macos");
        }
    }
}

fn check_environment_variables() {
    let mut vars_to_check = vec![
        ("ANDROID_HOME", "Android SDK location"),
        ("ANDROID_SDK", "Android SDK location (alternative)"),
        ("ANDROID_NDK_HOME", "Android NDK location"),
        ("ANDROID_NDK", "Android NDK location (alternative)"),
        ("HOME", "User home directory"),
    ];

    // Add Windows-specific environment variables
    if cfg!(windows) {
        vars_to_check.push(("LIBCLANG_PATH", "Clang library path for bindgen"));
    }

    for (var, description) in vars_to_check {
        match env::var(var) {
            Ok(value) => {
                // Special validation for LIBCLANG_PATH
                if var == "LIBCLANG_PATH" {
                    let path = Path::new(&value);
                    let libclang_dll = path.join("libclang.dll");
                    if path.exists() && libclang_dll.exists() {
                        print_message(
                            MessageType::Success,
                            &format!("{}: {} ({}) ✓", var, value, description),
                        );
                    } else if path.exists() {
                        print_message(
                            MessageType::Warning,
                            &format!(
                                "{}: {} ({}) - libclang.dll not found",
                                var, value, description
                            ),
                        );
                    } else {
                        print_message(
                            MessageType::Error,
                            &format!("{}: {} ({}) - Path does not exist", var, value, description),
                        );
                    }
                } else {
                    print_message(
                        MessageType::Success,
                        &format!("{}: {} ({})", var, value, description),
                    );
                }
            }
            Err(_) => {
                if var == "LIBCLANG_PATH" && cfg!(windows) {
                    // Try to auto-detect LIBCLANG_PATH
                    if let Some(detected_path) = crate::platform::find_libclang_path() {
                        print_message(
                            MessageType::Success,
                            &format!(
                                "{}: Not set but auto-detected at: {} ({})",
                                var, detected_path, description
                            ),
                        );
                        println!("  To use this path, set it with:");
                        println!("  set LIBCLANG_PATH={}", detected_path);
                        println!(
                            "  Or add it to your system environment variables for permanent use."
                        );
                    } else {
                        print_message(
                            MessageType::Warning,
                            &format!(
                                "{}: Not set ({}) - Required for Windows builds",
                                var, description
                            ),
                        );
                        println!("  Common locations:");
                        println!("  - C:\\Program Files\\Microsoft Visual Studio\\2022\\Community\\VC\\Tools\\LLVM\\x64\\bin");
                        println!("  - C:\\Program Files\\LLVM\\bin");
                        println!("\n  We tried to auto-detect using vswhere but couldn't find a valid installation.");
                        println!(
                            "  Make sure Visual Studio is installed with C++ development tools."
                        );
                    }
                } else {
                    print_message(
                        MessageType::Info,
                        &format!("{}: Not set ({})", var, description),
                    );
                }
            }
        }
    }
}
