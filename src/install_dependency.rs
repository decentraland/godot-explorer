use directories::ProjectDirs;
use flate2::read::GzDecoder;
use reqwest::blocking::Client;
use serde_json::Value;
use std::env;
use std::fs::{self, File};
use std::io::{self};
use std::path::{Path, PathBuf};
use tar::Archive;
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

const PROTOCOL_FIXED_VERSION_URL: Option<&str> = Some("https://registry.npmjs.org/@dcl/protocol/-/protocol-1.0.0-21486681149.commit-da1da45.tgz");
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

/// Fetches the latest @next version URL from npm, ignoring any fixed version
fn get_next_protocol_url() -> Result<(String, String), anyhow::Error> {
    let package_name = "@dcl/protocol";

    let client = Client::new();
    let response = client
        .get(format!("https://registry.npmjs.org/{package_name}"))
        .send()?
        .json::<Value>()?;

    let next_version = response["dist-tags"][PROTOCOL_TAG]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Could not find @{} version in npm registry", PROTOCOL_TAG))?;
    let tarball_url = response["versions"][next_version]["dist"]["tarball"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Could not find tarball URL for version {}", next_version))?;

    Ok((next_version.to_string(), tarball_url.to_string()))
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

/// Updates the protocol to the latest version from npm using the configured tag and pins it in the source code
pub fn update_protocol() -> Result<(), anyhow::Error> {
    print_section(&format!("Updating DCL Protocol to @{}", PROTOCOL_TAG));

    // 1. Fetch the latest version from npm
    let spinner = create_spinner(&format!("Fetching latest @{} version from npm...", PROTOCOL_TAG));
    let (version, tarball_url) = get_next_protocol_url()?;
    spinner.finish_and_clear();
    print_message(
        MessageType::Success,
        &format!("Found version: {}", version),
    );

    // 2. Download and extract proto files
    let destination_path = format!("{RUST_LIB_PROJECT_FOLDER}src/dcl/components");
    let spinner = create_spinner("Downloading protocol files...");

    let client = Client::new();
    let response = client.get(&tarball_url).send()?;
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

    // 3. Update the source code to pin to the new version
    let spinner = create_spinner("Updating source code to pin new version...");

    let source_file = Path::new("src/install_dependency.rs");
    let content = fs::read_to_string(source_file)?;

    // Find and replace the PROTOCOL_FIXED_VERSION_URL line
    let new_content = update_protocol_version_in_source(&content, &tarball_url)?;
    fs::write(source_file, new_content)?;

    spinner.finish_with_message("✅ Source code updated");

    print_message(
        MessageType::Success,
        &format!("Protocol updated to version: {}", version),
    );
    print_message(
        MessageType::Info,
        "Run 'cargo run -- build' to rebuild with the new protocol.",
    );
    print_message(
        MessageType::Warning,
        "Note: You may need to fix breaking changes if the protocol API changed.",
    );

    Ok(())
}

/// Updates the PROTOCOL_FIXED_VERSION_URL in the source code
fn update_protocol_version_in_source(content: &str, new_url: &str) -> Result<String, anyhow::Error> {
    use regex::Regex;

    let pattern = Regex::new(r#"const PROTOCOL_FIXED_VERSION_URL: Option<&str> = (?:None|Some\("[^"]*"\));"#)?;

    let replacement = format!(
        r#"const PROTOCOL_FIXED_VERSION_URL: Option<&str> = Some("{}");"#,
        new_url
    );

    if !pattern.is_match(content) {
        return Err(anyhow::anyhow!(
            "Could not find PROTOCOL_FIXED_VERSION_URL in source file"
        ));
    }

    Ok(pattern.replace(content, replacement.as_str()).to_string())
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

pub fn clear_cache_dir() -> io::Result<()> {
    if let Some(dirs) = ProjectDirs::from("org", "decentraland", "devgodot") {
        let cache_dir = dirs.cache_dir();

        if cache_dir.exists() {
            for entry in fs::read_dir(cache_dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    fs::remove_dir_all(&path)?;
                } else {
                    fs::remove_file(&path)?;
                }
            }
        }

        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "The project cache directory could not be determined",
        ))
    }
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
    let android_deps_url = "https://files.dclexplorer.com/android_deps.zip";
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

pub fn install(
    skip_download_templates: bool,
    platforms: &[String],
    no_strip: bool,
) -> Result<(), anyhow::Error> {
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

    if !PathBuf::from(GODOT_SENTRY_ADDON_FOLDER).exists() {
        print_section("Installing Sentry Addon");

        let sentry_addon_folder = PathBuf::from(GODOT_SENTRY_ADDON_FOLDER);
        let uncompressed_folder = PathBuf::from(GODOT_SENTRY_ADDON_FOLDER).join("zip");
        download_and_extract_zip(
            SENTRY_ADDON_URL,
            uncompressed_folder.to_str().unwrap(),
            Some(format!("sentry.zip")),
        )?;

        fs::rename(
            uncompressed_folder.join("addons/sentry/bin"),
            sentry_addon_folder.join("bin"),
        )?;

        fs::remove_dir_all(uncompressed_folder)?;
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
        prepare_templates(platforms, no_strip)?;
    }

    print_message(MessageType::Success, "Installation complete!");

    // Show next steps based on OS
    print_section("Next Steps");

    let next_steps = get_next_steps_instructions();
    println!("{}", next_steps);

    Ok(())
}
