use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

/// Message types with consistent icons and colors
pub enum MessageType {
    Info,
    Success,
    Warning,
    Error,
    Step,
}

/// Print a formatted message with icon and color
pub fn print_message(msg_type: MessageType, message: &str) {
    let output = match msg_type {
        MessageType::Info => format!("ℹ️  {}", message).bright_blue(),
        MessageType::Success => format!("✅ {}", message).green(),
        MessageType::Warning => format!("⚠️  {}", message).yellow(),
        MessageType::Error => format!("❌ {}", message).red(),
        MessageType::Step => format!("▶️  {}", message).cyan(),
    };
    println!("{}", output);
}

/// Print a section header
pub fn print_section(title: &str) {
    println!("\n{}", format!("=== {} ===", title).bold().bright_white());
}

/// Create a progress bar for downloads
#[allow(dead_code)]
pub fn create_download_progress(total_size: u64) -> ProgressBar {
    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("#>-"),
    );
    pb
}

/// Create a spinner for operations without known duration
pub fn create_spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
}

/// Print platform-specific installation instructions
pub fn print_install_instructions(tool: &str, platform: &str) {
    print_message(MessageType::Info, &format!("To install {}, try:", tool));
    
    match (tool, platform) {
        ("rustup", "linux") | ("rustup", "macos") => {
            println!("  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh");
        }
        ("rustup", "windows") => {
            println!("  Download and run: https://win.rustup.rs/");
        }
        ("android-sdk", "linux") | ("android-sdk", "macos") => {
            println!("  1. Download Android Studio: https://developer.android.com/studio");
            println!("  2. Install Android SDK via SDK Manager");
            println!("  3. Set ANDROID_HOME or ANDROID_SDK environment variable");
        }
        ("android-sdk", "windows") => {
            println!("  1. Download Android Studio: https://developer.android.com/studio");
            println!("  2. Install Android SDK via SDK Manager");
            println!("  3. Set ANDROID_HOME or ANDROID_SDK in System Environment Variables");
        }
        ("android-ndk", _) => {
            println!("  1. Open Android Studio SDK Manager");
            println!("  2. SDK Tools → NDK (Side by side) → Select version 27.1.12297006");
            println!("  3. Click 'Apply' to install");
        }
        ("protoc", "linux") => {
            println!("  # Ubuntu/Debian:");
            println!("  sudo apt-get install protobuf-compiler");
            println!("  # Fedora:");
            println!("  sudo dnf install protobuf-compiler");
            println!("  # Arch:");
            println!("  sudo pacman -S protobuf");
        }
        ("protoc", "macos") => {
            println!("  brew install protobuf");
        }
        ("protoc", "windows") => {
            println!("  1. Download from: https://github.com/protocolbuffers/protobuf/releases");
            println!("  2. Add to PATH");
        }
        ("ffmpeg", "linux") => {
            println!("  # Ubuntu/Debian:");
            println!("  sudo apt-get install ffmpeg libavcodec-dev libavformat-dev libavutil-dev libswscale-dev");
            println!("  # Fedora:");
            println!("  sudo dnf install ffmpeg ffmpeg-devel");
            println!("  # Arch:");
            println!("  sudo pacman -S ffmpeg");
        }
        ("ffmpeg", "macos") => {
            println!("  brew install ffmpeg");
        }
        ("ffmpeg", "windows") => {
            println!("  The xtask will download FFmpeg automatically for Windows");
        }
        _ => {
            println!("  Please refer to the official documentation for {} on {}", tool, platform);
        }
    }
}

/// Print a divider line
pub fn print_divider() {
    println!("{}", "─".repeat(60).bright_black());
}

/// Print build status
pub fn print_build_status(target: &str, status: &str) {
    let icon = match status {
        "starting" => "🔨",
        "success" => "✅",
        "failed" => "❌",
        _ => "📦",
    };
    println!("{} {} build: {}", icon, target.bold(), status);
}

/// Format bytes to human readable
#[allow(dead_code)]
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];
    if bytes == 0 {
        return "0 B".to_string();
    }
    let i = (bytes as f64).log2() / 10.0;
    let i = i.floor() as usize;
    let size = bytes as f64 / (1024_f64).powi(i as i32);
    format!("{:.2} {}", size, UNITS[i])
}