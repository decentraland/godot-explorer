use std::{
    collections::{HashMap, HashSet},
    sync::{Mutex, OnceLock},
    time::Duration,
};

use futures_util::future::join_all;
use godot::{
    builtin::{PackedByteArray, VarDictionary, Variant},
    classes::FontFile,
    meta::ToGodot,
    obj::{Gd, NewGd},
};
use serde::Deserialize;

use crate::{
    godot_classes::{font::FontStyle, promise::Promise},
    http_request::request_response::{RequestOption, ResponseEnum, ResponseType},
};

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

const FONTSOURCE_API: &str = "https://api.fontsource.org/v1/fonts/";
const FONTSOURCE_CDN: &str = "https://cdn.jsdelivr.net/fontsource/fonts/";

/// Trimmed `font_src` if it can name a scene content file or a font family. External URLs
/// fall back to the built-in font.
pub fn scene_font_path(font_src: &str) -> Result<&str, &'static str> {
    let src = font_src.trim();
    if src.is_empty() {
        return Err("empty");
    }
    if src.len() > MAX_FONT_SRC_BYTES {
        return Err("longer than 256 bytes");
    }
    if src.contains("://") || src.starts_with("//") {
        return Err("external URLs are not supported");
    }
    Ok(src)
}

const MAX_FONT_SRC_BYTES: usize = 256;
/// Font cache keys one scene loads: 8 sources, each as raster (UI) and MSDF (TextShape).
/// A Fontsource load holds up to 4 FontFiles, so a scene keeps at most 64.
const MAX_SCENE_FONT_LOADS: usize = 16;
/// Loads of one cache key per scene, retries after a failure included.
const MAX_FONT_LOAD_ATTEMPTS: u8 = 3;
/// Distinct `font_src` values that warn; later ones log at debug.
const MAX_FONT_WARNINGS: usize = 64;
/// `font_src` prefix kept for warnings and their dedupe.
const FONT_SRC_LOG_CHARS: usize = 64;

/// Counts a load of `cache_key` in a scene's `loads`; false once the scene is over budget.
pub fn admit_font_load(loads: &mut HashMap<String, u8>, cache_key: &str) -> bool {
    if let Some(attempts) = loads.get_mut(cache_key) {
        if *attempts >= MAX_FONT_LOAD_ATTEMPTS {
            return false;
        }
        *attempts += 1;
        return true;
    }
    if loads.len() >= MAX_SCENE_FONT_LOADS {
        return false;
    }
    loads.insert(cache_key.to_owned(), 1);
    true
}

/// Every component using a `font_src` requests it, so each failing source warns once.
pub fn warn_font_fallback(font_src: &str, reason: &str) {
    static WARNED: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let src: String = font_src.chars().take(FONT_SRC_LOG_CHARS).collect();
    let mut warned = WARNED.get_or_init(Default::default).lock().unwrap();
    if warned.contains(&src) {
        return;
    }
    if warned.len() >= MAX_FONT_WARNINGS {
        tracing::debug!("font_src '{src}': {reason}; using the built-in font");
        return;
    }
    tracing::warn!("font_src '{src}': {reason}; using the built-in font");
    warned.insert(src);
}

pub fn font_cache_key(file_hash: &str, msdf: bool) -> String {
    if msdf {
        format!("font_msdf_{file_hash}")
    } else {
        format!("font_{file_hash}")
    }
}

pub fn font_family_cache_key(id: &str, msdf: bool) -> String {
    font_cache_key(&format!("family_{id}"), msdf)
}

/// Fontsource id of a family name, as Bevy derives it: "Playfair Display" ->
/// "playfair-display". None if `src` can't be a family name (e.g. a file path).
pub fn fontsource_id(src: &str) -> Option<String> {
    let id = src
        .split_whitespace()
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>()
        .join("-");
    (!id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')).then_some(id)
}

/// The parts of a Fontsource family record (`api.fontsource.org/v1/fonts/{id}`) we use.
#[derive(Deserialize)]
struct FontsourceFamily {
    subsets: Vec<String>,
    #[serde(rename = "npmVersion")]
    npm_version: String,
    /// weight ("400") -> style ("normal" / "italic") -> subset ("latin") -> files
    variants: HashMap<String, HashMap<String, HashMap<String, FontsourceVariant>>>,
}

#[derive(Deserialize)]
struct FontsourceVariant {
    url: FontsourceUrls,
}

#[derive(Deserialize)]
struct FontsourceUrls {
    ttf: Option<String>,
}

impl FontsourceFamily {
    /// TTF of the latin subset (or the family's first subset), pinned to the record's
    /// package version so the disk cache follows font updates.
    fn ttf(&self, weight: &str, style: &str) -> Option<String> {
        let version_ok = self
            .npm_version
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-');
        if !version_ok {
            return None;
        }
        let subsets = self.variants.get(weight)?.get(style)?;
        let subset = subsets
            .get("latin")
            .or_else(|| self.subsets.iter().find_map(|subset| subsets.get(subset)))?;
        let url = subset
            .url
            .ttf
            .as_ref()?
            .replace("@latest/", &format!("@{}/", self.npm_version));
        url.starts_with(FONTSOURCE_CDN).then_some(url)
    }

    /// The file for each style the family really has, with Bevy's weight preferences. A
    /// style that would reuse another style's file is left out, so it gets faux styling.
    fn faces(&self) -> Vec<(FontStyle, String)> {
        const REGULAR: &[&str] = &["400", "500", "300", "600"];
        const BOLD: &[&str] = &["700", "600", "800", "500", "900"];
        let wanted: [(FontStyle, &[&str], &str); 4] = [
            (FontStyle::Regular, REGULAR, "normal"),
            (FontStyle::Bold, BOLD, "normal"),
            (FontStyle::Italic, REGULAR, "italic"),
            (FontStyle::BoldItalic, BOLD, "italic"),
        ];
        let mut faces: Vec<(FontStyle, String)> = Vec::new();
        for (style, weights, font_style) in wanted {
            let url = weights.iter().find_map(|w| self.ttf(w, font_style));
            if let Some(url) = url.filter(|url| faces.iter().all(|(_, used)| used != url)) {
                faces.push((style, url));
            }
        }
        faces
    }
}

/// Disk cache name of a Fontsource file, from its version-pinned URL.
fn fontsource_cache_name(url: &str) -> String {
    let path = url.strip_prefix(FONTSOURCE_CDN).unwrap_or(url);
    let name: String = path
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    format!("fontsource_{name}")
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

    if let Err(error) = validate_ttf(&bytes) {
        ctx.resource_provider.delete_file(&absolute_file_path).await;
        return Err(error);
    }

    let _thread_safe_check =
        GodotSingleThreadSafety::acquire_owned(&ctx)
            .await
            .ok_or(anyhow::Error::msg(
                "Failed while trying to acquire the Godot thread",
            ))?;

    let font = build_font_file(&PackedByteArray::from_vec(&bytes), msdf)?;
    let mut faces = VarDictionary::new();
    faces.set(FontStyle::Regular.key(), font);
    Ok(Some(faces.to_variant()))
}

/// Resolves a Google Fonts family through Fontsource, like Bevy. Only the latin subset is
/// loaded; other glyphs come from the built-in fallback font.
pub async fn load_font_family(
    id: String,
    msdf: bool,
    ctx: ContentProviderContext,
) -> Result<Option<Variant>, anyhow::Error> {
    let request = RequestOption::new(
        0,
        format!("{FONTSOURCE_API}{id}"),
        http::Method::GET,
        ResponseType::AsJson,
        None,
        None,
        Some(Duration::from_secs(30)),
    );
    let response = ctx
        .http_queue_requester
        .request(request, 0)
        .await
        .map_err(|e| anyhow::anyhow!("font family lookup failed: {}", e.error_message))?;
    if response.status_code == http::StatusCode::NOT_FOUND {
        anyhow::bail!("unknown font family");
    }
    if !response.status_code.is_success() {
        anyhow::bail!("font family lookup failed: {}", response.status_code);
    }
    let Ok(ResponseEnum::Json(Ok(record))) = response.response_data else {
        anyhow::bail!("font family lookup failed: invalid response");
    };
    let family: FontsourceFamily = serde_json::from_value(record)?;

    let downloads = family.faces().into_iter().map(|(style, url)| {
        let resource_provider = ctx.resource_provider.clone();
        let cache_name = fontsource_cache_name(&url);
        let path = cache_file_path(&ctx.content_folder, &cache_name);
        async move {
            let bytes = resource_provider
                .fetch_resource_with_data(&url, &cache_name, &path)
                .await
                .map_err(anyhow::Error::msg);
            let bytes = match bytes {
                Ok(bytes) => match validate_ttf(&bytes) {
                    Ok(()) => Ok(bytes),
                    Err(error) => {
                        resource_provider.delete_file(&path).await;
                        Err(error)
                    }
                },
                Err(error) => Err(error),
            };
            (style, bytes)
        }
    });
    let mut files = Vec::new();
    for (style, bytes) in join_all(downloads).await {
        match bytes {
            Ok(bytes) => files.push((style, bytes)),
            Err(error) if style == FontStyle::Regular => {
                anyhow::bail!("font family download failed: {error}")
            }
            Err(error) => tracing::debug!("font family '{id}' style {}: {error}", style.key()),
        }
    }
    if files.iter().all(|(style, _)| *style != FontStyle::Regular) {
        anyhow::bail!("font family has no regular style");
    }

    let _thread_safe_check =
        GodotSingleThreadSafety::acquire_owned(&ctx)
            .await
            .ok_or(anyhow::Error::msg(
                "Failed while trying to acquire the Godot thread",
            ))?;

    let mut faces = VarDictionary::new();
    for (style, bytes) in files {
        let font = build_font_file(&PackedByteArray::from_vec(&bytes), msdf)?;
        faces.set(style.key(), font);
    }
    Ok(Some(faces.to_variant()))
}

/// The faces (`FontStyle::key` -> FontFile) a `fetch_font` promise resolved with, or None
/// while loading or after a rejection.
pub fn faces_from_promise(promise: &Gd<Promise>) -> Option<VarDictionary> {
    promise.bind().get_data().try_to::<VarDictionary>().ok()
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
        assert!(scene_font_path(&"a".repeat(MAX_FONT_SRC_BYTES)).is_ok());
        assert!(scene_font_path(&"a".repeat(MAX_FONT_SRC_BYTES + 1)).is_err());
    }

    #[test]
    fn cache_keys_differ_per_variant() {
        assert_ne!(font_cache_key("h", true), font_cache_key("h", false));
        assert_ne!(
            font_family_cache_key("h", false),
            font_cache_key("h", false)
        );
    }

    #[test]
    fn derives_fontsource_ids() {
        assert_eq!(fontsource_id("Lora").as_deref(), Some("lora"));
        assert_eq!(
            fontsource_id("  Playfair   Display ").as_deref(),
            Some("playfair-display")
        );
        assert_eq!(fontsource_id("Open_Sans"), None);
        assert_eq!(fontsource_id("fonts/a.ttf"), None);
        assert_eq!(fontsource_id(""), None);
    }

    fn family(json: serde_json::Value) -> FontsourceFamily {
        serde_json::from_value(json).unwrap()
    }

    fn variant(subset: &str, file: &str) -> serde_json::Value {
        let url = format!("{FONTSOURCE_CDN}x@latest/{file}");
        serde_json::json!({ subset: { "url": { "ttf": url } } })
    }

    #[test]
    fn picks_real_faces_only() {
        let faces = family(serde_json::json!({
            "subsets": ["latin"],
            "npmVersion": "5.3.0",
            "variants": {
                "400": { "normal": variant("latin", "400-normal.ttf"),
                         "italic": variant("latin", "400-italic.ttf") },
                "600": { "normal": variant("latin", "600-normal.ttf") },
            }
        }))
        .faces();
        let urls: Vec<(FontStyle, &str)> = faces
            .iter()
            .map(|(style, url)| (*style, url.strip_prefix(FONTSOURCE_CDN).unwrap()))
            .collect();
        assert_eq!(
            urls,
            vec![
                (FontStyle::Regular, "x@5.3.0/400-normal.ttf"),
                (FontStyle::Bold, "x@5.3.0/600-normal.ttf"),
                (FontStyle::Italic, "x@5.3.0/400-italic.ttf"),
            ]
        );
    }

    #[test]
    fn regular_only_family_has_no_bold() {
        // 500 is both the regular and the bold pick; bold must not reuse it.
        let faces = family(serde_json::json!({
            "subsets": ["latin-ext"],
            "npmVersion": "1.0.0",
            "variants": { "500": { "normal": variant("latin-ext", "500.ttf") } }
        }))
        .faces();
        assert_eq!(faces.len(), 1);
        assert_eq!(faces[0].0, FontStyle::Regular);
    }

    #[test]
    fn rejects_files_outside_fontsource() {
        let faces = family(serde_json::json!({
            "subsets": ["latin"],
            "npmVersion": "1.0.0",
            "variants": { "400": { "normal": { "latin": { "url": {
                "ttf": "https://example.com/400.ttf" } } } } }
        }))
        .faces();
        assert!(faces.is_empty());
    }

    #[test]
    fn rejects_unsafe_npm_version() {
        let faces = family(serde_json::json!({
            "subsets": ["latin"],
            "npmVersion": "1.0/../x",
            "variants": { "400": { "normal": variant("latin", "400.ttf") } }
        }))
        .faces();
        assert!(faces.is_empty());
    }

    #[test]
    fn scene_font_budget_limits_attempts_and_loads() {
        let mut loads = HashMap::new();
        for _ in 0..MAX_FONT_LOAD_ATTEMPTS {
            assert!(admit_font_load(&mut loads, "a"));
        }
        assert!(!admit_font_load(&mut loads, "a"));
        for i in 1..MAX_SCENE_FONT_LOADS {
            assert!(admit_font_load(&mut loads, &format!("k{i}")));
        }
        assert!(!admit_font_load(&mut loads, "one_too_many"));
        assert!(admit_font_load(&mut loads, "k1"));
        assert!(admit_font_load(&mut HashMap::new(), "one_too_many"));
    }

    #[test]
    fn cache_name_tracks_version() {
        assert_eq!(
            fontsource_cache_name(&format!("{FONTSOURCE_CDN}lora@5.3.0/latin-400-normal.ttf")),
            "fontsource_lora_5_3_0_latin_400_normal_ttf"
        );
    }
}
