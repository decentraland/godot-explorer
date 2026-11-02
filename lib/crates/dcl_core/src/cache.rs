//! Content-cache policy: which files are tracked, how big they are and which go first when the
//! folder is over its limit. The folder I/O stays with the caller.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use tokio::time::Instant;

struct FileMetadata {
    file_size: i64,
    last_accessed: Instant,
}

/// In-memory index of the cache folder with least-recently-used eviction.
#[derive(Default)]
pub struct CacheIndex {
    files: HashMap<String, FileMetadata>,
}

impl CacheIndex {
    pub fn contains(&self, path: &str) -> bool {
        self.files.contains_key(path)
    }

    pub fn paths(&self) -> Vec<String> {
        self.files.keys().cloned().collect()
    }

    /// Tracks `path`; a re-insert resets its age.
    pub fn insert(&mut self, path: String, file_size: i64) {
        self.files.insert(
            path,
            FileMetadata {
                file_size,
                last_accessed: Instant::now(),
            },
        );
    }

    pub fn touch(&mut self, path: &str) {
        if let Some(metadata) = self.files.get_mut(path) {
            metadata.last_accessed = Instant::now();
        }
    }

    /// Stops tracking `path`; the caller deletes the file.
    pub fn remove(&mut self, path: &str) -> bool {
        self.files.remove(path).is_some()
    }

    pub fn total_size(&self) -> i64 {
        self.files.values().map(|m| m.file_size).sum()
    }

    /// Drops the least recently used entries until `incoming` more bytes fit under `max_size`;
    /// returns the paths to delete from disk.
    pub fn evict_for(&mut self, incoming: i64, max_size: i64) -> Vec<String> {
        let mut evicted = Vec::new();
        while self.total_size() + incoming > max_size {
            let Some(oldest) = self
                .files
                .iter()
                .min_by_key(|(_, metadata)| metadata.last_accessed)
                .map(|(path, _)| path.clone())
            else {
                break;
            };
            self.files.remove(&oldest);
            evicted.push(oldest);
        }
        evicted
    }

    /// Sum of tracked sizes for entries whose file's base name (see `cache_file_base_name`) is
    /// in `base_names`.
    pub fn size_for_base_names(&self, base_names: &HashSet<String>) -> i64 {
        self.files
            .iter()
            .filter_map(|(file_path, metadata)| {
                let file_name = Path::new(file_path).file_name()?.to_str()?;
                base_names
                    .contains(cache_file_base_name(file_name))
                    .then_some(metadata.file_size)
            })
            .sum()
    }
}

/// Base identity of a cache-folder file name — the key that ties every on-disk
/// form of an asset back to its content hash (or url-hash). CID hashes contain
/// no '.', and the hex of "hashed_{hex}" url-texture names never contains 'q':
///   - "{hash}-mobile.zip"         -> "{hash}"        (optimized asset pack)
///   - "{hash}.scn" / "{hash}.ext" -> "{hash}"        (runtime-processed forms)
///   - "{hash}-{16 hex}.opt.scn"   -> "{hash}"        (baked scene GLB, `scene_bake_key`)
///   - "hashed_{hex}_q{N}"         -> "hashed_{hex}"  (url-texture quality variant)
///   - anything else               -> unchanged
pub fn cache_file_base_name(file_name: &str) -> &str {
    if let Some(base) = file_name.strip_suffix("-mobile.zip") {
        return base;
    }
    let base = file_name.split('.').next().unwrap_or(file_name);
    if base.len() > 17 {
        let (head, tail) = base.split_at(base.len() - 17);
        if tail.starts_with('-') && tail[1..].bytes().all(|b| b.is_ascii_hexdigit()) {
            return head;
        }
    }
    if let Some(hex_and_variant) = base.strip_prefix("hashed_") {
        if let Some(pos) = hex_and_variant.rfind("_q") {
            let digits = &hex_and_variant[pos + 2..];
            if pos > 0 && !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) {
                return &base[.."hashed_".len() + pos];
            }
        }
    }
    base
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_file_base_name() {
        // Raw download, processed form, optimized pack — same base.
        assert_eq!(cache_file_base_name("bafkreiabc"), "bafkreiabc");
        assert_eq!(cache_file_base_name("bafkreiabc.scn"), "bafkreiabc");
        assert_eq!(cache_file_base_name("bafkreiabc-mobile.zip"), "bafkreiabc");
        assert_eq!(
            cache_file_base_name("bafkreiabc-0123456789abcdef.opt.scn"),
            "bafkreiabc"
        );
        assert_eq!(cache_file_base_name("bafkreiabc.opt.scn"), "bafkreiabc");
        assert_eq!(
            cache_file_base_name("bafkreiabc-0123456789abcdeg.scn"),
            "bafkreiabc-0123456789abcdeg"
        );
        assert_eq!(cache_file_base_name("Qm123.png"), "Qm123");
        // Url-texture quality variants collapse onto the base url-hash.
        assert_eq!(cache_file_base_name("hashed_a1b2c3"), "hashed_a1b2c3");
        assert_eq!(cache_file_base_name("hashed_a1b2c3_q2"), "hashed_a1b2c3");
        assert_eq!(cache_file_base_name("hashed_a1b2c3_q12"), "hashed_a1b2c3");
        // Not quality suffixes: no digits / empty hex.
        assert_eq!(cache_file_base_name("hashed_a1b2_qx"), "hashed_a1b2_qx");
        assert_eq!(cache_file_base_name("hashed__q2"), "hashed__q2");
    }

    #[test]
    fn size_for_base_names_sums_every_form_of_a_hash() {
        let mut index = CacheIndex::default();
        for (path, size) in [
            ("/cache/bafkhash1", 10),
            ("/cache/bafkhash1.scn", 20),
            ("/cache/bafkhash1-mobile.zip", 40),
            ("/cache/bafkhash2", 100), // not requested
            ("/cache/hashed_abc123", 3),
            ("/cache/hashed_abc123_q2", 7),
            ("/cache/hashed_ffff_q1", 1000), // not requested
        ] {
            index.insert(path.to_string(), size);
        }
        let wanted: HashSet<String> = ["bafkhash1", "hashed_abc123"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(index.size_for_base_names(&wanted), 10 + 20 + 40 + 3 + 7);
        assert_eq!(index.size_for_base_names(&HashSet::new()), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn eviction_drops_least_recently_used_first() {
        let mut index = CacheIndex::default();
        for path in ["/a", "/b", "/c"] {
            index.insert(path.to_string(), 10);
            tokio::time::advance(std::time::Duration::from_millis(10)).await;
        }
        index.touch("/a");
        assert_eq!(index.evict_for(10, 30), vec!["/b".to_string()]);
        assert_eq!(index.total_size(), 20);
        assert!(index.evict_for(0, 100).is_empty());
        assert_eq!(index.evict_for(5, 0).len(), 2);
        assert!(!index.remove("/missing"));
    }
}
