use flate2::read::GzDecoder;
use reqwest::blocking::Client;
use serde_json::Value;
use std::env;
use std::fs::{self, File};
use std::io::{self};
use std::path::Path;
use tar::Archive;
use zip::ZipArchive;

const PROTOC_BASE_URL: &str =
    "https://github.com/protocolbuffers/protobuf/releases/download/v23.2/protoc-23.2-";

const GODOT4_BIN_BASE_URL: &str =
    "https://github.com/godotengine/godot/releases/download/4.0.3-stable/Godot_v4.0.3-stable_";

// pub const GODOT4_EXPORT_TEMPLATES_BASE_URL: &str =
//     "https://downloads.tuxfamily.org/godotengine/4.0.3/Godot_v4.0.3-stable_export_templates.tpz";

fn create_directory_all(path: &Path) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn get_protocol_url() -> Result<String, anyhow::Error> {
    let package_name = "@dcl/protocol";

    let client = Client::new();
    let response = client
        .get(format!("https://registry.npmjs.org/{package_name}"))
        .send()?
        .json::<Value>()?;

    let next_version = response["dist-tags"]["next"].as_str().unwrap();
    let tarball_url = response["versions"][next_version]["dist"]["tarball"]
        .as_str()
        .unwrap();

    Ok(tarball_url.to_string())
}

pub fn install_dcl_protocol() -> Result<(), anyhow::Error> {
    let protocol_url = get_protocol_url()?;
    let destination_path = "./decentraland-godot-lib/src/dcl/components";

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

        let dest_path = Path::new(destination_path).join(path.strip_prefix("package/").unwrap());
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

pub fn download_and_extract_zip(url: &str, destination_path: &str) -> Result<(), anyhow::Error> {
    println!("Downloading {url:?}");
    let response = reqwest::blocking::get(url)?;
    let zip_bytes = response.bytes()?;

    let mut zip_archive = ZipArchive::new(std::io::Cursor::new(zip_bytes))?;

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

fn set_executable_permission(_file_path: &Path) -> std::io::Result<()> {
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
        ("linux", "x86_64") => Some("Godot_v4.0.3-stable_linux.x86_64".to_string()),
        ("windows", "x86_64") => Some("Godot_v4.0.3-stable_win64.exe".to_string()),
        ("macos", _) => Some("Godot.app/Contents/MacOS/Godot".to_string()),
        _ => None,
    }?;

    Some(os_url)
}

pub fn get_godot_editor_path() -> Option<String> {
    let os = env::consts::OS;
    let arch = env::consts::ARCH;

    let os_url = match (os, arch) {
        ("linux", "x86_64") => Some("Godot_v4.0.3-stable_linux.x86_64".to_string()),
        ("windows", "x86_64") => Some("Godot_v4.0.3-stable_win64.exe".to_string()),
        ("macos", _) => Some("Godot.app".to_string()),
        _ => None,
    }?;

    Some(os_url)
}

pub fn copy_library(debug_mode: bool) -> Result<(), anyhow::Error> {
    let os = env::consts::OS;
    let arch = env::consts::ARCH;
    let file_name = match (os, arch) {
        ("linux", _) => Some("libdecentraland_godot_lib.so".to_string()),
        ("windows", _) => Some("decentraland_godot_lib.dll".to_string()),
        ("macos", _) => Some("libdecentraland_godot_lib.dylib".to_string()),
        _ => None,
    }
    .expect("Couldn't find a library for this platform");

    let source_folder: &str = if debug_mode {
        "target/debug/"
    } else {
        "target/release/"
    };

    let source_file = fs::canonicalize(source_folder)?.join(file_name.clone());
    let destination_file = fs::canonicalize("./../godot/lib")?.join(file_name);
    fs::copy(source_file, destination_file)?;

    Ok(())
}

pub fn install() -> Result<(), anyhow::Error> {
    install_dcl_protocol()?;

    download_and_extract_zip(get_protoc_url().unwrap().as_str(), "./../.bin/protoc")?;
    download_and_extract_zip(get_godot_url().unwrap().as_str(), "./../.bin/godot")?;

    match (env::consts::OS, env::consts::ARCH) {
        ("linux", _) | ("macos", _) => {
            set_executable_permission(Path::new("./../.bin/protoc/bin/protoc"))?;
            set_executable_permission(Path::new(
                format!("./../.bin/godot/{}", get_godot_executable_path().unwrap()).as_str(),
            ))?;
        }
        _ => (),
    };

    Ok(())
}
