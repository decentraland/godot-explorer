use std::{
    collections::HashSet,
    sync::{Mutex, OnceLock},
};

use godot::{
    builtin::{PackedByteArray, Variant},
    classes::FontFile,
    meta::ToGodot,
    obj::{Gd, NewGd},
};

use crate::godot_classes::promise::Promise;

use super::{
    cache_file_name::cache_file_path, content_provider::ContentProviderContext,
    packed_array::PackedByteArrayFromVec, thread_safety::GodotSingleThreadSafety,
};

/// Same cap as Unity's runtime TTF loader.
pub const MAX_FONT_BYTES: usize = 16 * 1024 * 1024;

const TRUETYPE_SFNT_VERSION: [u8; 4] = [0x00, 0x01, 0x00, 0x00];

// Match the .import settings of the bundled TextShape fonts.
const MSDF_PIXEL_RANGE: i32 = 32;
const MSDF_SIZE: i32 = 48;

/// Trimmed `font_src` if it can name a scene content file. Scene TTF files are the only
/// supported source: URLs and family names fall back to the built-in font.
pub fn scene_font_path(font_src: &str) -> Result<&str, &'static str> {
    let src = font_src.trim();
    if src.is_empty() {
        return Err("empty");
    }
    if src.contains("://") || src.starts_with("//") {
        return Err("external URLs are not supported");
    }
    Ok(src)
}

/// Every component using a `font_src` requests it, so each failing source warns once.
pub fn warn_font_fallback(font_src: &str, reason: &str) {
    static WARNED: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let mut warned = WARNED.get_or_init(Default::default).lock().unwrap();
    if warned.insert(format!("{font_src}\n{reason}")) {
        tracing::warn!("font_src '{font_src}': {reason}; using the built-in font");
    }
}

pub fn font_cache_key(file_hash: &str, msdf: bool) -> String {
    if msdf {
        format!("font_msdf_{file_hash}")
    } else {
        format!("font_{file_hash}")
    }
}

/// Checked before the bytes reach FreeType: TrueType outlines only (no OTF, WOFF or
/// collections), under the size cap.
pub fn validate_ttf(bytes: &[u8]) -> Result<(), anyhow::Error> {
    if bytes.len() > MAX_FONT_BYTES {
        anyhow::bail!(
            "file is {} bytes, the limit is {MAX_FONT_BYTES}",
            bytes.len()
        );
    }
    if bytes.len() < 12 || bytes[..4] != TRUETYPE_SFNT_VERSION {
        anyhow::bail!("not a TrueType (.ttf) font");
    }
    Ok(())
}

pub async fn load_font(
    url: String,
    file_hash: String,
    msdf: bool,
    ctx: ContentProviderContext,
) -> Result<Option<Variant>, anyhow::Error> {
    let absolute_file_path = cache_file_path(&ctx.content_folder, &file_hash);
    let bytes = ctx
        .resource_provider
        .fetch_resource_with_data(&url, &file_hash, &absolute_file_path)
        .await
        .map_err(anyhow::Error::msg)?;

    validate_ttf(&bytes)?;

    let _thread_safe_check =
        GodotSingleThreadSafety::acquire_owned(&ctx)
            .await
            .ok_or(anyhow::Error::msg(
                "Failed while trying to acquire the Godot thread",
            ))?;

    let font = build_font_file(&PackedByteArray::from_vec(&bytes), msdf)?;
    Ok(Some(font.to_variant()))
}

/// The font a `fetch_font` promise resolved with, or None while loading or after a rejection.
pub fn font_from_promise(promise: &Gd<Promise>) -> Option<Gd<FontFile>> {
    promise.bind().get_data().try_to::<Gd<FontFile>>().ok()
}

pub fn build_font_file(bytes: &PackedByteArray, msdf: bool) -> Result<Gd<FontFile>, anyhow::Error> {
    let mut font = FontFile::new_gd();
    if msdf {
        font.set_multichannel_signed_distance_field(true);
        font.set_msdf_pixel_range(MSDF_PIXEL_RANGE);
        font.set_msdf_size(MSDF_SIZE);
    }
    font.set_data(bytes);
    if font.get_face_count() == 0 {
        anyhow::bail!("the font file could not be parsed");
    }
    Ok(font)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ttf_header(tag: [u8; 4]) -> Vec<u8> {
        let mut bytes = tag.to_vec();
        bytes.resize(64, 0);
        bytes
    }

    #[test]
    fn accepts_truetype() {
        assert!(validate_ttf(&ttf_header(TRUETYPE_SFNT_VERSION)).is_ok());
    }

    #[test]
    fn rejects_other_formats() {
        assert!(validate_ttf(&ttf_header(*b"OTTO")).is_err());
        assert!(validate_ttf(&ttf_header(*b"wOFF")).is_err());
        assert!(validate_ttf(&ttf_header(*b"wOF2")).is_err());
        assert!(validate_ttf(&ttf_header(*b"ttcf")).is_err());
        assert!(validate_ttf(&[]).is_err());
        assert!(validate_ttf(&TRUETYPE_SFNT_VERSION).is_err());
    }

    #[test]
    fn rejects_oversized() {
        let mut bytes = ttf_header(TRUETYPE_SFNT_VERSION);
        bytes.resize(MAX_FONT_BYTES + 1, 0);
        assert!(validate_ttf(&bytes).is_err());
    }

    #[test]
    fn classifies_font_src() {
        assert_eq!(scene_font_path("  fonts/A.ttf "), Ok("fonts/A.ttf"));
        assert_eq!(scene_font_path("Lora"), Ok("Lora"));
        assert!(scene_font_path("").is_err());
        assert!(scene_font_path("   ").is_err());
        assert!(scene_font_path("https://example.com/a.ttf").is_err());
        assert!(scene_font_path("//example.com/a.ttf").is_err());
    }

    #[test]
    fn cache_keys_differ_per_variant() {
        assert_ne!(font_cache_key("h", true), font_cache_key("h", false));
    }
}
