//! Bundled glyph fallbacks for the TextShape faces: colour emoji (Android 15+ only ships COLRv1
//! emoji, which Godot cannot draw) and Chinese/Japanese as MSDF like the faces, so outlines draw
//! as a ring. The CJK copy shares the chat font's data.

use std::cell::RefCell;

use godot::classes::{Font, FontFile, FontVariation, TextServerManager};
use godot::prelude::*;

use super::glyph_cache;
use crate::content::font::{MSDF_PIXEL_RANGE, MSDF_SIZE};

const FONT_EMOJI: &str = "uid://ivxdopbqg6xx";
const FONT_CJK: &str = "uid://dfx6banyqu12e";
const CJK_WEIGHT: i64 = 400;
const CJK_WEIGHT_BOLD: i64 = 700;

#[derive(Default)]
struct Fallbacks {
    regular: Array<Gd<Font>>,
    bold: Array<Gd<Font>>,
}

thread_local! {
    static FALLBACKS: RefCell<Option<Fallbacks>> = const { RefCell::new(None) };
}

/// `font` with the bundled fallbacks attached once, and its rendered glyphs tracked for the
/// disk cache.
pub fn with_glyph_fallbacks(mut font: Gd<Font>, bold: bool) -> Gd<Font> {
    if let Ok(file) = font.clone().try_cast::<FontFile>() {
        let name = file.get_path().to_string();
        glyph_cache::track(&file, name.rsplit('/').next().unwrap_or(&name));
    }
    if font.get_fallbacks().is_empty() {
        FALLBACKS.with(|cell| {
            let mut cell = cell.borrow_mut();
            let fallbacks = cell.get_or_insert_with(build);
            font.set_fallbacks(if bold {
                &fallbacks.bold
            } else {
                &fallbacks.regular
            });
        });
    }
    font
}

fn build() -> Fallbacks {
    let mut emoji = FontVariation::new_gd();
    if let Ok(base) = try_load::<Font>(FONT_EMOJI) {
        emoji.set_base_font(&base);
    }
    emoji.set_baseline_offset(-0.1);

    let mut fallbacks = Fallbacks::default();
    fallbacks.regular.push(&emoji.clone().upcast::<Font>());
    fallbacks.bold.push(&emoji.upcast::<Font>());

    let Some(mut cjk) = try_load::<FontFile>(FONT_CJK)
        .ok()
        .map(|font| font.duplicate_resource())
        .and_then(|font| font.try_cast::<FontFile>().ok())
    else {
        return fallbacks;
    };
    cjk.set_multichannel_signed_distance_field(true);
    cjk.set_msdf_pixel_range(MSDF_PIXEL_RANGE);
    cjk.set_msdf_size(MSDF_SIZE);
    glyph_cache::track(&cjk, "NotoSansSC-msdf");
    fallbacks
        .regular
        .push(&cjk_weight(&cjk, CJK_WEIGHT).upcast::<Font>());
    fallbacks
        .bold
        .push(&cjk_weight(&cjk, CJK_WEIGHT_BOLD).upcast::<Font>());
    fallbacks
}

/// The variable CJK face defaults to its thinnest weight.
fn cjk_weight(cjk: &Gd<FontFile>, weight: i64) -> Gd<FontVariation> {
    let mut variation = FontVariation::new_gd();
    variation.set_base_font(cjk);
    if let Some(server) = TextServerManager::singleton().get_primary_interface() {
        let mut axes = VarDictionary::new();
        axes.set(server.name_to_tag("wght"), weight);
        variation.set_variation_opentype(&axes);
    }
    variation
}
