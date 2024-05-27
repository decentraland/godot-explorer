use directories::ProjectDirs;
use flate2::read::GzDecoder;
use reqwest::blocking::Client;
use serde_json::Value;
use std::env;
use std::fs::{self, File};
use std::io::{self};
use std::path::Path;
use tar::Archive;
use zip::ZipArchive;

use crate::download_file::download_file;
use crate::export::prepare_templates;

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
    let protocol_url = get_protocol_url()?;
    let destination_path = format!("{RUST_LIB_PROJECT_FOLDER}src/dcl/components");

    println!("Downloading {protocol_url:?}");

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
        println!("Getting cached file of {url:?}");
        fs::copy(already_existing_file, "./tmp-file.zip")?;
    } else {
        println!("Downloading {url:?}");
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
        ("linux", "x86_64") => Some("linux.x86_64.zip".to_string()),
        ("windows", "x86_64") => Some("win64.exe.zip".to_string()),
        ("macos", _) => Some("macos.universal.zip".to_string()),
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
        ("linux", "x86_64") => Some("Godot_v4.2.1-stable_linux.x86_64".to_string()),
        ("windows", "x86_64") => Some("Godot_v4.2.1-stable_win64.exe".to_string()),
        ("macos", _) => Some("Godot.app/Contents/MacOS/Godot".to_string()),
        _ => None,
    }?;

    Some(os_url)
}

pub fn install(skip_download_templates: bool) -> Result<(), anyhow::Error> {
    let persistent_path = get_persistent_path(Some("test.zip".into())).unwrap();
    println!("Using persistent path: {persistent_path:?}");

    install_dcl_protocol()?;

    if env::consts::OS == "windows" {
        download_and_extract_zip(
            "https://github.com/GyanD/codexffmpeg/releases/download/6.0/ffmpeg-6.0-full_build-shared.zip",
            format!("{BIN_FOLDER}ffmpeg").as_str(),
            Some("ffmpeg-6.0-full_build-shared.zip".to_string()),
        )?;
    }

    download_and_extract_zip(
        get_protoc_url().unwrap().as_str(),
        format!("{BIN_FOLDER}protoc").as_str(),
        None,
    )?;
    download_and_extract_zip(
        get_godot_url().unwrap().as_str(),
        format!("{BIN_FOLDER}godot").as_str(),
        Some(format!("{GODOT_CURRENT_VERSION}.executable.zip")),
    )?;

    let program_path = format!("{BIN_FOLDER}godot/{}", get_godot_executable_path().unwrap());
    let dest_program_path = format!("{BIN_FOLDER}godot/godot4_bin");

    match (env::consts::OS, env::consts::ARCH) {
        ("linux", _) | ("macos", _) => {
            set_executable_permission(Path::new(
                format!("{BIN_FOLDER}protoc/bin/protoc").as_str(),
            ))?;
            set_executable_permission(Path::new(program_path.as_str()))?;
        }
        _ => (),
    };

    fs::copy(program_path, dest_program_path.as_str())?;

    if !skip_download_templates {
        prepare_templates()?;
    }

    Ok(())
}
