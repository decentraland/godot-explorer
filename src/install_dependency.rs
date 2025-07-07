use directories::ProjectDirs;
use flate2::read::GzDecoder;
use reqwest::blocking::Client;
use serde_json::Value;
use std::env;
use std::fs::{self, File};
use std::io::{self, BufReader};
use std::path::Path;
use tar::Archive;
use xz2::read::XzDecoder;
use zip::ZipArchive;

use crate::consts::*;
use crate::download_file::download_file;
use crate::export::prepare_templates;
use crate::helpers::BinPaths;
use crate::platform::{
    check_command, check_development_dependencies, get_install_command,
    get_next_steps_instructions, get_platform_info,
};
use crate::ui::{create_spinner, print_message, print_section, MessageType};

use crate::consts::{
    BIN_FOLDER, GODOT4_BIN_BASE_URL, GODOT_CURRENT_VERSION, PROTOC_BASE_URL,
    RUST_LIB_PROJECT_FOLDER,
};

fn create_directory_all(path: &Path) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.as_ref().join(entry.file_name());

        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else if metadata.file_type().is_symlink() {
            // Remove existing file/symlink if it exists
            if dst_path.exists() {
                fs::remove_file(&dst_path).ok();
            }

            // Handle symlinks
            #[cfg(unix)]
            {
                let link_target = fs::read_link(&src_path)?;
                use std::os::unix::fs::symlink;
                symlink(&link_target, &dst_path)?;
            }
            #[cfg(not(unix))]
            {
                // On non-Unix, just copy the file
                fs::copy(&src_path, &dst_path)?;
            }
        } else {
            // Regular file - remove existing if present
            if dst_path.exists() {
                fs::remove_file(&dst_path)?;
            }
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

const PROTOCOL_FIXED_VERSION_URL: Option<&str> = None; // Some("https://sdk-team-cdn.decentraland.org/@dcl/protocol/branch//dcl-protocol-1.0.0-9110137086.commit-1d6d5b0.tgz");
const PROTOCOL_TAG: &str = "protocol-squad";

fn get_protocol_url() -> Result<String, anyhow::Error> {
    if let Some(fixed_version_url) = PROTOCOL_FIXED_VERSION_URL {
        return Ok(fixed_version_url.to_string());
    }

    let package_name = "@dcl/protocol";

    let client = Client::new();
    let response = client
        .get(format!("https://registry.npmjs.org/{package_name}"))
        .send()?
        .json::<Value>()?;

    let next_version = response["dist-tags"][PROTOCOL_TAG].as_str().unwrap();
    let tarball_url = response["versions"][next_version]["dist"]["tarball"]
        .as_str()
        .unwrap();

    Ok(tarball_url.to_string())
}

pub fn install_dcl_protocol() -> Result<(), anyhow::Error> {
    print_section("Installing DCL Protocol");

    let protocol_url = get_protocol_url()?;
    let destination_path = format!("{RUST_LIB_PROJECT_FOLDER}src/dcl/components");

    let spinner = create_spinner("Downloading protocol files...");

    let client = Client::new();
    let response = client.get(protocol_url).send()?;
    let tarball = response.bytes()?;

    let decoder = GzDecoder::new(&tarball[..]);
    let mut archive = Archive::new(decoder);

    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;

        // Ignore the rest of the files
        if !path.starts_with("package/proto") {
            continue;
        }

        let dest_path =
            Path::new(destination_path.as_str()).join(path.strip_prefix("package/").unwrap());
        create_directory_all(&dest_path)?;
        let mut file = File::create(dest_path)?;
        io::copy(&mut entry, &mut file)?;
    }

    spinner.finish_with_message("✅ Protocol files installed");

    Ok(())
}

fn get_protoc_url() -> Option<String> {
    let os = env::consts::OS;
    let arch = env::consts::ARCH;

    let os_url = match (os, arch) {
        ("linux", "x86_64") => Some("linux-x86_64.zip".to_string()),
        ("linux", "aarch64") => Some("linux-aarch_64.zip".to_string()),
        ("windows", "x86_64") => Some("win64.zip".to_string()),
        ("macos", _) => Some("osx-universal_binary.zip".to_string()),
        _ => None,
    }?;

    Some(format!("{PROTOC_BASE_URL}{os_url}"))
}

fn get_existing_cached_file(persistent_cache: Option<String>) -> Option<String> {
    let persistent_cache = persistent_cache?;
    let dirs = ProjectDirs::from("org", "decentraland", "devgodot")?;

    fs::create_dir_all(dirs.cache_dir()).ok()?;
    let cache_file_path = dirs.cache_dir().join(persistent_cache);
    if cache_file_path.exists() {
        Some(cache_file_path.to_str().unwrap().to_string())
    } else {
        None
    }
}

fn get_persistent_path(persistent_cache: Option<String>) -> Option<String> {
    let persistent_cache = persistent_cache?;
    let dirs = ProjectDirs::from("org", "decentraland", "devgodot")?;
    fs::create_dir_all(dirs.cache_dir()).ok()?;
    let cache_file_path = dirs.cache_dir().join(persistent_cache);
    Some(cache_file_path.to_str().unwrap().to_string())
}

pub fn godot_export_templates_path() -> Option<String> {
    let os = env::consts::OS;

    match os {
        "windows" => env::var("APPDATA").ok().map(|appdata| {
            format!(
                "{}\\Godot\\export_templates\\{}.stable\\",
                appdata, GODOT_CURRENT_VERSION
            )
        }),
        "linux" => env::var("HOME").ok().map(|home| {
            format!(
                "{}/.local/share/godot/export_templates/{}.stable",
                home, GODOT_CURRENT_VERSION
            )
        }),
        "macos" => env::var("HOME").ok().map(|home| {
            format!(
                "{}/Library/Application Support/Godot/export_templates/{}.stable/",
                home, GODOT_CURRENT_VERSION
            )
        }),
        _ => None, // Unsupported OS
    }
}

pub fn download_and_extract_zip(
    url: &str,
    destination_path: &str,
    persistent_cache: Option<String>,
) -> Result<(), anyhow::Error> {
    if Path::new("./tmp-file.zip").exists() {
        fs::remove_file("./tmp-file.zip")?;
    }

    // If the cached file exist, use it
    if let Some(already_existing_file) = get_existing_cached_file(persistent_cache.clone()) {
        print_message(
            MessageType::Info,
            &format!("Using cached file: {}", already_existing_file),
        );
        fs::copy(already_existing_file, "./tmp-file.zip")?;
    } else {
        print_message(MessageType::Info, &format!("Downloading: {}", url));
        download_file(url, "./tmp-file.zip")?;

        // when the download is done, copy the file to the persistent cache if it applies
        if let Some(persistent_cache) = persistent_cache {
            let persistent_path = get_persistent_path(Some(persistent_cache)).unwrap();
            fs::copy("./tmp-file.zip", persistent_path)?;
        }
    }

    let file = File::open("./tmp-file.zip")?;
    let mut zip_archive = ZipArchive::new(file)?;

    for i in 0..zip_archive.len() {
        let mut file = zip_archive.by_index(i)?;
        let file_path = file.mangled_name();
        let dest_path = Path::new(destination_path).join(file_path);
        create_directory_all(&dest_path)?;
        if file.is_file() {
            let mut extracted_file = File::create(&dest_path)?;
            std::io::copy(&mut file, &mut extracted_file)?;
        }
    }

    fs::remove_file("./tmp-file.zip")?;

    Ok(())
}

fn get_godot_url() -> Option<String> {
    let os = env::consts::OS;
    let arch = env::consts::ARCH;

    let os_url = match (os, arch) {
        ("linux", "x86_64") => Some(
            format!(
                "godot.{}.stable.linux.editor.x86_64.zip",
                GODOT_CURRENT_VERSION
            )
            .to_string(),
        ),
        ("windows", "x86_64") => Some(
            format!(
                "godot.{}.stable.windows.editor.x86_64.exe.zip",
                GODOT_CURRENT_VERSION
            )
            .to_string(),
        ),
        ("macos", _) => Some(
            format!(
                "godot.{}.stable.macos.editor.arm64.zip",
                GODOT_CURRENT_VERSION
            )
            .to_string(),
        ),
        _ => None,
    }?;

    Some(format!("{GODOT4_BIN_BASE_URL}{os_url}"))
}

pub fn set_executable_permission(_file_path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        let mut permissions = fs::metadata(_file_path)?.permissions();
        use std::os::unix::prelude::PermissionsExt;
        permissions.set_mode(0o755);
        fs::set_permissions(_file_path, permissions)?;
        Ok(())
    }
    #[cfg(not(unix))]
    {
        Ok(())
    }
}

pub fn get_godot_executable_path() -> Option<String> {
    let os = env::consts::OS;
    let arch = env::consts::ARCH;

    let os_url = match (os, arch) {
        ("linux", "x86_64") => {
            Some(format!("godot.{}.stable.linux.editor.x86_64", GODOT_CURRENT_VERSION).to_string())
        }
        ("windows", "x86_64") => Some(
            format!(
                "godot.{}.stable.windows.editor.x86_64.exe",
                GODOT_CURRENT_VERSION
            )
            .to_string(),
        ),
        ("macos", _) => Some("Godot.app/Contents/MacOS/Godot".to_string()),
        _ => None,
    }?;

    Some(os_url)
}

fn install_android_tools() -> Result<(), anyhow::Error> {
    print_section("Android Build Tools");

    // Add Android target
    let spinner = create_spinner("Adding Android target to rustup...");
    let add_target_status = std::process::Command::new("rustup")
        .args(["target", "add", "aarch64-linux-android"])
        .status()?;

    if !add_target_status.success() {
        spinner.finish_with_message("❌ Failed to add Android target");
        return Err(anyhow::anyhow!("Failed to add Android target"));
    }
    spinner.finish_with_message("✅ Android target added successfully");

    // Check Android SDK/NDK environment
    print_message(MessageType::Step, "Checking Android SDK/NDK setup...");

    if let Ok(android_ndk_home) = env::var("ANDROID_NDK_HOME") {
        print_message(
            MessageType::Success,
            &format!("ANDROID_NDK_HOME is set: {}", android_ndk_home),
        );
    } else if let Ok(android_ndk) = env::var("ANDROID_NDK") {
        print_message(
            MessageType::Success,
            &format!("ANDROID_NDK is set: {}", android_ndk),
        );
    } else if let Ok(android_sdk) = env::var("ANDROID_SDK") {
        print_message(
            MessageType::Success,
            &format!("ANDROID_SDK is set: {}", android_sdk),
        );
        print_message(
            MessageType::Info,
            &format!(
                "Looking for NDK at: {}/ndk/{}",
                android_sdk, ANDROID_NDK_VERSION
            ),
        );
    } else if let Ok(android_home) = env::var("ANDROID_HOME") {
        print_message(
            MessageType::Success,
            &format!("ANDROID_HOME is set: {}", android_home),
        );
        print_message(
            MessageType::Info,
            &format!(
                "Looking for NDK at: {}/ndk/{}",
                android_home, ANDROID_NDK_VERSION
            ),
        );
    } else {
        print_message(
            MessageType::Warning,
            "No Android SDK/NDK environment variables found",
        );
        print_message(
            MessageType::Info,
            &format!(
                "Will look for NDK at: ~/Android/Sdk/ndk/{}",
                ANDROID_NDK_VERSION
            ),
        );
    }

    print_message(
        MessageType::Success,
        "Android build tools installation complete!",
    );
    print_message(
        MessageType::Info,
        &format!(
            "Note: Make sure you have Android NDK version {} installed",
            ANDROID_NDK_VERSION
        ),
    );

    Ok(())
}

fn download_prebuilt_dependencies() -> Result<(), anyhow::Error> {
    // Ensure BIN_FOLDER exists
    fs::create_dir_all(BIN_FOLDER)?;

    // Download Android dependencies
    let android_deps_url = "https://godot-artifacts.kuruk.net/android_deps.zip";
    let android_deps_path = BinPaths::android_deps_zip();
    let android_deps_extracted_path = BinPaths::android_deps();

    // Check if already extracted
    if !android_deps_extracted_path.exists() {
        if !android_deps_path.exists() {
            print_message(
                MessageType::Info,
                "Android dependencies missing. Downloading...",
            );
            download_file(android_deps_url, android_deps_path.to_str().unwrap())?;
            print_message(MessageType::Success, "Android dependency downloaded");
        }

        // Extract the dependencies
        let spinner = create_spinner("Extracting Android dependencies...");

        fs::create_dir_all(&android_deps_extracted_path)?;

        // Use the zip crate to extract instead of system unzip
        let file = File::open(&android_deps_path)?;
        let mut zip_archive = ZipArchive::new(file)?;

        for i in 0..zip_archive.len() {
            let mut file = zip_archive.by_index(i)?;
            let file_path = file.mangled_name();
            let dest_path = android_deps_extracted_path.join(file_path);

            if file.is_dir() {
                fs::create_dir_all(&dest_path)?;
            } else {
                if let Some(parent) = dest_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                let mut extracted_file = File::create(&dest_path)?;
                std::io::copy(&mut file, &mut extracted_file)?;
            }
        }

        spinner.finish_and_clear();
        print_message(
            MessageType::Success,
            &format!(
                "Android dependencies extracted to {}",
                android_deps_extracted_path.display()
            ),
        );
    } else {
        print_message(
            MessageType::Success,
            "Android dependencies already extracted",
        );
    }

    Ok(())
}

pub fn install(skip_download_templates: bool, platforms: &[String]) -> Result<(), anyhow::Error> {
    print_section("Installing Dependencies");

    let platform = get_platform_info();
    print_message(
        MessageType::Info,
        &format!("Platform: {}", platform.display_name),
    );

    // Check for missing development dependencies
    let dev_deps = check_development_dependencies();
    let missing_deps: Vec<_> = dev_deps
        .iter()
        .filter(|(_, available, _)| !available)
        .collect();

    if !missing_deps.is_empty() {
        print_message(
            MessageType::Warning,
            "Missing development dependencies detected:",
        );
        for (dep, _, desc) in &missing_deps {
            print_message(MessageType::Error, &format!("  {} - {}", dep, desc));
        }

        if let Some(install_cmd) = get_install_command() {
            print_message(MessageType::Info, "\nTo install missing dependencies, run:");
            println!("\n{}\n", install_cmd);

            // Add macOS-specific help
            if platform.os == "macos" {
                print_message(
                    MessageType::Info,
                    "\nNote: If FFmpeg is installed but not detected:",
                );
                println!("  1. Check if PKG_CONFIG_PATH is set correctly");
                println!("  2. For Homebrew ffmpeg@6: export PKG_CONFIG_PATH=\"/opt/homebrew/opt/ffmpeg@6/lib/pkgconfig:$PKG_CONFIG_PATH\"");
                println!("  3. For Intel Macs: export PKG_CONFIG_PATH=\"/usr/local/opt/ffmpeg@6/lib/pkgconfig:$PKG_CONFIG_PATH\"");
            }

            print_message(
                MessageType::Warning,
                "Please install these dependencies before continuing.",
            );
            return Err(anyhow::anyhow!("Missing required development dependencies"));
        }
    }

    let persistent_path = get_persistent_path(Some("test.zip".into())).unwrap();
    print_message(
        MessageType::Info,
        &format!("Cache directory: {}", persistent_path),
    );

    // Check required tools first
    if !check_command("protoc") {
        print_message(
            MessageType::Warning,
            "protoc not found - it will be downloaded",
        );
    }

    install_dcl_protocol()?;

    // Install Android-specific tools and dependencies if Android platform is requested
    if platforms.contains(&"android".to_string()) {
        install_android_tools()?;
        download_prebuilt_dependencies()?;
    }

    // Install FFmpeg 6 for all platforms
    install_ffmpeg()?;

    // Check if protoc is already installed
    if !crate::helpers::is_tool_installed("protoc") {
        print_section("Installing Protocol Buffers Compiler");
        download_and_extract_zip(
            get_protoc_url().unwrap().as_str(),
            BinPaths::protoc().to_str().unwrap(),
            None,
        )?;
        print_message(MessageType::Success, "protoc installed");
    } else {
        print_message(MessageType::Success, "protoc already installed");
    }

    // Check if Godot is already installed
    if !crate::helpers::is_tool_installed("godot") {
        print_section("Installing Godot Engine");
        download_and_extract_zip(
            get_godot_url().unwrap().as_str(),
            BinPaths::godot().to_str().unwrap(),
            Some(format!("{GODOT_CURRENT_VERSION}.executable.zip")),
        )?;

        let program_path = BinPaths::godot().join(get_godot_executable_path().unwrap());
        let dest_program_path = BinPaths::godot_bin();

        match (env::consts::OS, env::consts::ARCH) {
            ("linux", _) | ("macos", _) => {
                set_executable_permission(&program_path)?;
            }
            _ => (),
        };
        fs::copy(program_path, dest_program_path)?;
        print_message(MessageType::Success, "Godot binary installed");
    } else {
        print_message(MessageType::Success, "Godot binary already installed");
    }

    // Set executable permissions for protoc if on Unix-like systems
    match (env::consts::OS, env::consts::ARCH) {
        ("linux", _) | ("macos", _) => {
            let protoc_bin = BinPaths::protoc_bin();
            if protoc_bin.exists() {
                set_executable_permission(&protoc_bin)?;
            }
        }
        _ => (),
    };

    if !skip_download_templates {
        prepare_templates(platforms)?;
    }

    print_message(MessageType::Success, "Installation complete!");

    // Show next steps based on OS
    print_section("Next Steps");

    let next_steps = get_next_steps_instructions();
    println!("{}", next_steps);

    Ok(())
}

pub fn download_and_extract_tar_xz(
    url: &str,
    destination_path: &str,
    persistent_cache: Option<String>,
) -> Result<(), anyhow::Error> {
    if Path::new("./tmp-file.tar.xz").exists() {
        fs::remove_file("./tmp-file.tar.xz")?;
    }

    // If the cached file exists, use it
    if let Some(already_existing_file) = get_existing_cached_file(persistent_cache.clone()) {
        print_message(
            MessageType::Info,
            &format!("Using cached file: {}", already_existing_file),
        );
        fs::copy(already_existing_file, "./tmp-file.tar.xz")?;
    } else {
        print_message(MessageType::Info, &format!("Downloading: {}", url));
        download_file(url, "./tmp-file.tar.xz")?;

        // when the download is done, copy the file to the persistent cache if it applies
        if let Some(persistent_cache) = persistent_cache {
            let persistent_path = get_persistent_path(Some(persistent_cache)).unwrap();
            fs::copy("./tmp-file.tar.xz", persistent_path)?;
        }
    }

    let file = File::open("./tmp-file.tar.xz")?;
    let reader = BufReader::new(file);
    let xz_decoder = XzDecoder::new(reader);
    let mut tar_archive = Archive::new(xz_decoder);

    // Create destination directory if it doesn't exist
    fs::create_dir_all(destination_path)?;

    // Extract the archive preserving symlinks
    tar_archive.set_preserve_permissions(true);
    tar_archive.set_preserve_ownerships(false);
    tar_archive.unpack(destination_path)?;

    fs::remove_file("./tmp-file.tar.xz")?;

    Ok(())
}

pub fn install_ffmpeg() -> Result<(), anyhow::Error> {
    let ffmpeg_folder = BinPaths::ffmpeg();

    // Check if FFmpeg is already installed
    if let Some(ffmpeg_path) = crate::helpers::get_tool_path("ffmpeg") {
        // Check if it's the local FFmpeg by seeing if the path contains .bin
        let path_str = ffmpeg_path.to_string_lossy();
        let is_local = path_str.contains(".bin");

        if is_local {
            // For local FFmpeg, we need to set LD_LIBRARY_PATH
            let mut cmd = std::process::Command::new(&ffmpeg_path);
            cmd.arg("-version");

            // Set LD_LIBRARY_PATH to include the lib directory
            let lib_path = Path::new(".bin/ffmpeg/lib");
            if lib_path.exists() {
                let lib_path_str = lib_path.to_string_lossy();
                if let Ok(existing_ld_path) = env::var("LD_LIBRARY_PATH") {
                    cmd.env(
                        "LD_LIBRARY_PATH",
                        format!("{}:{}", lib_path_str, existing_ld_path),
                    );
                } else {
                    cmd.env("LD_LIBRARY_PATH", lib_path_str.to_string());
                }
            }

            let output = cmd.output();

            if let Ok(output) = output {
                let version_str = String::from_utf8_lossy(&output.stdout);
                if version_str.contains("ffmpeg version n6.") {
                    print_message(MessageType::Success, "FFmpeg 6.x already installed");
                    return Ok(());
                } else {
                    print_message(
                        MessageType::Warning,
                        "FFmpeg found but not version 6.x, reinstalling...",
                    );
                    fs::remove_dir_all(&ffmpeg_folder).ok();
                }
            } else {
                // If we can't run ffmpeg, it might be missing libraries
                print_message(
                    MessageType::Warning,
                    "Failed to check FFmpeg version, reinstalling...",
                );
                fs::remove_dir_all(&ffmpeg_folder).ok();
            }
        }
    }

    print_section("Installing FFmpeg 6.1");

    match env::consts::OS {
        "linux" => {
            // Use BtbN's shared library builds for FFmpeg 6.1
            let url = "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-n6.1-latest-linux64-lgpl-shared-6.1.tar.xz";
            let temp_extract_path = BinPaths::temp_dir("ffmpeg_temp");

            // Clean up any existing temp directory first
            if temp_extract_path.exists() {
                fs::remove_dir_all(&temp_extract_path)?;
            }

            download_and_extract_tar_xz(
                url,
                temp_extract_path.to_str().unwrap(),
                Some("ffmpeg-n6.1-latest-linux64-lgpl-shared-6.1.tar.xz".to_string()),
            )?;

            // The archive extracts to a folder like ffmpeg
            // We need to move its contents to our ffmpeg folder
            let extracted_folder =
                temp_extract_path.join("ffmpeg-n6.1-latest-linux64-lgpl-shared-6.1");

            // Create the final ffmpeg folder
            fs::create_dir_all(&ffmpeg_folder)?;

            // Copy everything from the extracted folder
            // This includes bin/, lib/, include/ directories needed for development
            for entry in fs::read_dir(extracted_folder)? {
                let entry = entry?;
                let file_name = entry.file_name();
                let src = entry.path();
                let dst = ffmpeg_folder.join(file_name);

                if src.is_dir() {
                    // Copy directory recursively using a simple recursive copy
                    copy_dir_all(&src, &dst)?;
                } else {
                    // Remove existing file if present
                    if dst.exists() {
                        fs::remove_file(&dst)?;
                    }
                    fs::copy(&src, &dst)?;
                }
            }

            // Set executable permissions for binaries
            let bin_dir = ffmpeg_folder.join("bin");
            if bin_dir.exists() {
                for entry in fs::read_dir(&bin_dir)? {
                    let entry = entry?;
                    set_executable_permission(&entry.path())?;
                }
            }

            // Clean up temp directory - this time with error handling
            if let Err(e) = fs::remove_dir_all(&temp_extract_path) {
                print_message(
                    MessageType::Warning,
                    &format!("Failed to clean up temp directory: {}", e),
                );
            }

            print_message(
                MessageType::Success,
                "FFmpeg 6.1 shared libraries installed for Linux",
            );
        }
        "windows" => {
            let temp_extract_path = BinPaths::temp_dir("ffmpeg_temp");

            // Clean up any existing temp directory first
            if temp_extract_path.exists() {
                fs::remove_dir_all(&temp_extract_path)?;
            }

            download_and_extract_zip(
                "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-n6.1-latest-win64-lgpl-shared-6.1.zip",
                temp_extract_path.to_str().unwrap(),
                Some("ffmpeg-n6.1-latest-win64-lgpl-shared-6.1.zip".to_string()),
            )?;

            // The archive extracts to a folder like ffmpeg-n6.1-latest-win64-lgpl-shared-6.1
            let extracted_folder =
                temp_extract_path.join("ffmpeg-n6.1-latest-win64-lgpl-shared-6.1");

            // Create the final ffmpeg folder
            fs::create_dir_all(&ffmpeg_folder)?;

            // Copy everything from the extracted folder
            for entry in fs::read_dir(extracted_folder)? {
                let entry = entry?;
                let file_name = entry.file_name();
                let src = entry.path();
                let dst = ffmpeg_folder.join(file_name);

                if src.is_dir() {
                    copy_dir_all(&src, &dst)?;
                } else {
                    if dst.exists() {
                        fs::remove_file(&dst)?;
                    }
                    fs::copy(&src, &dst)?;
                }
            }

            // Clean up temp directory
            if let Err(e) = fs::remove_dir_all(&temp_extract_path) {
                print_message(
                    MessageType::Warning,
                    &format!("Failed to clean up temp directory: {}", e),
                );
            }

            print_message(MessageType::Success, "FFmpeg 6.1 installed for Windows");
        }
        "macos" => {
            // For macOS, we could use evermeet.cx builds or similar
            print_message(
                MessageType::Warning,
                "FFmpeg installation for macOS not yet implemented. Please install via Homebrew: brew install ffmpeg@6",
            );
        }
        _ => {
            print_message(
                MessageType::Warning,
                &format!(
                    "FFmpeg installation not supported for OS: {}",
                    env::consts::OS
                ),
            );
        }
    }

    Ok(())
}
