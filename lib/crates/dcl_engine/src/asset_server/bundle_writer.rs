//! Zip writer for the scene bundles (`{entity}-boot.zip`, `{entity}-static.zip`).
//!
//! Uses the `zip` crate rather than Godot's `ZIPPacker`: it writes true Stored
//! entries (the `.scn`/`.res` payloads are zstd already) and needs no Godot thread.

use std::fs::File;
use std::io::{BufWriter, Write};

use zip::write::FileOptions;
use zip::CompressionMethod;

/// One entry of a bundle.
pub enum BundleEntry {
    /// Copy a file from disk under `name`.
    File { name: String, path: String },
    /// Write in-memory bytes under `name`.
    Bytes { name: String, data: Vec<u8> },
}

/// Write `entries` to `zip_path` with `method` and return the archive size.
/// Files are streamed (`io::copy`), never read whole into memory.
pub fn write_bundle(
    zip_path: &str,
    entries: &[BundleEntry],
    method: CompressionMethod,
) -> Result<u64, anyhow::Error> {
    let file = File::create(zip_path).map_err(|e| anyhow::anyhow!("create {}: {}", zip_path, e))?;
    let mut writer = zip::ZipWriter::new(BufWriter::new(file));
    let options = FileOptions::default()
        .compression_method(method)
        .large_file(false);

    for entry in entries {
        match entry {
            BundleEntry::File { name, path } => {
                let mut src =
                    File::open(path).map_err(|e| anyhow::anyhow!("open {}: {}", path, e))?;
                writer.start_file(name, options)?;
                std::io::copy(&mut src, &mut writer)
                    .map_err(|e| anyhow::anyhow!("copy {} into {}: {}", path, zip_path, e))?;
            }
            BundleEntry::Bytes { name, data } => {
                writer.start_file(name, options)?;
                writer.write_all(data)?;
            }
        }
    }

    let mut inner = writer.finish()?;
    inner.flush()?;
    let size = std::fs::metadata(zip_path).map(|m| m.len()).unwrap_or(0);
    Ok(size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn round_trip_stored_and_deflated() {
        let dir = std::env::temp_dir().join(format!("bundle-writer-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let src = dir.join("a.opt.scn");
        std::fs::write(&src, b"RSCC-fake-payload").unwrap();
        for (method, zip_name) in [
            (CompressionMethod::Stored, "static.zip"),
            (CompressionMethod::Deflated, "boot.zip"),
        ] {
            let zip_path = dir.join(zip_name);
            let size = write_bundle(
                zip_path.to_str().unwrap(),
                &[
                    BundleEntry::File {
                        name: "a.opt.scn".into(),
                        path: src.to_str().unwrap().into(),
                    },
                    BundleEntry::Bytes {
                        name: "manifest.json".into(),
                        data: b"{}".to_vec(),
                    },
                ],
                method,
            )
            .unwrap();
            assert!(size > 0);
            let mut archive = zip::ZipArchive::new(File::open(&zip_path).unwrap()).unwrap();
            assert_eq!(archive.len(), 2);
            let mut entry = archive.by_name("a.opt.scn").unwrap();
            assert_eq!(entry.compression(), method);
            let mut data = Vec::new();
            entry.read_to_end(&mut data).unwrap();
            assert_eq!(data, b"RSCC-fake-payload");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
