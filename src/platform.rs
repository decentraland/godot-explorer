use std::env;
use which::which;
use crate::ui::{print_message, MessageType};

/// Platform information
#[derive(Debug, Clone)]
pub struct PlatformInfo {
    pub os: String,
    #[allow(dead_code)]
    pub arch: String,
    pub display_name: String,
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

/// Check if a command exists in PATH
pub fn check_command(cmd: &str) -> bool {
    which(cmd).is_ok()
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
        let ndk_path = format!("{}/ndk/27.1.12297006", sdk);
        if std::path::Path::new(&ndk_path).exists() {
            return Ok(ndk_path);
        }
    }
    
    if let Ok(home) = env::var("ANDROID_HOME") {
        let ndk_path = format!("{}/ndk/27.1.12297006", home);
        if std::path::Path::new(&ndk_path).exists() {
            return Ok(ndk_path);
        }
    }
    
    // Check default location
    if let Ok(home) = env::var("HOME") {
        let ndk_path = format!("{}/Android/Sdk/ndk/27.1.12297006", home);
        if std::path::Path::new(&ndk_path).exists() {
            return Ok(ndk_path);
        }
    }
    
    Err("Android NDK not found. Please install Android NDK 27.1.12297006".to_string())
}

/// Check if iOS development is available (macOS only)
pub fn check_ios_development() -> Result<(), String> {
    let platform = get_platform_info();
    if platform.os != "macos" {
        return Err("iOS development is only available on macOS".to_string());
    }
    
    // Check for Xcode command line tools
    if !check_command("xcrun") {
        return Err("Xcode Command Line Tools not found. Install with: xcode-select --install".to_string());
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
        tools.push(("unzip", "Archive extraction tool"));
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
        "android" => {
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
                print_message(MessageType::Warning, 
                    "Cross-compiling for macOS from non-macOS platform may have limitations");
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

/// Suggest how to install missing dependency
pub fn suggest_install(tool: &str) {
    use crate::ui::print_install_instructions;
    let platform = get_platform_info();
    print_install_instructions(tool, &platform.os);
}