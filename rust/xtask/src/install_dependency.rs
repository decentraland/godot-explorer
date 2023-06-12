use flate2::read::GzDecoder;
use reqwest::blocking::Client;
use serde_json::Value;
use std::env;
use std::fs::{self, File};
use std::io::{self};
use std::os::unix::prelude::PermissionsExt;
use std::path::Path;
use tar::Archive;
use zip::ZipArchive;

fn create_directory_all(path: &Path) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn get_protocol_url() -> Result<String, Box<dyn std::error::Error>> {
    let package_name = "@dcl/protocol";

    let client = Client::new();
    let response = client
        .get(format!("https://registry.npmjs.org/{}", package_name))
        .send()?
        .json::<Value>()?;

    let next_version = response["dist-tags"]["next"].as_str().unwrap();
    let tarball_url = response["versions"][next_version]["dist"]["tarball"]
        .as_str()
        .unwrap();

    Ok(tarball_url.to_string())
}

pub fn install_dcl_protocol() -> Result<(), Box<dyn std::error::Error>> {
    let protocol_url = get_protocol_url()?;
    let destination_path = "./decentraland-godot-lib/src/dcl/components";

    println!("Downloading {:?}", protocol_url);

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
    let base_url =
        "https://github.com/protocolbuffers/protobuf/releases/download/v23.2/protoc-23.2-";

    let os_url = match (os, arch) {
        ("linux", "x86_64") => Some("linux-x86_64.zip".to_string()),
        ("linux", "aarch64") => Some("linux-aarch_64.zip".to_string()),
        ("windows", "x86_64") => Some("win64.zip".to_string()),
        ("macos", _) => Some("osx-universal_binary.zip".to_string()),
        _ => None,
    }?;

    Some(format!("{}{}", base_url, os_url))
}

fn download_and_extract_zip(
    url: &str,
    destination_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Downloading {:?}", url);
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
    let base_url = "https://downloads.tuxfamily.org/godotengine/4.0.3/Godot_v4.0.3-stable_";

    let os_url = match (os, arch) {
        ("linux", "x86_64") => Some("linux.x86_64.zip".to_string()),
        ("windows", "x86_64") => Some("win64.exe.zip".to_string()),
        ("macos", _) => Some("macos.universal.zip".to_string()),
        _ => None,
    }?;

    Some(format!("{}{}", base_url, os_url))
}

fn set_executable_permission(file_path: &Path) -> std::io::Result<()> {
    let permissions = fs::metadata(file_path)?.permissions();
    let mut new_permissions = permissions.clone();
    new_permissions.set_mode(0o755);
    fs::set_permissions(file_path, new_permissions)?;
    Ok(())
}

pub fn install() -> Result<(), Box<dyn std::error::Error>> {
    install_dcl_protocol()?;
    
    download_and_extract_zip(get_protoc_url().unwrap().as_str(), "./../.bin/protoc")?;
    match (env::consts::OS, env::consts::ARCH) {
        ("linux", _) | ("macos", _) => {
            set_executable_permission(Path::new("./../.bin/protoc/bin/protoc"))?;
        }
        _ => (),
    };

    download_and_extract_zip(get_godot_url().unwrap().as_str(), "./../.bin/godot")?;

    Ok(())
}
