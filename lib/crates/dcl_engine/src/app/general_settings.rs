//! General (non-graphics, non-audio) settings the engine applies.

use crate::fetcher::globals;

const MB: i64 = 1000 * 1000;

/// The content cache cap for the "max cache size" setting: 0 is 1 GB, 1 is 2 GB, else 4 GB.
pub fn cache_size_bytes(setting: i64) -> i64 {
    match setting {
        0 => 1024 * MB,
        1 => 2048 * MB,
        _ => 4096 * MB,
    }
}

pub fn apply_max_cache_size() {
    let setting = globals::engine()
        .bind()
        .setting("max_cache_size".into())
        .try_to::<i64>()
        .unwrap_or(2);
    globals::content_provider()
        .bind_mut()
        .set_cache_folder_max_size(cache_size_bytes(setting));
}

#[cfg(test)]
mod tests {
    use super::cache_size_bytes;

    #[test]
    fn the_cache_cap_follows_the_setting() {
        assert_eq!(cache_size_bytes(0), 1_024_000_000);
        assert_eq!(cache_size_bytes(1), 2_048_000_000);
        assert_eq!(cache_size_bytes(2), 4_096_000_000);
        assert_eq!(cache_size_bytes(-1), 4_096_000_000);
    }
}
