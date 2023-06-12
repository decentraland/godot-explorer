use flate2::read::GzDecoder;
use reqwest::blocking::Client;
use serde_json::Value;
use std::fs::{self, File};
use std::io::{self};
use std::path::Path;
use tar::Archive;

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

pub fn install_dependency() -> Result<(), Box<dyn std::error::Error>> {
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

        if !path.starts_with("package/proto") {
            continue;
        }

        let dest_path = Path::new(destination_path).join(path.strip_prefix("package/").unwrap());
        println!("Extracting {:?}", dest_path);

        create_directory_all(&dest_path)?;
        let mut file = File::create(dest_path)?;
        io::copy(&mut entry, &mut file)?;
    }

    Ok(())
}
