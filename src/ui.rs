use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

use crate::consts::ANDROID_NDK_VERSION;

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
            println!(
                "  2. SDK Tools → NDK (Side by side) → Select version {}",
                ANDROID_NDK_VERSION
            );
            println!("  3. Click 'Apply' to install");
        }
        ("protoc", _) => {
            println!("  protoc is automatically installed by running:");
            println!("  cargo run -- install");
            println!("  ");
            println!("  It will be available at: .bin/protoc/bin/protoc");
        }
        _ => {
            println!(
                "  Please refer to the official documentation for {} on {}",
                tool, platform
            );
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

/// Format a Duration into a human-readable string
pub fn format_duration(d: Duration) -> String {
    let total_secs = d.as_secs();
    let millis = d.subsec_millis();
    if total_secs >= 60 {
        format!("{}m {:02}s", total_secs / 60, total_secs % 60)
    } else if total_secs > 0 {
        format!("{}.{}s", total_secs, millis / 100)
    } else {
        format!("{}ms", millis)
    }
}

/// A row in the summary table
pub struct SummaryRow {
    pub name: String,
    pub duration: String,
    pub passed: Option<bool>, // None = skip, Some(true) = pass, Some(false) = fail
}

/// Print a summary table of test steps with timing
pub fn print_summary_table(rows: &[SummaryRow], total_duration: Duration) {
    println!();
    print_section("Full Test Summary");
    println!(
        " {:<40} {:<14} {}",
        "Step".bold(),
        "Duration".bold(),
        "Status".bold()
    );
    println!(" {} {} {}", "─".repeat(40), "─".repeat(14), "─".repeat(8));

    let mut any_failed = false;
    for row in rows {
        let status_colored = match row.passed {
            Some(true) => "PASS".green().to_string(),
            Some(false) => {
                any_failed = true;
                "FAIL".red().to_string()
            }
            None => "SKIP".yellow().to_string(),
        };
        println!(" {:<40} {:<14} {}", row.name, row.duration, status_colored);
    }

    println!(" {} {} {}", "─".repeat(40), "─".repeat(14), "─".repeat(8));

    let total_status = if any_failed {
        "FAIL".red().to_string()
    } else {
        "PASS".green().to_string()
    };
    println!(
        " {:<40} {:<14} {}",
        "Total".bold(),
        format_duration(total_duration),
        total_status
    );
    println!();
}
