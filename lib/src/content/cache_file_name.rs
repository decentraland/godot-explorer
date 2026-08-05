//! Maps a content hash to the name it gets inside the content cache folder.
//!
//! Catalyst hashes are short CIDs (`bafkrei…`, 59 chars) and can be used verbatim, but the
//! SDK preview server hashes a file as `b64-<base64(absolute_path + "-" + machine_id)>`
//! (see `b64HashingFunction` in @dcl/sdk-commands). That name grows with the developer's
//! project path: a scene living in a deep folder produces 250-300+ char hashes, past the
//! 255-byte per-component filename limit of ext4/f2fs (Android), APFS and NTFS. `File::create`
//! then fails with ENAMETOOLONG ("File name too long") and the asset never loads.
//!
//! Names that don't fit — or that contain a path separator, which base64 can emit — are folded
//! into a short deterministic digest of the hash. The mapping is pure and stable across runs,
//! so cache hits keep working; short hashes are returned untouched, so existing caches and the
//! `-mobile.zip` / `.scn` naming built on top of them are unaffected.

use std::borrow::Cow;

use crate::godot_classes::dcl_hashing::hash_v1;

/// Max length of a single path component on every filesystem we ship on.
const MAX_FILE_NAME_BYTES: usize = 255;

/// Hard cap on the hash part of a cache file name. Well under `MAX_FILE_NAME_BYTES` on
/// purpose: it leaves room for the `wearable_`/`emote_` prefixes and the `.scn` / `.tmp`
/// suffixes callers add, and it keeps the *whole* path short on platforms that bound the
/// full path too (Windows `MAX_PATH` is 260 for the entire string). Everything we hash
/// legitimately is far below it — catalyst CIDs are 59 bytes and url-texture
/// `hashed_{hex}_q{N}` names ~74 — so in practice only oversized preview hashes fold.
const MAX_HASH_BYTES: usize = 128;

/// File name to use inside the content cache folder for `hash`.
///
/// Only the on-disk name is folded — the hash itself stays the identity used for URLs,
/// content mappings and in-flight bookkeeping.
pub fn cache_file_name(hash: &str) -> Cow<'_, str> {
    if hash.len() <= MAX_HASH_BYTES && !hash.contains(['/', '\\']) {
        return Cow::Borrowed(hash);
    }
    Cow::Owned(format!("long-{}", hash_v1(hash.as_bytes())))
}

/// Absolute path of `hash` inside `content_folder` (which already ends with a separator).
pub fn cache_file_path(content_folder: &str, hash: &str) -> String {
    format!("{}{}", content_folder, cache_file_name(hash))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A preview hash for a file inside a deeply nested project. Verbatim it is longer than
    /// any filesystem allows, which is what made every asset of such a scene fail to download.
    fn long_preview_hash() -> String {
        format!("b64-{}", "QUJDREVGR0hJSktMTU5PUFFSU1RVVldYWVo=".repeat(8))
    }

    #[test]
    fn names_within_the_cap_are_used_verbatim() {
        let cid = "bafkreibmrvrdgqthfrvehyell552sk7ivuas2ozzjdmlojbzttqlcrxiya";
        assert_eq!(cache_file_name(cid), cid);
        assert!(matches!(cache_file_name(cid), Cow::Borrowed(_)));

        // The cap is exact: MAX_HASH_BYTES still passes through, one more byte folds.
        let at_cap = "a".repeat(MAX_HASH_BYTES);
        assert_eq!(cache_file_name(&at_cap), at_cap.as_str());
        assert_ne!(cache_file_name(&format!("{}a", at_cap)), at_cap.as_str());
    }

    #[test]
    fn long_hashes_fold_into_a_name_that_fits_with_room_for_prefixes_and_suffixes() {
        let hash = long_preview_hash();
        assert!(hash.len() > MAX_FILE_NAME_BYTES);

        let name = cache_file_name(&hash);
        assert_ne!(name, hash.as_str());
        assert!(format!("wearable_{}.scn", name).len() <= MAX_HASH_BYTES);
    }

    #[test]
    fn folding_is_stable_and_distinguishes_hashes() {
        let hash = long_preview_hash();
        let other = format!("{}x", hash);
        assert_eq!(cache_file_name(&hash), cache_file_name(&hash));
        assert_ne!(cache_file_name(&hash), cache_file_name(&other));
    }

    /// base64 can emit `/`, which would otherwise turn the cache file name into a
    /// non-existent sub-directory.
    #[test]
    fn path_separators_are_folded_away() {
        let name = cache_file_name("b64-abc/def");
        assert!(!name.contains('/'));
    }

    #[test]
    fn folded_names_are_stable_under_a_second_fold() {
        let folded = cache_file_name(&long_preview_hash()).into_owned();
        assert_eq!(cache_file_name(&folded), folded.as_str());
    }
}
