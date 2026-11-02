//! TextShape sizing, alignment and color: Unity TextMeshPro's numbers mapped onto a Godot
//! `Label3D`. Every default and TMP-matching constant lives here.
//!
//! The per-face tables were calibrated on a measurement grid against the Unity reference
//! (issue #2371): the shared conversion rendered glyphs oversized, the tracking too wide, the
//! first line placed higher and the line pitch tighter, each by a different amount per face.

use godot::builtin::{Color, Vector2};
use godot::global::{HorizontalAlignment, VerticalAlignment};

use super::markup::strip_to_plain;

/// The three TextShape faces, matching the Unity reference's Font List, plus a scene font
/// (`font_src`), which cannot be calibrated: it uses the mean of the three faces in every table.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Face {
    Sans,
    Serif,
    Mono,
    Custom,
}

impl Face {
    /// The face of a `Font` enum value (0 sans, 1 serif, 2 monospace).
    pub fn from_index(index: i32) -> Face {
        match index {
            1 => Face::Serif,
            2 => Face::Mono,
            _ => Face::Sans,
        }
    }

    /// The face a font file name belongs to; None for a font that is not one of ours.
    pub fn from_file_name(name: &str) -> Option<Face> {
        match name {
            "Inter-SemiBold.ttf" => Some(Face::Sans),
            "NotoSerif.ttf" => Some(Face::Serif),
            "AtkinsonHyperlegibleMono.ttf" => Some(Face::Mono),
            _ => None,
        }
    }

    /// Regular face, by UID so it survives file moves: Inter SemiBold v3.3 (the build the Unity
    /// reference bakes its SDF atlas from), Noto Serif, Atkinson Hyperlegible Mono. All flattened
    /// MSDF imports.
    pub fn font_uid(self) -> &'static str {
        match self {
            Face::Sans | Face::Custom => "uid://dsp1v3gwqgdvu",
            Face::Serif => "uid://d4f8lc838jeet",
            Face::Mono => "uid://4ldm8tygdnbc",
        }
    }

    /// The real bold face, for `[b]` markup (synthetic embolden mangles MSDF glyphs).
    pub fn bold_font_uid(self) -> &'static str {
        match self {
            Face::Sans | Face::Custom => "uid://ru8tau3y1nqn",
            Face::Serif => "uid://1vmt0d4x5gr1",
            Face::Mono => "uid://bfqjyubth6dkc",
        }
    }

    /// TMP `fontSize` to Godot glyph pixels: the base conversion (22) times the pitch-match
    /// factor (0.887) divided by the measured per-face glyph oversize.
    fn font_size_factor(self) -> f32 {
        match self {
            Face::Sans => 18.242,
            Face::Serif => 18.812,
            Face::Mono => 19.122,
            Face::Custom => 18.725,
        }
    }

    /// Extra per-glyph spacing, per-mille of the em, measured against Unity's tracking.
    fn glyph_spacing_per_mille(self) -> f32 {
        match self {
            Face::Sans => 61.0,
            Face::Serif => 38.0,
            Face::Mono => 28.0,
            Face::Custom => 42.0,
        }
    }

    /// Vertical anchor correction (fraction of the em) between Label3D and TMP: Unity's
    /// top-anchored text hangs a little lower than Godot's, by a constant per face.
    fn anchor_offset_em(self) -> f32 {
        match self {
            Face::Sans => 0.088,
            Face::Serif => 0.064,
            Face::Mono => 0.029,
            Face::Custom => 0.060,
        }
    }

    /// Line spacing (fraction of the em) added so the pitch matches Unity after the glyph shrink.
    fn line_spacing_em(self) -> f32 {
        match self {
            Face::Sans => 0.086,
            Face::Serif => 0.056,
            Face::Mono => 0.031,
            Face::Custom => 0.058,
        }
    }
}

/// The shared multiplier, used with no face (inline `<size>` markup has none in scope).
const FONT_SIZE_DEFAULT: f32 = 19.514;
const GLYPH_SPACING_DEFAULT: f32 = 75.0;
const ANCHOR_OFFSET_DEFAULT: f32 = 0.088;
const LINE_SPACING_DEFAULT: f32 = 0.0;

/// Unity TMP autosize bounds, in TMP `fontSize` units.
pub const UNITY_TMP_FONT_SIZE_MIN: f32 = 18.0;
pub const UNITY_TMP_FONT_SIZE_MAX: f32 = 72.0;
/// Scales the TMP outline width to Godot's pixel `outline_size`.
const OUTLINE_WIDTH: f32 = 0.02;
/// TMP rect width (meters) to Label3D wrap width.
const LABEL_WIDTH: f32 = 200.0;
/// Shadow offset and blur as fractions of the em, scaled by the font size like the outline.
const SHADOW_OFFSET: f32 = 0.05;
const SHADOW_BLUR: f32 = 0.02;
/// Meters per glyph pixel: the Label3D `pixel_size`.
pub const PIXEL_SIZE: f32 = 0.005;

/// TMP `fontSize` in Godot glyph pixels.
pub fn unity_to_godot_font_size(tmp_font_size: f32, face: Option<Face>) -> f32 {
    tmp_font_size * face.map_or(FONT_SIZE_DEFAULT, Face::font_size_factor)
}

/// Local-space Y offset (meters, before the entity scale) added to the Label3D so its anchored
/// text lands where Unity places it. Only the top anchor is grid-calibrated; the bottom gets the
/// mirror and the middle is left (centering matches closely).
pub fn anchor_offset(face: Option<Face>, godot_font_size: i32, v_align: VerticalAlignment) -> f32 {
    let em = face.map_or(ANCHOR_OFFSET_DEFAULT, Face::anchor_offset_em);
    let dy = em * godot_font_size as f32 * PIXEL_SIZE;
    match v_align {
        VerticalAlignment::TOP => -dy,
        VerticalAlignment::BOTTOM => dy,
        _ => 0.0,
    }
}

/// Extra line spacing in glyph pixels.
pub fn line_spacing_px(face: Option<Face>, godot_font_size: i32) -> f32 {
    face.map_or(LINE_SPACING_DEFAULT, Face::line_spacing_em) * godot_font_size as f32
}

pub fn world_to_pixels(meters: f32) -> f32 {
    meters / PIXEL_SIZE
}

/// The raw TextShape fields, `None` where the scene set nothing.
#[derive(Clone, Debug, Default)]
pub struct TextShapeParams {
    pub text: String,
    pub font: Option<i32>,
    pub font_src: Option<String>,
    pub font_size: Option<f32>,
    pub font_auto_size: Option<bool>,
    pub text_align: Option<i32>,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub text_wrapping: Option<bool>,
    pub outline_width: Option<f32>,
    pub line_spacing: Option<f32>,
    pub text_color: Option<Color>,
    pub outline_color: Option<Color>,
    pub shadow_blur: Option<f32>,
    pub shadow_offset_x: Option<f32>,
    pub shadow_offset_y: Option<f32>,
    pub shadow_color: Option<Color>,
}

/// What a renderer needs, all numbers resolved.
#[derive(Clone, Debug)]
pub struct ResolvedText {
    pub godot_font_size: i32,
    pub outline_size_f: f32,
    pub fill_color: Color,
    pub outline_color: Color,
    pub plain_text: String,
    pub raw_text: String,
    pub h_align: HorizontalAlignment,
    pub v_align: VerticalAlignment,
    pub x_pos: f32,
    pub y_pos: f32,
    pub width_meter: f32,
    pub height_meter: f32,
    pub label_width: f32,
    pub text_wrapping: bool,
    pub shadow_color: Color,
    pub shadow_offset: Vector2,
    pub shadow_outline_size: f32,
    pub glyph_spacing_px: i32,
}

/// The largest font size (px) at which the text fits `width_world x height_world` meters, in
/// `[label_min, label_max]`. `measure(size)` is the text's size in pixels at that font size; word
/// wrap is off (only explicit newlines).
pub fn compute_auto_font_size(
    measure: impl Fn(i32) -> Vector2,
    width_world: f32,
    height_world: f32,
    label_min: i32,
    label_max: i32,
) -> i32 {
    let rect_w = world_to_pixels(width_world).max(1.0);
    let rect_h = world_to_pixels(height_world).max(1.0);
    let (mut lo, mut hi, mut best) = (label_min, label_max, label_min);
    while lo <= hi {
        let mid = lo + (hi - lo) / 2;
        let size = measure(mid);
        if size.x <= rect_w && size.y <= rect_h {
            best = mid;
            lo = mid + 1;
        } else {
            hi = mid - 1;
        }
    }
    best
}

/// Resolves the raw params for `face` (the face of the font, None for an unknown one).
/// `measure(text, size)` measures multiline text, for autosize.
pub fn resolve(
    params: &TextShapeParams,
    face: Option<Face>,
    measure: impl Fn(&str, i32) -> Vector2,
) -> ResolvedText {
    let raw_text = params.text.clone();

    let mut opacity = 1.0;
    let mut text_color = Color::WHITE;
    if let Some(color) = params.text_color {
        text_color = color;
        opacity = color.a;
    }

    // Tags stripped for the plain text, and the first inline color picked up.
    let (plain_text, tag_color) = strip_to_plain(&raw_text);
    let fill_rgb = tag_color.unwrap_or(text_color);
    let fill_color = Color::from_rgba(fill_rgb.r, fill_rgb.g, fill_rgb.b, opacity);

    let outline_rgb = params.outline_color.unwrap_or(Color::WHITE);
    let outline_color = Color::from_rgba(outline_rgb.r, outline_rgb.g, outline_rgb.b, opacity);

    let text_align = params.text_align.unwrap_or(4);
    let text_wrapping = params.text_wrapping.unwrap_or(false);
    let font_auto_size = params.font_auto_size.unwrap_or(false);

    let godot_font_size = if font_auto_size {
        // Unity's reference client fits the text inside (width, height) only when text_wrapping
        // is set; otherwise the rect is degenerate and TMP falls back to fontSizeMin.
        let fit_w = if text_wrapping {
            params.width.unwrap_or(1.0)
        } else {
            0.0
        };
        let fit_h = if text_wrapping {
            params.height.unwrap_or(1.0)
        } else {
            0.0
        };
        let min = unity_to_godot_font_size(UNITY_TMP_FONT_SIZE_MIN, face);
        if fit_w <= 0.0 || fit_h <= 0.0 {
            min
        } else {
            compute_auto_font_size(
                |size| measure(&plain_text, size),
                fit_w,
                fit_h,
                min as i32,
                unity_to_godot_font_size(UNITY_TMP_FONT_SIZE_MAX, face) as i32,
            ) as f32
        }
    } else {
        unity_to_godot_font_size(params.font_size.unwrap_or(3.0), face).max(1.0)
    };

    let outline_width = params.outline_width.unwrap_or(0.0);
    let outline_size_f = godot_font_size * outline_width * OUTLINE_WIDTH;

    let width_meter = if text_wrapping {
        params.width.unwrap_or(0.0)
    } else {
        0.0
    };
    let height_meter = if text_wrapping {
        params.height.unwrap_or(0.0)
    } else {
        0.0
    };
    let label_width = params.width.unwrap_or(16.0) * LABEL_WIDTH;

    let row = text_align / 3;
    let col = text_align % 3;
    let (v_align, y_pos) = match row {
        1 => (VerticalAlignment::CENTER, 0.0),
        2 => (VerticalAlignment::BOTTOM, -0.5),
        _ => (VerticalAlignment::TOP, 0.5),
    };
    let (h_align, x_pos) = match col {
        1 => (HorizontalAlignment::CENTER, 0.0),
        2 => (HorizontalAlignment::RIGHT, 0.5),
        _ => (HorizontalAlignment::LEFT, -0.5),
    };

    // Drop shadow: any shadow field enables it, in the given color (alpha = the text opacity) or
    // black. Without one the alpha is 0, so no shadow is drawn.
    let has_shadow = params.shadow_color.is_some()
        || params.shadow_offset_x.is_some()
        || params.shadow_offset_y.is_some()
        || params.shadow_blur.is_some();
    let shadow_rgb = params.shadow_color.unwrap_or(Color::BLACK);
    let shadow_alpha = if has_shadow { opacity } else { 0.0 };
    let shadow_color = Color::from_rgba(shadow_rgb.r, shadow_rgb.g, shadow_rgb.b, shadow_alpha);
    let (off_x, off_y, blur) = if has_shadow {
        (
            params.shadow_offset_x.unwrap_or(0.0),
            params.shadow_offset_y.unwrap_or(0.0),
            params.shadow_blur.unwrap_or(0.0),
        )
    } else {
        (0.0, 0.0, 0.0)
    };
    // Y inverted to match Unity's direction (a positive shadow_offset_y is downward).
    let shadow_offset = Vector2::new(
        godot_font_size * off_x * SHADOW_OFFSET,
        -(godot_font_size * off_y * SHADOW_OFFSET),
    );
    let shadow_outline_size = godot_font_size * blur * SHADOW_BLUR;

    let spacing_per_mille = face.map_or(GLYPH_SPACING_DEFAULT, Face::glyph_spacing_per_mille);
    let glyph_spacing_px = (spacing_per_mille / 1000.0 * godot_font_size).round() as i32;

    ResolvedText {
        godot_font_size: godot_font_size as i32,
        outline_size_f,
        fill_color,
        outline_color,
        plain_text,
        raw_text,
        h_align,
        v_align,
        x_pos,
        y_pos,
        width_meter,
        height_meter,
        label_width,
        text_wrapping,
        shadow_color,
        shadow_offset,
        shadow_outline_size,
        glyph_spacing_px,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_measure(_: &str, _: i32) -> Vector2 {
        Vector2::ZERO
    }

    fn params(text: &str) -> TextShapeParams {
        TextShapeParams {
            text: text.into(),
            ..Default::default()
        }
    }

    #[test]
    fn defaults_are_tmp_size_three_centered_white() {
        let r = resolve(&params("hi"), Some(Face::Sans), no_measure);
        assert_eq!(r.godot_font_size, (3.0 * 18.242) as i32);
        assert_eq!(r.fill_color, Color::WHITE);
        assert_eq!(r.h_align, HorizontalAlignment::CENTER);
        assert_eq!(r.v_align, VerticalAlignment::CENTER);
        assert_eq!((r.x_pos, r.y_pos), (0.0, 0.0));
        assert!(!r.text_wrapping);
        assert_eq!(r.label_width, 16.0 * 200.0);
    }

    #[test]
    fn alignment_rows_and_columns() {
        let mut p = params("x");
        p.text_align = Some(0);
        let r = resolve(&p, None, no_measure);
        assert_eq!(
            (r.v_align, r.h_align),
            (VerticalAlignment::TOP, HorizontalAlignment::LEFT)
        );
        assert_eq!((r.x_pos, r.y_pos), (-0.5, 0.5));
        p.text_align = Some(8);
        let r = resolve(&p, None, no_measure);
        assert_eq!(
            (r.v_align, r.h_align),
            (VerticalAlignment::BOTTOM, HorizontalAlignment::RIGHT)
        );
        assert_eq!((r.x_pos, r.y_pos), (0.5, -0.5));
    }

    #[test]
    fn opacity_comes_from_the_text_color_and_applies_to_outline_and_shadow() {
        let mut p = params("x");
        p.text_color = Some(Color::from_rgba(1.0, 0.0, 0.0, 0.4));
        p.shadow_blur = Some(1.0);
        let r = resolve(&p, None, no_measure);
        assert_eq!(r.fill_color.a, 0.4);
        assert_eq!(r.outline_color.a, 0.4);
        assert_eq!(r.shadow_color.a, 0.4);
        // Without a shadow field the shadow is invisible.
        let r = resolve(&params("x"), None, no_measure);
        assert_eq!(r.shadow_color.a, 0.0);
    }

    #[test]
    fn an_inline_color_beats_the_text_color() {
        let mut p = params("<color=blue>x</color>");
        p.text_color = Some(Color::from_rgba(1.0, 0.0, 0.0, 0.5));
        let r = resolve(&p, None, no_measure);
        assert_eq!(r.fill_color, Color::from_rgba(0.0, 0.0, 1.0, 0.5));
        assert_eq!(r.plain_text, "x");
        assert_eq!(r.raw_text, "<color=blue>x</color>");
    }

    #[test]
    fn wrapping_decides_whether_the_rect_is_used() {
        let mut p = params("x");
        p.width = Some(4.0);
        p.height = Some(2.0);
        let r = resolve(&p, None, no_measure);
        assert_eq!((r.width_meter, r.height_meter), (0.0, 0.0));
        p.text_wrapping = Some(true);
        let r = resolve(&p, None, no_measure);
        assert_eq!((r.width_meter, r.height_meter), (4.0, 2.0));
    }

    #[test]
    fn autosize_without_wrapping_uses_the_minimum() {
        let mut p = params("x");
        p.font_auto_size = Some(true);
        let r = resolve(&p, Some(Face::Mono), no_measure);
        assert_eq!(r.godot_font_size, (18.0 * 19.122) as i32);
    }

    #[test]
    fn autosize_finds_the_largest_fit() {
        // Text 2 px wide per font px: a 100 px rect fits up to size 50.
        let fit = compute_auto_font_size(
            |s| Vector2::new(s as f32 * 2.0, 10.0),
            100.0 * PIXEL_SIZE,
            1000.0,
            10,
            200,
        );
        assert_eq!(fit, 50);
        // Nothing fits: the minimum.
        let fit = compute_auto_font_size(|_| Vector2::new(1e6, 1e6), 1.0, 1.0, 10, 200);
        assert_eq!(fit, 10);
    }

    #[test]
    fn autosize_with_a_wrapping_rect_resolves_through_the_measure() {
        let mut p = params("abc");
        p.font_auto_size = Some(true);
        p.text_wrapping = Some(true);
        p.width = Some(1.0);
        p.height = Some(1.0);
        let r = resolve(&p, None, |_, size| Vector2::new(size as f32, size as f32));
        // 1 m is 200 px: the largest size within [18*19.514, 72*19.514] that is <= 200 is the min.
        assert_eq!(r.godot_font_size, (18.0 * 19.514) as i32);
    }

    #[test]
    fn faces_have_distinct_corrections() {
        assert_eq!(Face::from_file_name("NotoSerif.ttf"), Some(Face::Serif));
        assert_eq!(Face::from_file_name("other.ttf"), None);
        assert_eq!(Face::from_index(2), Face::Mono);
        assert_eq!(Face::from_index(9), Face::Sans);
        assert!(line_spacing_px(Some(Face::Sans), 100) > line_spacing_px(Some(Face::Mono), 100));
        assert_eq!(line_spacing_px(None, 100), 0.0);
    }

    #[test]
    fn a_scene_font_uses_the_mean_of_the_faces() {
        let mean = |f: fn(Face) -> f32| (f(Face::Sans) + f(Face::Serif) + f(Face::Mono)) / 3.0;
        assert!((Face::Custom.font_size_factor() - mean(Face::font_size_factor)).abs() < 0.01);
        assert!(
            (Face::Custom.glyph_spacing_per_mille() - mean(Face::glyph_spacing_per_mille)).abs()
                < 0.5
        );
        assert!((Face::Custom.anchor_offset_em() - mean(Face::anchor_offset_em)).abs() < 0.005);
        assert!((Face::Custom.line_spacing_em() - mean(Face::line_spacing_em)).abs() < 0.005);
    }

    #[test]
    fn the_top_anchor_moves_down_and_the_bottom_up() {
        let top = anchor_offset(Some(Face::Sans), 100, VerticalAlignment::TOP);
        let bottom = anchor_offset(Some(Face::Sans), 100, VerticalAlignment::BOTTOM);
        assert!(top < 0.0 && (top + bottom).abs() < 1e-6);
        assert_eq!(
            anchor_offset(Some(Face::Sans), 100, VerticalAlignment::CENTER),
            0.0
        );
    }

    #[test]
    fn glyph_spacing_scales_with_the_font() {
        let r = resolve(&params("x"), Some(Face::Sans), no_measure);
        assert_eq!(
            r.glyph_spacing_px,
            (0.061 * r.godot_font_size as f32).round() as i32
        );
    }

    #[test]
    fn outline_and_shadow_scale_with_the_font_size() {
        let mut p = params("x");
        p.outline_width = Some(2.0);
        p.shadow_offset_x = Some(1.0);
        p.shadow_offset_y = Some(1.0);
        p.shadow_blur = Some(1.0);
        let r = resolve(&p, None, no_measure);
        let size = 3.0 * 19.514;
        assert!((r.outline_size_f - size * 2.0 * 0.02).abs() < 1e-3);
        assert!((r.shadow_offset.x - size * 0.05).abs() < 1e-3);
        assert!((r.shadow_offset.y + size * 0.05).abs() < 1e-3);
        assert!((r.shadow_outline_size - size * 0.02).abs() < 1e-3);
    }
}
