use crate::consts::ANDROID_NDK_VERSION;
use crate::helpers::get_android_ndk_path;
use crate::ui::{print_message, MessageType};
use std::env;

/// Platform information
#[derive(Debug, Clone)]
pub struct PlatformInfo {
    pub os: String,
    #[allow(dead_code)]
    pub arch: String,
    pub display_name: String,
}

/// Detect Linux package manager
pub fn detect_linux_package_manager() -> Option<&'static str> {
    if check_command("apt-get") {
        Some("apt")
    } else if check_command("pacman") {
        Some("pacman")
    } else if check_command("dnf") {
        Some("dnf")
    } else if check_command("yum") {
        Some("yum")
    } else if check_command("zypper") {
        Some("zypper")
    } else {
        None
    }
}

/// Get current platform information
pub fn get_platform_info() -> PlatformInfo {
    let os = env::consts::OS;
    let arch = env::consts::ARCH;

    let display_name = match (os, arch) {
        ("linux", "x86_64") => "Linux (64-bit)",
        ("linux", "aarch64") => "Linux (ARM64)",
        ("windows", "x86_64") => "Windows (64-bit)",
        ("macos", "x86_64") => "macOS (Intel)",
        ("macos", "aarch64") => "macOS (Apple Silicon)",
        _ => "Unknown platform",
    };

    PlatformInfo {
        os: os.to_string(),
        arch: arch.to_string(),
        display_name: display_name.to_string(),
    }
}

/// Check if a command exists in PATH or in local .bin directory
pub fn check_command(cmd: &str) -> bool {
    crate::helpers::is_tool_installed(cmd)
}

/// Check if Android SDK is properly configured
pub fn check_android_sdk() -> Result<String, String> {
    // Check environment variables in order of preference
    if let Ok(ndk_home) = env::var("ANDROID_NDK_HOME") {
        if std::path::Path::new(&ndk_home).exists() {
            return Ok(ndk_home);
        }
    }

    if let Ok(ndk) = env::var("ANDROID_NDK") {
        if std::path::Path::new(&ndk).exists() {
            return Ok(ndk);
        }
    }

    if let Ok(sdk) = env::var("ANDROID_SDK") {
        let ndk_path = get_android_ndk_path(&sdk);
        if ndk_path.exists() {
            return Ok(ndk_path.to_string_lossy().to_string());
        }
    }

    if let Ok(home) = env::var("ANDROID_HOME") {
        let ndk_path = get_android_ndk_path(&home);
        if ndk_path.exists() {
            return Ok(ndk_path.to_string_lossy().to_string());
        }
    }

    // Check default location
    if let Ok(home) = env::var("HOME") {
        let android_sdk = format!("{}/Android/Sdk", home);
        let ndk_path = get_android_ndk_path(&android_sdk);
        if ndk_path.exists() {
            return Ok(ndk_path.to_string_lossy().to_string());
        }
    }

    Err(format!(
        "Android NDK not found. Please install Android NDK {}",
        ANDROID_NDK_VERSION
    ))
}

/// Check if iOS development is available (macOS only)
pub fn check_ios_development() -> Result<(), String> {
    let platform = get_platform_info();
    if platform.os != "macos" {
        return Err("iOS development is only available on macOS".to_string());
    }

    // Check for Xcode command line tools
    if !check_command("xcrun") {
        return Err(
            "Xcode Command Line Tools not found. Install with: xcode-select --install".to_string(),
        );
    }

    Ok(())
}

/// Get required tools for current platform
pub fn get_required_tools() -> Vec<(&'static str, &'static str)> {
    let mut tools = vec![
        ("rustup", "Rust toolchain manager"),
        ("cargo", "Rust package manager"),
        ("protoc", "Protocol Buffers compiler"),
    ];

    let platform = get_platform_info();
    if platform.os == "windows" {
        // Windows-specific tools
        tools.push(("curl", "Command line download tool"));
    } else {
        // Unix-like tools
        tools.push(("curl", "Command line download tool"));
    }

    tools
}

/// Check all required tools and report status
pub fn check_required_tools() -> Vec<(&'static str, bool, &'static str)> {
    get_required_tools()
        .into_iter()
        .map(|(tool, desc)| (tool, check_command(tool), desc))
        .collect()
}

/// Validate platform for target build
pub fn validate_platform_for_target(target: &str) -> Result<(), anyhow::Error> {
    let platform = get_platform_info();

    match target {
        "ios" => {
            if platform.os != "macos" {
                return Err(anyhow::anyhow!("iOS builds can only be created on macOS"));
            }
            check_ios_development().map_err(|e| anyhow::anyhow!(e))?;
        }
        "android" | "quest" => {
            check_android_sdk().map_err(|e| anyhow::anyhow!(e))?;
        }
        "windows" | "win64" => {
            // Can be built from any platform with proper tools
        }
        "linux" => {
            // Can be built from any platform with proper tools
        }
        "macos" => {
            if platform.os != "macos" {
                print_message(
                    MessageType::Warning,
                    "Cross-compiling for macOS from non-macOS platform may have limitations",
                );
            }
        }
        _ => return Err(anyhow::anyhow!("Unknown target platform: {}", target)),
    }

    Ok(())
}

/// Get Android target architecture
#[allow(dead_code)]
pub fn get_android_target_arch() -> &'static str {
    // For now, we only support ARM64 Android
    "aarch64-linux-android"
}

/// Get iOS target architecture
#[allow(dead_code)]
pub fn get_ios_target_arch() -> &'static str {
    "aarch64-apple-ios"
}

/// Check development dependencies based on OS
pub fn check_development_dependencies() -> Vec<(&'static str, bool, &'static str)> {
    let platform = get_platform_info();

    match platform.os.as_str() {
        "linux" => vec![
            // Audio/Video deps
            (
                "libasound2-dev",
                check_pkg_config("alsa"),
                "ALSA sound library",
            ),
            ("libudev-dev", check_pkg_config("libudev"), "udev library"),
            // FFmpeg is now installed locally via cargo run -- install
            // No need to check for system FFmpeg dev packages
            // LiveKit deps
            ("libssl-dev", check_pkg_config("openssl"), "OpenSSL library"),
            ("libx11-dev", check_pkg_config("x11"), "X11 library"),
            ("libgl1-mesa-dev", check_pkg_config("gl"), "OpenGL library"),
            (
                "libxext-dev",
                check_pkg_config("xext"),
                "X11 extension library",
            ),
            // Build tools
            ("clang", check_command("clang"), "C/C++ compiler"),
            (
                "pkg-config",
                check_command("pkg-config"),
                "Package configuration tool",
            ),
        ],
        "macos" => vec![
            (
                "pkg-config",
                check_command("pkg-config"),
                "Package configuration tool",
            ),
            // FFmpeg is now installed locally via cargo run -- install
            // No need to check for system FFmpeg packages
        ],
        "windows" => vec![
            // Windows-specific checks
            (
                "LIBCLANG_PATH",
                check_libclang_path(),
                "Clang library path for bindgen",
            ),
            (
                "cl.exe",
                check_command("cl") || check_vs_installed(),
                "Visual Studio C++ compiler",
            ),
        ],
        _ => vec![],
    }
}

/// Check if a library is available via pkg-config
fn check_pkg_config(lib: &str) -> bool {
    if !check_command("pkg-config") {
        return false;
    }

    let mut cmd = std::process::Command::new("pkg-config");

    // FFmpeg is now installed locally, no need to check system paths

    cmd.args(["--exists", lib])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// Check if LIBCLANG_PATH is set and valid
fn check_libclang_path() -> bool {
    // First check if LIBCLANG_PATH is already set
    if let Ok(libclang_path) = env::var("LIBCLANG_PATH") {
        // Check if the path exists and contains libclang.dll
        let path = std::path::Path::new(&libclang_path);
        if path.exists() {
            let libclang_dll = path.join("libclang.dll");
            return libclang_dll.exists();
        }
    }

    // If not set, try to auto-detect it
    if find_libclang_path().is_some() {
        return true;
    }

    false
}

/// Try to find LIBCLANG_PATH automatically using vswhere
pub fn find_libclang_path() -> Option<String> {
    // Only try on Windows
    if !cfg!(windows) {
        return None;
    }

    // Try using vswhere to find Visual Studio installation
    let vswhere_paths = vec![
        "C:\\Program Files (x86)\\Microsoft Visual Studio\\Installer\\vswhere.exe",
        "C:\\Program Files\\Microsoft Visual Studio\\Installer\\vswhere.exe",
    ];

    for vswhere_path in vswhere_paths {
        if std::path::Path::new(vswhere_path).exists() {
            if let Ok(output) = std::process::Command::new(vswhere_path)
                .args(["-latest", "-property", "installationPath"])
                .output()
            {
                if output.status.success() {
                    let install_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !install_path.is_empty() {
                        let libclang_path = format!("{}\\VC\\Tools\\LLVM\\x64\\bin", install_path);
                        let path = std::path::Path::new(&libclang_path);
                        if path.exists() && path.join("libclang.dll").exists() {
                            return Some(libclang_path);
                        }
                    }
                }
            }
        }
    }

    // Fallback: try common installation paths
    let common_paths = vec![
        "C:\\Program Files\\Microsoft Visual Studio\\2022\\Community\\VC\\Tools\\LLVM\\x64\\bin",
        "C:\\Program Files\\Microsoft Visual Studio\\2022\\Professional\\VC\\Tools\\LLVM\\x64\\bin",
        "C:\\Program Files\\Microsoft Visual Studio\\2022\\Enterprise\\VC\\Tools\\LLVM\\x64\\bin",
        "C:\\Program Files (x86)\\Microsoft Visual Studio\\2019\\Community\\VC\\Tools\\LLVM\\x64\\bin",
        "C:\\Program Files (x86)\\Microsoft Visual Studio\\2019\\Professional\\VC\\Tools\\LLVM\\x64\\bin",
        "C:\\Program Files (x86)\\Microsoft Visual Studio\\2019\\Enterprise\\VC\\Tools\\LLVM\\x64\\bin",
        "C:\\Program Files\\LLVM\\bin",
    ];

    for path_str in common_paths {
        let path = std::path::Path::new(path_str);
        if path.exists() && path.join("libclang.dll").exists() {
            return Some(path_str.to_string());
        }
    }

    None
}

/// Check if Visual Studio is installed
fn check_vs_installed() -> bool {
    // Check common Visual Studio installation paths
    let vs_paths = [
        "C:\\Program Files\\Microsoft Visual Studio\\2022\\Community",
        "C:\\Program Files\\Microsoft Visual Studio\\2022\\Professional",
        "C:\\Program Files\\Microsoft Visual Studio\\2022\\Enterprise",
        "C:\\Program Files (x86)\\Microsoft Visual Studio\\2019\\Community",
        "C:\\Program Files (x86)\\Microsoft Visual Studio\\2019\\Professional",
        "C:\\Program Files (x86)\\Microsoft Visual Studio\\2019\\Enterprise",
    ];

    vs_paths
        .iter()
        .any(|path| std::path::Path::new(path).exists())
}

/// Get installation command for missing dependencies
pub fn get_install_command() -> Option<String> {
    let platform = get_platform_info();

    match platform.os.as_str() {
        "linux" => {
            if let Some(pkg_manager) = detect_linux_package_manager() {
                match pkg_manager {
                    "apt" => Some(
                        "sudo apt-get update && sudo apt-get install -y \\\n  \
                         libasound2-dev libudev-dev \\\n  \
                         clang curl pkg-config \\\n  \
                         libssl-dev libx11-dev libgl1-mesa-dev libxext-dev"
                            .to_string(),
                    ),
                    "pacman" => Some(
                        "sudo pacman -S --needed \\\n  \
                         alsa-lib systemd-libs \\\n  \
                         clang curl pkgconf \\\n  \
                         openssl libx11 mesa libxext"
                            .to_string(),
                    ),
                    "dnf" => Some(
                        "sudo dnf install -y \\\n  \
                         alsa-lib-devel systemd-devel \\\n  \
                         clang curl pkg-config \\\n  \
                         openssl-devel libX11-devel mesa-libGL-devel libXext-devel"
                            .to_string(),
                    ),
                    _ => None,
                }
            } else {
                None
            }
        }
        "macos" => Some("brew install pkg-config".to_string()),
        _ => None,
    }
}

/// Get complete next steps instructions after installation
pub fn get_next_steps_instructions() -> String {
    let platform = get_platform_info();
    let mut instructions = String::new();

    // Check if there are missing dependencies
    let dev_deps = check_development_dependencies();
    let has_missing_deps = dev_deps.iter().any(|(_, available, _)| !available);

    if has_missing_deps {
        match platform.os.as_str() {
            "linux" => {
                instructions
                    .push_str("For Linux development, you need to install system dependencies:\n");
                if let Some(install_cmd) = get_install_command() {
                    instructions.push_str("\n# Install required development dependencies:\n");
                    instructions.push_str(&install_cmd);
                    instructions.push('\n');
                } else {
                    instructions
                        .push_str("\n# Check your package manager documentation for installing:\n");
                    instructions.push_str("  - ALSA and udev development libraries\n");
                    instructions.push_str("  - OpenSSL, X11, and OpenGL development libraries\n");
                    instructions.push_str("  - clang and pkg-config\n");
                }
            }
            "macos" => {
                instructions.push_str("For macOS development:\n");
                instructions.push_str("\n# Install Homebrew if not already installed:\n");
                instructions.push_str("# /bin/bash -c \"$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\"\n");

                if let Some(install_cmd) = get_install_command() {
                    instructions.push_str("\n# Install required development dependencies:\n");
                    instructions.push_str(&install_cmd);
                    instructions.push('\n');
                }

                // FFmpeg is now installed locally via cargo run -- install
                // No need for system environment setup
            }
            "windows" => {
                instructions.push_str("For Windows development:\n");
                instructions.push_str(
                    "\n# FFmpeg will be downloaded automatically during the build process.\n",
                );
                instructions.push_str(
                    "# Make sure you have Visual Studio with C++ development tools installed.\n",
                );
                instructions.push_str("\n# IMPORTANT: LIBCLANG_PATH environment variable:\n");
                instructions.push_str("# This is required for bindgen to work properly.\n");
                instructions.push_str("# The build system will try to auto-detect it from Visual Studio, but you can set it manually:\n");
                instructions.push_str(
                    "\n# Auto-detection uses vswhere to find Visual Studio's LLVM installation.\n",
                );
                instructions.push_str("# If auto-detection fails, set it manually:\n");
                instructions.push_str("# set LIBCLANG_PATH=C:\\Program Files\\Microsoft Visual Studio\\2022\\Community\\VC\\Tools\\LLVM\\x64\\bin\n");
                instructions.push_str("\n# Or if you have LLVM installed separately:\n");
                instructions.push_str("# set LIBCLANG_PATH=C:\\Program Files\\LLVM\\bin\n");
                instructions.push_str("\n# To set it permanently, use System Environment Variables in Windows Settings.\n");
            }
            _ => {}
        }
    }

    instructions.push_str("\n# To verify your setup, run:\n");
    instructions.push_str("cargo run -- doctor\n");
    instructions.push_str("\n# To build the project:\n");
    instructions.push_str("cargo run -- build\n");
    instructions.push_str("\n# To run the Godot editor:\n");
    instructions.push_str("cargo run -- run -e\n");

    instructions
}

/// Suggest how to install missing dependency
pub fn suggest_install(tool: &str) {
    use crate::ui::print_install_instructions;
    let platform = get_platform_info();
    print_install_instructions(tool, &platform.os);
}
