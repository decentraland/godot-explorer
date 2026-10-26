use godot::{
    builtin::{Callable, Transform2D, VarDictionary, Vector2},
    classes::{FontFile, FontVariation},
    meta::ToGodot,
    obj::{Gd, NewGd},
    prelude::Array,
};

use crate::{
    content::{
        content_mapping::{ContentMappingAndUrlRef, DclContentMappingAndUrl},
        font::faces_from_promise,
    },
    dcl::components::proto_components::sdk::components::common::Font,
    framework::TestContext,
    godot_classes::{dcl_global::DclGlobal, promise::Promise},
};

// Same faux styles as the TextShape glyph map, for styles a font doesn't ship.
const FAUX_BOLD_EMBOLDEN: f32 = 0.6;
const ITALIC_SHEAR_X: f32 = 0.25;

impl Font {
    fn get_font_path(self) -> &'static str {
        match self {
            Font::FSansSerif => {
                "res://assets/themes/fonts/noto/NotoSans-Regular_emoji_fallback.tres"
            }
            Font::FSerif => "res://assets/themes/fonts/noto/NotoSerif-Regular_emoji_fallback.tres",
            Font::FMonospace => {
                "res://assets/themes/fonts/noto/NotoSansMono-Regular_emoji_fallback.tres"
            }
        }
    }

    fn get_font_bold_path(self) -> &'static str {
        match self {
            Font::FSansSerif => "res://assets/themes/fonts/noto/NotoSans-Bold_emoji_fallback.tres",
            Font::FSerif => "res://assets/themes/fonts/noto/NotoSans-Bold_emoji_fallback.tres",
            Font::FMonospace => "res://assets/themes/fonts/noto/NotoSans-Bold_emoji_fallback.tres",
        }
    }

    fn get_font_italic_path(self) -> &'static str {
        match self {
            Font::FSansSerif => {
                "res://assets/themes/fonts/noto/NotoSans-Italic_emoji_fallback.tres"
            }
            Font::FSerif => "res://assets/themes/fonts/noto/NotoSans-Italic_emoji_fallback.tres",
            Font::FMonospace => {
                "res://assets/themes/fonts/noto/NotoSans-Italic_emoji_fallback.tres"
            }
        }
    }

    fn get_font_bold_italic_path(self) -> &'static str {
        match self {
            Font::FSansSerif => {
                "res://assets/themes/fonts/noto/NotoSans-BoldItalic_emoji_fallback.tres"
            }
            Font::FSerif => {
                "res://assets/themes/fonts/noto/NotoSans-BoldItalic_emoji_fallback.tres"
            }
            Font::FMonospace => {
                "res://assets/themes/fonts/noto/NotoSans-BoldItalic_emoji_fallback.tres"
            }
        }
    }

    pub fn try_get_font_resource(&self) -> Option<Gd<godot::classes::Font>> {
        godot::tools::try_load(self.get_font_path()).ok()
    }

    pub fn try_get_font_bold_resource(&self) -> Option<Gd<godot::classes::Font>> {
        godot::tools::try_load(self.get_font_bold_path()).ok()
    }

    pub fn try_get_font_italic_resource(&self) -> Option<Gd<godot::classes::Font>> {
        godot::tools::try_load(self.get_font_italic_path()).ok()
    }

    pub fn try_get_font_bold_italic_resource(&self) -> Option<Gd<godot::classes::Font>> {
        godot::tools::try_load(self.get_font_bold_italic_path()).ok()
    }

    // TODO: Maybe the load can be cached and then return a duplicated one
    pub fn get_font_resource(&self) -> Gd<godot::classes::Font> {
        self.try_get_font_resource().expect("Failed to load font")
    }
    pub fn get_font_bold_resource(&self) -> Gd<godot::classes::Font> {
        self.try_get_font_bold_resource()
            .expect("Failed to load font")
    }
    pub fn get_font_italic_resource(&self) -> Gd<godot::classes::Font> {
        self.try_get_font_italic_resource()
            .expect("Failed to load font")
    }
    pub fn get_font_bold_italic_resource(&self) -> Gd<godot::classes::Font> {
        self.try_get_font_bold_italic_resource()
            .expect("Failed to load font")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontStyle {
    Regular,
    Bold,
    Italic,
    BoldItalic,
}

impl FontStyle {
    /// Key of this style in the faces Dictionary a `fetch_font` promise resolves with.
    pub fn key(self) -> &'static str {
        match self {
            FontStyle::Regular => "regular",
            FontStyle::Bold => "bold",
            FontStyle::Italic => "italic",
            FontStyle::BoldItalic => "bold_italic",
        }
    }

    fn builtin(self, font: Font) -> Gd<godot::classes::Font> {
        match self {
            FontStyle::Regular => font.get_font_resource(),
            FontStyle::Bold => font.get_font_bold_resource(),
            FontStyle::Italic => font.get_font_italic_resource(),
            FontStyle::BoldItalic => font.get_font_bold_italic_resource(),
        }
    }
}

/// The font of a scene UI text node: the `font` enum, or the scene TTF or font family
/// named by `font_src` once it has loaded (the enum font stays as glyph fallback).
pub struct SceneFont {
    font: Font,
    src: Option<String>,
    promise: Option<Gd<Promise>>,
}

impl Default for SceneFont {
    fn default() -> Self {
        Self {
            font: Font::FSansSerif,
            src: None,
            promise: None,
        }
    }
}

impl SceneFont {
    /// Returns true when the font changed and must be re-applied; starts loading a new
    /// `font_src`.
    pub fn update(
        &mut self,
        font: Font,
        font_src: Option<&str>,
        content_mapping: &ContentMappingAndUrlRef,
    ) -> bool {
        let src = font_src
            .map(str::trim)
            .filter(|src| !src.is_empty())
            .map(str::to_owned);
        if font == self.font && src == self.src {
            return false;
        }
        self.font = font;
        self.src = src;
        self.promise = self.src.as_ref().map(|src| {
            DclGlobal::singleton()
                .bind()
                .get_content_provider()
                .bind_mut()
                .fetch_font(
                    src.to_godot(),
                    DclContentMappingAndUrl::from_ref(content_mapping.clone()),
                    false,
                )
        });
        true
    }

    /// Whether the custom font loaded (false while loading, after a rejection or when unset).
    pub fn is_loaded(&self) -> bool {
        self.promise.as_ref().and_then(faces_from_promise).is_some()
    }

    /// Calls `callable` once the custom font finishes loading, if it is still loading.
    pub fn on_loaded(&self, callable: &Callable) {
        let Some(mut promise) = self.promise.clone() else {
            return;
        };
        if !promise.bind().is_resolved() && !promise.is_connected("on_resolved", callable) {
            promise.connect("on_resolved", callable);
        }
    }

    pub fn get(&self, style: FontStyle) -> Gd<godot::classes::Font> {
        let builtin = style.builtin(self.font);
        let Some(faces) = self.promise.as_ref().and_then(faces_from_promise) else {
            return builtin;
        };
        let Some((custom, faux)) = pick_face(&faces, style) else {
            return builtin;
        };
        custom_font_variation(custom, faux, builtin).upcast()
    }
}

/// The closest face the font has for `style`, and the faux style still to apply to it.
fn pick_face(faces: &VarDictionary, style: FontStyle) -> Option<(Gd<FontFile>, FontStyle)> {
    use FontStyle::*;
    let candidates: &[(FontStyle, FontStyle)] = match style {
        Regular => &[(Regular, Regular)],
        Bold => &[(Bold, Regular), (Regular, Bold)],
        Italic => &[(Italic, Regular), (Regular, Italic)],
        BoldItalic => &[
            (BoldItalic, Regular),
            (Bold, Italic),
            (Italic, Bold),
            (Regular, BoldItalic),
        ],
    };
    candidates.iter().find_map(|(face, faux)| {
        let font = faces.get(face.key())?.try_to::<Gd<FontFile>>().ok()?;
        Some((font, *faux))
    })
}

fn custom_font_variation(
    custom: Gd<FontFile>,
    faux: FontStyle,
    fallback: Gd<godot::classes::Font>,
) -> Gd<FontVariation> {
    let mut variation = FontVariation::new_gd();
    variation.set_base_font(&custom);
    let mut fallbacks: Array<Gd<godot::classes::Font>> = Array::new();
    fallbacks.push(&fallback);
    variation.set_fallbacks(&fallbacks);
    if matches!(faux, FontStyle::Bold | FontStyle::BoldItalic) {
        variation.set_variation_embolden(FAUX_BOLD_EMBOLDEN);
    }
    if matches!(faux, FontStyle::Italic | FontStyle::BoldItalic) {
        variation.set_variation_transform(Transform2D::from_cols(
            Vector2::new(1.0, 0.0),
            Vector2::new(ITALIC_SHEAR_X, 1.0),
            Vector2::ZERO,
        ));
    }
    variation
}

#[godot::test::itest]
fn test_font_load(_context: &TestContext) {
    assert_ne!(Font::FSansSerif.try_get_font_resource(), None);
    assert_ne!(Font::FSerif.try_get_font_resource(), None);
    assert_ne!(Font::FMonospace.try_get_font_resource(), None);
}

#[godot::test::itest]
fn test_scene_font_from_bytes(_context: &TestContext) {
    use crate::content::font::{build_font_file, validate_ttf};
    use godot::classes::FileAccess;

    let bytes =
        FileAccess::get_file_as_bytes("res://assets/themes/fonts/noto/NotoSans-Regular.ttf");
    validate_ttf(bytes.as_slice()).expect("bundled NotoSans is a TrueType font");

    let ui_font = build_font_file(&bytes, false).expect("raster font");
    assert!(!ui_font.is_multichannel_signed_distance_field());
    let msdf_font = build_font_file(&bytes, true).expect("MSDF font");
    assert!(msdf_font.is_multichannel_signed_distance_field());

    assert!(build_font_file(&godot::builtin::PackedByteArray::new(), false).is_err());

    let bold_bytes =
        FileAccess::get_file_as_bytes("res://assets/themes/fonts/noto/NotoSans-Bold.ttf");
    let bold_font = build_font_file(&bold_bytes, false).expect("bold font");
    let mut faces = VarDictionary::new();
    faces.set(FontStyle::Regular.key(), ui_font.clone());
    faces.set(FontStyle::Bold.key(), bold_font.clone());
    let (font, faux) = pick_face(&faces, FontStyle::Bold).unwrap();
    assert_eq!((font, faux), (bold_font.clone(), FontStyle::Regular));
    let (font, faux) = pick_face(&faces, FontStyle::Italic).unwrap();
    assert_eq!((font, faux), (ui_font.clone(), FontStyle::Italic));
    let (font, faux) = pick_face(&faces, FontStyle::BoldItalic).unwrap();
    assert_eq!((font, faux), (bold_font, FontStyle::Italic));
    assert!(pick_face(&VarDictionary::new(), FontStyle::Regular).is_none());
    assert_eq!(
        SceneFont::default().get(FontStyle::Regular),
        Font::FSansSerif.get_font_resource()
    );
}
