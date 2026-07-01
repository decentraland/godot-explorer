use directories::ProjectDirs;
use flate2::read::GzDecoder;
use reqwest::blocking::Client;
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
    godot_editor_base_url, godot_editor_base_url_for_branch, godot_release_tag,
    sanitize_branch_for_url, BIN_FOLDER, GODOT_BUILD_SHA, GODOT_CURRENT_VERSION, PROTOC_BASE_URL,
    RUST_LIB_PROJECT_FOLDER,
};

fn create_directory_all(path: &Path) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

// Resolve @dcl/protocol from the npm `next` dist-tag (see PROTOCOL_NPM_DIST_TAG).
// Set this to `Some("<tarball-url>")` only to temporarily pin a specific build
// (e.g. a per-PR protocol tarball); leave it `None` to track @next.
// Pinned here to the controls-customization protocol build (PR #426 rebased on main) that
// ships the new mobile_input_controls/ui_input_binding components alongside current main.
const PROTOCOL_FIXED_VERSION_URL: Option<&str> = Some("https://sdk-team-cdn.decentraland.org/@dcl/protocol/branch//dcl-protocol-1.0.0-28452214137.commit-9a82e23.tgz");
const PROTOCOL_NPM_DIST_TAG: &str = "next";

fn get_protocol_url() -> Result<String, anyhow::Error> {
    if let Some(url) = PROTOCOL_FIXED_VERSION_URL {
        return Ok(url.to_string());
    }

    let manifest_url = format!("https://registry.npmjs.org/@dcl/protocol/{PROTOCOL_NPM_DIST_TAG}");
    let manifest: serde_json::Value = Client::new().get(&manifest_url).send()?.json()?;
    manifest
        .get("dist")
        .and_then(|d| d.get("tarball"))
        .and_then(|t| t.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Could not resolve tarball URL for @dcl/protocol@{PROTOCOL_NPM_DIST_TAG} from npm registry"
            )
        })
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

fn get_godot_url(branch: Option<&str>) -> Option<String> {
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

    let base_url = match branch {
        Some(b) => godot_editor_base_url_for_branch(b),
        None => godot_editor_base_url(),
    };

    Some(format!("{base_url}{os_url}"))
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

/// Status of the locally-installed Godot editor binary relative to the pinned
/// `GODOT_CURRENT_VERSION` + `GODOT_BUILD_SHA`.
#[derive(Debug)]
pub enum GodotBinaryStatus {
    /// Installed and matches the expected version + fork SHA.
    Ok,
    /// Installed but reports a different version/SHA (`found` = raw `--version` line).
    Mismatch { found: String },
    /// No binary at `BinPaths::godot_bin()`.
    Missing,
    /// Present but its version couldn't be determined (couldn't exec, or unrecognized output).
    Unverifiable(String),
}

/// Parse a Godot `--version` line of the form `"{version}.stable.gh.{sha} - {label}"` into
/// `(version, sha)`. Returns `None` for any line lacking the `.stable.gh.` marker (e.g. branch or
/// custom builds whose SHA we don't pin). OS-independent — the format comes from Godot's
/// `VERSION_FULL_NAME` and is identical across platforms.
pub fn parse_godot_version(line: &str) -> Option<(String, String)> {
    let token = line.split_whitespace().next()?; // first token; drops the " - <label>" suffix
    let (version, rest) = token.split_once(".stable.gh.")?;
    let sha = rest.split('.').next()?; // tolerate any trailing segments
    Some((version.to_string(), sha.to_string()))
}

/// Run the installed `godot4_bin --version` and compare against `GODOT_CURRENT_VERSION` +
/// `GODOT_BUILD_SHA`. Always validates the HOST editor binary (never the cross-compiled export
/// templates — those are covered by the per-platform SHA marker in `prepare_templates`).
pub fn validate_installed_godot_binary() -> GodotBinaryStatus {
    let bin = BinPaths::godot_bin(); // .bin/godot/godot4_bin — same file is_tool_installed checks
    if !bin.exists() {
        return GodotBinaryStatus::Missing;
    }
    let output = match std::process::Command::new(&bin).arg("--version").output() {
        Ok(o) => o,
        Err(e) => return GodotBinaryStatus::Unverifiable(e.to_string()),
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.lines().find(|l| l.contains(".stable")).unwrap_or("");
    match parse_godot_version(line) {
        Some((v, sha)) if v == GODOT_CURRENT_VERSION && sha == GODOT_BUILD_SHA => {
            GodotBinaryStatus::Ok
        }
        Some(_) => GodotBinaryStatus::Mismatch {
            found: line.trim().to_string(),
        },
        None => {
            GodotBinaryStatus::Unverifiable(format!("unrecognized --version output: {stdout:?}"))
        }
    }
}

/// Warn (non-fatal) if the installed Godot binary doesn't match the pinned version+SHA. Used as a
/// pre-`run` / pre-`export` guard. Intentionally a warning, never an error: `run`/`export` carry no
/// `--branch` context, so a developer on a branch build would always "mismatch" the const SHA.
pub fn warn_if_godot_mismatch() {
    if let GodotBinaryStatus::Mismatch { found } = validate_installed_godot_binary() {
        print_message(
            MessageType::Warning,
            &format!(
                "Installed Godot is {found}, expected {} — run `cargo run -- install` to refresh",
                godot_release_tag()
            ),
        );
    }
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
    use_cache: bool,
    strip_ios: bool,
    branch: Option<&str>,
) -> Result<(), anyhow::Error> {
    print_section("Installing Dependencies");

    let platform = get_platform_info();
    print_message(
        MessageType::Info,
        &format!("Platform: {}", platform.display_name),
    );

    if let Some(b) = branch {
        print_message(
            MessageType::Info,
            &format!(
                "Using Godot branch build: '{}' (editors + templates will be fetched from /branches/{}/)",
                b, b
            ),
        );
    }

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

    if use_cache {
        let persistent_path = get_persistent_path(Some("test.zip".into())).unwrap();
        print_message(
            MessageType::Info,
            &format!("Cache directory: {}", persistent_path),
        );
    } else {
        print_message(MessageType::Info, "Cache disabled (use --cache to enable)");
    }

    // Helper: only pass cache key when --cache is enabled
    let cache_key = |key: String| -> Option<String> {
        if use_cache {
            Some(key)
        } else {
            None
        }
    };

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

    // (Re)install Godot if missing OR if the present binary doesn't match the pinned version+SHA.
    // A stale binary (same version, different fork SHA) is replaced; the SHA-tagged cache key below
    // guarantees a cache miss so the fresh fork build is fetched, not the stale cached zip. Branch
    // builds use existence-only (their fork SHA isn't tracked at const time).
    let needs_godot = match (branch, validate_installed_godot_binary()) {
        (Some(_), GodotBinaryStatus::Missing) => true,
        (Some(_), _) => false,
        (None, GodotBinaryStatus::Ok) => false,
        (None, GodotBinaryStatus::Missing) => true,
        (None, GodotBinaryStatus::Mismatch { found }) => {
            print_message(
                MessageType::Warning,
                &format!(
                    "Godot binary is {found}, expected {} — re-downloading",
                    godot_release_tag()
                ),
            );
            true
        }
        (None, GodotBinaryStatus::Unverifiable(e)) => {
            // Present but can't self-report (e.g. can't exec the host binary). Don't loop-redownload
            // the same artifact — warn and keep it.
            print_message(
                MessageType::Warning,
                &format!("Could not verify Godot binary ({e}); leaving as-is"),
            );
            false
        }
    };
    if needs_godot {
        print_section("Installing Godot Engine");
        let godot_cache_key = match branch {
            Some(b) => format!(
                "{GODOT_CURRENT_VERSION}.branch-{}.executable.zip",
                sanitize_branch_for_url(b)
            ),
            None => format!("{GODOT_CURRENT_VERSION}-{GODOT_BUILD_SHA}.executable.zip"),
        };
        download_and_extract_zip(
            get_godot_url(branch).unwrap().as_str(),
            BinPaths::godot().to_str().unwrap(),
            cache_key(godot_cache_key),
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
        print_message(
            MessageType::Success,
            "Godot binary already installed (version verified)",
        );
    }

    if !PathBuf::from(GODOT_SENTRY_ADDON_FOLDER).exists() {
        print_section("Installing Sentry Addon");

        let sentry_addon_folder = PathBuf::from(GODOT_SENTRY_ADDON_FOLDER);
        let uncompressed_folder = PathBuf::from(GODOT_SENTRY_ADDON_FOLDER).join("zip");
        download_and_extract_zip(
            SENTRY_ADDON_URL,
            uncompressed_folder.to_str().unwrap(),
            cache_key("sentry-1.6.0.zip".into()),
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
        prepare_templates(platforms, use_cache, strip_ios, branch)?;
    }

    print_message(MessageType::Success, "Installation complete!");

    // Show next steps based on OS
    print_section("Next Steps");

    let next_steps = get_next_steps_instructions();
    println!("{}", next_steps);

    Ok(())
}

#[cfg(test)]
mod godot_version_tests {
    use super::parse_godot_version;

    #[test]
    fn parses_stable_gh_build() {
        assert_eq!(
            parse_godot_version("4.6.2.stable.gh.9ee6af7ab - Protocol Squad"),
            Some(("4.6.2".to_string(), "9ee6af7ab".to_string()))
        );
    }

    #[test]
    fn tolerates_trailing_newline_and_crlf() {
        assert_eq!(
            parse_godot_version("4.6.2.stable.gh.9ee6af7ab - Protocol Squad\n"),
            Some(("4.6.2".to_string(), "9ee6af7ab".to_string()))
        );
        assert_eq!(
            parse_godot_version("4.6.2.stable.gh.9ee6af7ab - Protocol Squad\r\n"),
            Some(("4.6.2".to_string(), "9ee6af7ab".to_string()))
        );
    }

    #[test]
    fn rejects_non_gh_builds() {
        // Branch / custom builds without the `.stable.gh.` marker are not pinned.
        assert_eq!(parse_godot_version("4.6.2.stable.custom_build"), None);
        assert_eq!(parse_godot_version("4.6.2.stable"), None);
        assert_eq!(parse_godot_version(""), None);
    }
}
