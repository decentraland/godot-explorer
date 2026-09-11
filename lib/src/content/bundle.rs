//! Scene bundles: `{entity}-boot.zip` (manifest + main.js + main.crdt) and
//! `{entity}-static.zip` (the models `main.crdt` composes), downloaded once and
//! **extracted** into `user://content/`.
//!
//! Extraction, not mounting: v6 `.scn` files reference their textures as
//! `user://content/{hash}.opt.res`, so the bundled files must land exactly
//! where the per-file downloads would put them. Zip entry names are the cache
//! file names, so this is a straight copy; anything already on disk is skipped
//! (content-addressed, identical) and anything another download is producing
//! right now is left to it. Pure Rust (`zip` crate) on tokio blocking threads —
//! no Godot calls.

use std::collections::HashSet;
use std::io::Read;
use std::sync::Arc;

use super::cache_file_name::cache_file_path;
use super::content_provider::ContentProviderContext;
use super::resource_provider::ResourceProvider;

/// Outcome of one extraction.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct BundleReport {
    /// entries written to the cache folder
    pub extracted: usize,
    /// entries already on disk or being produced by a concurrent download
    pub skipped: usize,
    /// entries rejected (bad name, not expected, I/O error)
    pub failed: usize,
}

/// An entry name is a bare cache file name: no directories, no traversal.
pub fn validate_entry_name(name: &str) -> bool {
    !name.is_empty()
        && !name.contains("..")
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'_' || b == b'-')
}

/// Extract `user://content/{zip_name}` into the cache folder and delete the
/// archive. `expected`, when given, is the set of entry names the manifest
/// announced; anything else in the zip is rejected. Every written file is
/// registered with the `ResourceProvider` (LRU accounting) BEFORE its
/// `pending_downloads` reservation is released, so a `fetch_resource` that was
/// waiting on the same key sees it.
pub async fn extract_bundle(
    ctx: &ContentProviderContext,
    zip_name: &str,
    expected: Option<&HashSet<String>>,
) -> Result<BundleReport, String> {
    let zip_path = cache_file_path(&ctx.content_folder, zip_name);
    let content_folder = ctx.content_folder.to_string();
    let provider = ctx.resource_provider.clone();

    // List entries on a blocking thread (the crate is synchronous).
    let names: Vec<String> = {
        let zip_path = zip_path.clone();
        tokio::task::spawn_blocking(move || -> Result<Vec<String>, String> {
            let file =
                std::fs::File::open(&zip_path).map_err(|e| format!("open {}: {}", zip_path, e))?;
            let mut archive =
                zip::ZipArchive::new(file).map_err(|e| format!("read zip {}: {}", zip_path, e))?;
            Ok((0..archive.len())
                .filter_map(|i| archive.by_index(i).ok().map(|f| f.name().to_string()))
                .collect())
        })
        .await
        .map_err(|e| format!("zip listing task failed: {}", e))??
    };

    let mut report = BundleReport::default();
    for name in names {
        if !validate_entry_name(&name) || expected.is_some_and(|set| !set.contains(&name)) {
            tracing::warn!("bundle {}: rejecting entry '{}'", zip_name, name);
            report.failed += 1;
            continue;
        }
        let dest = cache_file_path(&content_folder, &name);
        if provider.file_exists_by_path(&dest).await || std::path::Path::new(&dest).exists() {
            report.skipped += 1;
            continue;
        }
        // Same key a per-file download of this file would use.
        if !provider.begin_local_install(&name).await {
            report.skipped += 1;
            continue;
        }
        let written = extract_one(&zip_path, &name, &dest).await;
        match written {
            Ok(size) => {
                provider.register_local_file(&dest, size as i64).await;
                report.extracted += 1;
            }
            Err(e) => {
                tracing::warn!("bundle {}: entry '{}' failed: {}", zip_name, name, e);
                report.failed += 1;
            }
        }
        provider.end_local_install(&name).await;
    }

    // The archive is not a second copy of the assets: drop it (and its LRU entry).
    delete_zip(&provider, zip_name, &zip_path).await;
    tracing::info!(
        "bundle {}: extracted={} skipped={} failed={}",
        zip_name,
        report.extracted,
        report.skipped,
        report.failed
    );
    Ok(report)
}

/// Stream one entry to `{dest}.tmp` and rename it into place; returns the size.
/// `.tmp` is appended (not `with_extension`) so it cannot collide with the
/// `X.opt.tmp` a concurrent `download_file` of the same hash would write.
async fn extract_one(zip_path: &str, name: &str, dest: &str) -> Result<u64, String> {
    let zip_path = zip_path.to_string();
    let name = name.to_string();
    let dest = dest.to_string();
    tokio::task::spawn_blocking(move || -> Result<u64, String> {
        let file =
            std::fs::File::open(&zip_path).map_err(|e| format!("open {}: {}", zip_path, e))?;
        let mut archive =
            zip::ZipArchive::new(file).map_err(|e| format!("read zip {}: {}", zip_path, e))?;
        let mut entry = archive
            .by_name(&name)
            .map_err(|e| format!("entry {}: {}", name, e))?;
        let tmp = format!("{}.tmp", dest);
        let mut out = std::fs::File::create(&tmp).map_err(|e| format!("create {}: {}", tmp, e))?;
        let size = std::io::copy(&mut entry, &mut out).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            format!("write {}: {}", tmp, e)
        })?;
        drop(out);
        std::fs::rename(&tmp, &dest).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            format!("rename {} -> {}: {}", tmp, dest, e)
        })?;
        Ok(size)
    })
    .await
    .map_err(|e| format!("extraction task failed: {}", e))?
}

async fn delete_zip(provider: &Arc<ResourceProvider>, zip_name: &str, zip_path: &str) {
    if provider.delete_file_by_hash(zip_name).await.is_none() {
        // Not tracked (adopted before `initialize`, or never registered) —
        // remove it from disk anyway.
        let _ = tokio::fs::remove_file(zip_path).await;
    }
}

/// Download `url` as `zip_name` into the cache and extract it. `Ok(None)` when
/// the server has no such bundle (404); other failures are errors.
pub async fn fetch_and_extract_bundle(
    ctx: &ContentProviderContext,
    zip_name: &str,
    url: &str,
    expected: Option<&HashSet<String>>,
) -> Result<Option<BundleReport>, String> {
    let zip_path = cache_file_path(&ctx.content_folder, zip_name);
    if let Err(e) = ctx
        .resource_provider
        .fetch_resource(url.to_string(), zip_name.to_string(), zip_path)
        .await
    {
        if e.contains("404") {
            return Ok(None);
        }
        return Err(e);
    }
    extract_bundle(ctx, zip_name, expected).await.map(Some)
}

/// Read a bundle entry into memory (used by tests and small boot files).
#[allow(dead_code)]
pub fn read_entry(zip_path: &str, name: &str) -> Result<Vec<u8>, String> {
    let file = std::fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    let mut entry = archive.by_name(name).map_err(|e| e.to_string())?;
    let mut data = Vec::new();
    entry.read_to_end(&mut data).map_err(|e| e.to_string())?;
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn test_ctx(dir: &std::path::Path) -> ContentProviderContext {
        let folder = format!("{}/", dir.display());
        ContentProviderContext {
            content_folder: Arc::new(folder.clone()),
            resource_provider: Arc::new(ResourceProvider::new(
                &folder,
                2048 * 1000 * 1000,
                4,
                #[cfg(feature = "use_resource_tracking")]
                Arc::new(super::super::resource_download_tracking::ResourceDownloadTracking::new()),
            )),
            http_queue_requester: Arc::new(
                crate::http_request::http_queue_requester::HttpQueueRequester::new(1, None),
            ),
            godot_single_thread: Arc::new(tokio::sync::Semaphore::new(1)),
            texture_quality: crate::godot_classes::dcl_config::TextureQuality::Medium,
        }
    }

    fn write_zip(path: &std::path::Path, entries: &[(&str, &[u8])]) {
        let file = std::fs::File::create(path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        let options =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);
        for (name, data) in entries {
            writer.start_file(*name, options).unwrap();
            writer.write_all(data).unwrap();
        }
        writer.finish().unwrap();
    }

    #[tokio::test]
    async fn extracts_registers_skips_and_deletes() {
        let dir = std::env::temp_dir().join(format!("bundle-extract-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let ctx = test_ctx(&dir);
        // an already-cached file must be left untouched
        std::fs::write(dir.join("cached.opt.res"), b"old").unwrap();
        write_zip(
            &dir.join("scene-static.zip"),
            &[
                ("a.opt.scn", b"RSCC-a"),
                ("cached.opt.res", b"new"),
                ("../evil", b"x"),
                ("not-expected.opt.res", b"y"),
            ],
        );
        let expected: HashSet<String> = ["a.opt.scn", "cached.opt.res", "../evil"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        let report = extract_bundle(&ctx, "scene-static.zip", Some(&expected))
            .await
            .unwrap();
        assert_eq!(report.extracted, 1);
        assert_eq!(report.skipped, 1);
        assert_eq!(report.failed, 2);
        assert_eq!(std::fs::read(dir.join("a.opt.scn")).unwrap(), b"RSCC-a");
        assert_eq!(std::fs::read(dir.join("cached.opt.res")).unwrap(), b"old");
        assert!(!dir.join("evil").exists());
        assert!(
            !dir.join("scene-static.zip").exists(),
            "archive deleted after extraction"
        );
        assert!(!dir.join("a.opt.scn.tmp").exists());
        // registered: the provider knows the file and its size
        let path = cache_file_path(&ctx.content_folder, "a.opt.scn");
        assert!(ctx.resource_provider.file_exists_by_path(&path).await);
        let provider = ctx.resource_provider.clone();
        let total = tokio::task::spawn_blocking(move || provider.get_cache_total_size())
            .await
            .unwrap();
        // 6 (extracted) + 3 (`cached.opt.res`, adopted by `initialize`); the zip
        // was adopted too and is gone from the accounting with the file.
        assert_eq!(total, 9);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn local_install_reservation_is_skipped_by_extractor() {
        let dir = std::env::temp_dir().join(format!("bundle-reserve-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let ctx = test_ctx(&dir);
        write_zip(&dir.join("b-static.zip"), &[("b.opt.scn", b"RSCC-b")]);
        // Someone else is producing b.opt.scn right now.
        assert!(ctx.resource_provider.begin_local_install("b.opt.scn").await);
        let report = extract_bundle(&ctx, "b-static.zip", None).await.unwrap();
        assert_eq!((report.extracted, report.skipped), (0, 1));
        assert!(!dir.join("b.opt.scn").exists());
        ctx.resource_provider.end_local_install("b.opt.scn").await;
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn entry_names_are_bare_cache_names() {
        assert!(validate_entry_name("bafkreiabc.opt.scn"));
        assert!(validate_entry_name("bafkreiabc"));
        assert!(validate_entry_name("bafkreiabc-optimized.json"));
        assert!(!validate_entry_name(""));
        assert!(!validate_entry_name("../x"));
        assert!(!validate_entry_name("a/b.scn"));
        assert!(!validate_entry_name("a\\b.scn"));
        assert!(!validate_entry_name("x..scn"));
    }
}
