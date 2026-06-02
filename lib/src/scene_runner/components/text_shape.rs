use crate::{
    dcl::{
        components::{
            proto_components::sdk::components::common::{Font, TextAlignMode},
            SceneComponentId,
        },
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
        ui_text_tags::strip_tags_extract_first_color,
    },
    scene_runner::scene::Scene,
};
use godot::{
    classes::{
        label_3d::AlphaCutMode,
        text_server::{AutowrapMode, JustificationFlag, LineBreakFlag},
        Label3D, Node,
    },
    global::{HorizontalAlignment, VerticalAlignment},
    prelude::*,
};

/// Empirical correction factor between Unity TMP's expected sizing and the
/// effective sizing in the live DCL Unity client. Visual tuning against the
/// reference rendering (both autosize-fallback and non-autosize cases) landed
/// on 17/18 — apply it once and have it flow through every TMP-derived
/// constant below.
const DCL_TMP_SIZE_FACTOR: f32 = 17.0 / 18.0;

/// Conversion between Unity TextMeshPro `fontSize` (world units per em) and
/// Godot `Label3D.font_size` (glyph pixels). Baseline test confirmed that SDK
/// `font_size = 1.4` renders identically in both clients when Label3D
/// `font_size = 30` and `pixel_size = 0.005`, i.e. 30 / 1.4 ≈ 21.4 — kept at
/// 22.0 to round-trip cleanly with `i32` sizes, then scaled by
/// `DCL_TMP_SIZE_FACTOR` to absorb the same correction the autosize bounds
/// use.
const TMP_TO_LABEL3D_FONT_SIZE: f32 = 22.0 * DCL_TMP_SIZE_FACTOR;

/// Unity TMP's autosize bounds, in TMP `fontSize` units. The component
/// documentation lists `fontSizeMin = 18` and `fontSizeMax = 72` as defaults.
/// Kept raw — the `DCL_TMP_SIZE_FACTOR` correction is carried by
/// `TMP_TO_LABEL3D_FONT_SIZE` and applied exactly once on conversion to
/// Label3D pixels.
const UNITY_TMP_FONT_SIZE_MIN: f32 = 18.0;
const UNITY_TMP_FONT_SIZE_MAX: f32 = 72.0;

/// Unity TMP's `_OutlineWidth` is a 0..1 shader-domain SDF distance — the
/// outline appears as a thin ring drawn outside the glyph silhouette without
/// expanding the glyph itself.
///
/// Godot's `Label3D.outline_size` is raw pixels and, by default, the outline
/// is rasterized underneath the glyph so the silhouette grows by
/// `outline_size` on each side — visually equivalent to making the font bolder.
///
/// To match Unity's "ring around the existing silhouette" look we:
///   1. Set `outline_render_priority = 1` so the outline is drawn on top of
///      the glyph fill — the outline color now overlays the glyph edge instead
///      of stretching the silhouette outward.
///   2. Scale the computed `outline_size` way down so the outline covers only
///      a thin ring at the glyph edge and doesn't eat the glyph interior.
///
/// (1) + (2) together approximate Unity's outline visual without needing a
/// custom shader. Tune (2) empirically per the test scene.
const TMP_TO_LABEL3D_OUTLINE_WIDTH: f32 = 0.8;

/// Converts a Unity TMP `fontSize` (em units from the SDK proto / autosize
/// bounds) to a Godot `Label3D.font_size` in glyph pixels. Applies the
/// `DCL_TMP_SIZE_FACTOR` correction exactly once.
fn unity_to_godot_font_size(tmp_font_size: f32) -> f32 {
    tmp_font_size * TMP_TO_LABEL3D_FONT_SIZE
}

/// Converts a Unity TMP `_OutlineWidth` (0..1 SDF distance) and the resolved
/// Godot glyph `font_size` (in pixels) to a `Label3D.outline_size`. The
/// `TMP_TO_LABEL3D_OUTLINE_WIDTH` scale keeps the outline as a thin ring on
/// the glyph edge rather than a glyph-thickening band. See the constant docs.
fn unity_to_godot_outline_size(godot_font_size: f32, tmp_outline_width: f32) -> f32 {
    godot_font_size * tmp_outline_width * TMP_TO_LABEL3D_OUTLINE_WIDTH
}

pub fn update_text_shape(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let dirty_lww_components = &scene.current_dirty.lww_components;
    if let Some(text_shape_dirty) = dirty_lww_components.get(&SceneComponentId::TEXT_SHAPE) {
        let text_shape_component = SceneCrdtStateProtoComponents::get_text_shape(crdt_state);

        for entity in text_shape_dirty {
            let new_value = text_shape_component.get(entity);
            if new_value.is_none() {
                continue;
            }

            let new_value = new_value.unwrap();
            let (_godot_entity_node, mut node_3d) = godot_dcl_scene.ensure_node_3d(entity);

            let new_value = new_value.value.clone();
            let existing = node_3d.try_get_node_as::<Label3D>("TextShape");

            if new_value.is_none() {
                if let Some(mut text_shape_node) = existing {
                    text_shape_node.queue_free();
                    node_3d.remove_child(&text_shape_node.upcast::<Node>());
                }
            } else if let Some(new_value) = new_value {
                let (mut label_3d, add_to_base) = match existing {
                    Some(label_3d) => (label_3d, false),
                    None => (Label3D::new_alloc(), true),
                };

                let text_align = TextAlignMode::from_i32(
                    new_value
                        .text_align
                        .unwrap_or(TextAlignMode::TamMiddleCenter as i32),
                )
                .unwrap_or(TextAlignMode::TamMiddleCenter);
                let opacity = new_value
                    .text_color
                    .as_ref()
                    .map(|color| color.a)
                    .unwrap_or(1.0);

                // Process text: strip Unity tags and extract first color
                let (display_text, tag_color) =
                    if let Some(strip_result) = strip_tags_extract_first_color(&new_value.text) {
                        let color = strip_result.first_color.and_then(|c| parse_color(&c));
                        (strip_result.text, color)
                    } else {
                        (new_value.text.clone(), None)
                    };

                // Use tag color if found, otherwise use the default text_color
                let text_color = tag_color
                    .map(|c| Color::from_rgba(c.0, c.1, c.2, opacity))
                    .unwrap_or_else(|| {
                        new_value
                            .text_color
                            .map(|color| Color::from_rgba(color.r, color.g, color.b, opacity))
                            .unwrap_or(Color::from_rgba(1.0, 1.0, 1.0, opacity))
                    });

                let outline_color = new_value
                    .outline_color
                    .map(|color| Color::from_rgba(color.r, color.g, color.b, opacity))
                    .unwrap_or(Color::from_rgba(1.0, 1.0, 1.0, opacity));

                label_3d.set_text(&display_text);
                label_3d.set_modulate(text_color);

                let new_font = match new_value.font {
                    Some(0) => Font::FSansSerif,
                    Some(1) => Font::FSerif,
                    Some(2) => Font::FMonospace,
                    _ => Font::FSansSerif,
                };
                let font_resource = new_font.get_font_resource();

                let text_wrapping = new_value.text_wrapping.unwrap_or_default();

                let font_size = if new_value.font_auto_size.unwrap_or(false) {
                    // Replicate Unity's reference client (`TMPProSdkExtensions.cs`),
                    // including its quirks — content authors have visually tuned
                    // their scenes against this behavior, so matching the proto's
                    // "fit in width/height" contract literally would break them.
                    //
                    //   tmpText.enableAutoSizing = textShape.FontAutoSize;
                    //   tmpText.rectTransform.sizeDelta =
                    //       textShape.TextWrapping ? new Vector2(w,h) : Vector2.zero;
                    //   tmpText.enableWordWrapping = TextWrapping && !enableAutoSizing;
                    //
                    // Behavior that follows:
                    //   - autosize=true, text_wrapping=true  → TMP fits text in
                    //     (width, height), no word wrap, fontSize ∈ [18, 72].
                    //   - autosize=true, text_wrapping=false → sizeDelta = (0,0),
                    //     fit-check always fails, TMP falls back to fontSizeMin=18.
                    //
                    // This deliberately ignores the proto's `width`/`height` when
                    // `text_wrapping=false`, matching Unity. See
                    // `lib/src/dcl/components/proto/decentraland/sdk/components/text_shape.proto:22`
                    // for the spec we are intentionally not following.
                    let (fit_w, fit_h) = if text_wrapping {
                        (
                            new_value.width.unwrap_or(1.0),
                            new_value.height.unwrap_or(1.0),
                        )
                    } else {
                        (0.0, 0.0)
                    };
                    if fit_w <= 0.0 || fit_h <= 0.0 {
                        // Unity TMP fallback when the autosize rect is degenerate.
                        unity_to_godot_font_size(UNITY_TMP_FONT_SIZE_MIN)
                    } else {
                        compute_auto_font_size(
                            &font_resource,
                            &display_text,
                            fit_w,
                            fit_h,
                            label_3d.get_pixel_size(),
                            unity_to_godot_font_size(UNITY_TMP_FONT_SIZE_MIN) as i32,
                            unity_to_godot_font_size(UNITY_TMP_FONT_SIZE_MAX) as i32,
                        ) as f32
                    }
                } else {
                    unity_to_godot_font_size(new_value.font_size.unwrap_or(3.0)).max(1.0)
                };
                let outline_size =
                    unity_to_godot_outline_size(font_size, new_value.outline_width.unwrap_or(0.0));
                label_3d.set_font_size(font_size as i32);
                label_3d.set_outline_size(outline_size as i32);
                label_3d.set_outline_modulate(outline_color);
                // Draw the outline on top of the glyph fill so it overlays the
                // glyph edge (matching Unity TMP) instead of expanding the
                // silhouette outward. See `TMP_TO_LABEL3D_OUTLINE_WIDTH` notes.
                label_3d.set_outline_render_priority(1);
                label_3d.set_alpha_cut_mode(AlphaCutMode::DISCARD);

                let (width_meter, height_meter) = if text_wrapping {
                    (
                        new_value.width.unwrap_or(0.0),
                        new_value.height.unwrap_or(0.0),
                    )
                } else {
                    (0.0, 0.0)
                };

                label_3d.set_autowrap_mode(if text_wrapping {
                    AutowrapMode::WORD_SMART
                } else {
                    AutowrapMode::OFF
                });
                label_3d.set_width(200.0 * new_value.width.unwrap_or(16.0));

                let (v_align, y_pos) = match text_align {
                    TextAlignMode::TamMiddleLeft
                    | TextAlignMode::TamMiddleRight
                    | TextAlignMode::TamMiddleCenter => (VerticalAlignment::CENTER, 0.0),
                    TextAlignMode::TamTopLeft
                    | TextAlignMode::TamTopRight
                    | TextAlignMode::TamTopCenter => (VerticalAlignment::TOP, 0.5),
                    TextAlignMode::TamBottomLeft
                    | TextAlignMode::TamBottomRight
                    | TextAlignMode::TamBottomCenter => (VerticalAlignment::BOTTOM, -0.5),
                };

                let (h_align, x_pos) = match text_align {
                    TextAlignMode::TamMiddleLeft
                    | TextAlignMode::TamTopLeft
                    | TextAlignMode::TamBottomLeft => (HorizontalAlignment::LEFT, -0.5),
                    TextAlignMode::TamMiddleRight
                    | TextAlignMode::TamTopRight
                    | TextAlignMode::TamBottomRight => (HorizontalAlignment::RIGHT, 0.5),
                    TextAlignMode::TamMiddleCenter
                    | TextAlignMode::TamTopCenter
                    | TextAlignMode::TamBottomCenter => (HorizontalAlignment::CENTER, 0.0),
                };

                label_3d.set_position(Vector3::new(width_meter * x_pos, height_meter * y_pos, 0.0));
                label_3d.set_vertical_alignment(v_align);
                label_3d.set_horizontal_alignment(h_align);

                label_3d.set_font(&font_resource);
                if add_to_base {
                    label_3d.set_name("TextShape");
                    node_3d.add_child(&label_3d.upcast::<Node>());
                }

                // TODO: missing properties
                // - padding (left/right/top/bottom)
                // - shadow (offsetX/offsetY/blur/color) (on Unity: it's actually an overlay)
                // - line_spacing (on Unity: it doesn't work)
                // - line_count (on Unity: it truncates instead of setting up the wrapping)
            }
        }
    }
}

/// Returns the largest Label3D `font_size` (in glyph pixels) for which `text`
/// fits inside a `width × height` world-space rect, clamped to
/// `[label_min, label_max]`. Word wrap is **off** while autosizing — matching
/// Unity's `enableWordWrapping = TextWrapping && !enableAutoSizing` (i.e. when
/// autosize is on, word wrap is forced off; only explicit `\n` line breaks
/// honored). If even `label_min` doesn't fit, returns `label_min` — Unity TMP
/// behaves the same with `overflowMode = Overflow`.
fn compute_auto_font_size(
    font: &Gd<godot::classes::Font>,
    text: &str,
    width_world: f32,
    height_world: f32,
    pixel_size: f32,
    label_min: i32,
    label_max: i32,
) -> i32 {
    let rect_w_px = (width_world / pixel_size).max(1.0);
    let rect_h_px = (height_world / pixel_size).max(1.0);

    let fits = |fs: i32| -> bool {
        let measured = font
            .get_multiline_string_size_ex(text)
            .max_lines(-1)
            .width(-1.0)
            .font_size(fs)
            .justification_flags(JustificationFlag::NONE)
            .brk_flags(LineBreakFlag::MANDATORY)
            .done();
        measured.x <= rect_w_px && measured.y <= rect_h_px
    };

    // Binary search `[label_min, label_max]` for the largest size that fits;
    // ~8 measurements for the typical [396, 1584] range.
    let mut lo = label_min;
    let mut hi = label_max;
    let mut best = label_min;
    while lo <= hi {
        let mid = lo + (hi - lo) / 2;
        if fits(mid) {
            best = mid;
            lo = mid + 1;
        } else {
            hi = mid - 1;
        }
    }
    best
}

/// Parses a color string (named color or hex) into RGB values (0.0-1.0)
fn parse_color(color: &str) -> Option<(f32, f32, f32)> {
    let color = color.trim().to_lowercase();

    // Named colors (common Unity/CSS colors)
    match color.as_str() {
        "red" => return Some((1.0, 0.0, 0.0)),
        "green" => return Some((0.0, 0.5, 0.0)),
        "blue" => return Some((0.0, 0.0, 1.0)),
        "white" => return Some((1.0, 1.0, 1.0)),
        "black" => return Some((0.0, 0.0, 0.0)),
        "yellow" => return Some((1.0, 1.0, 0.0)),
        "cyan" => return Some((0.0, 1.0, 1.0)),
        "magenta" => return Some((1.0, 0.0, 1.0)),
        "gray" | "grey" => return Some((0.5, 0.5, 0.5)),
        "orange" => return Some((1.0, 0.65, 0.0)),
        "purple" => return Some((0.5, 0.0, 0.5)),
        "pink" => return Some((1.0, 0.75, 0.8)),
        "brown" => return Some((0.65, 0.16, 0.16)),
        "lime" => return Some((0.0, 1.0, 0.0)),
        "navy" => return Some((0.0, 0.0, 0.5)),
        "teal" => return Some((0.0, 0.5, 0.5)),
        "olive" => return Some((0.5, 0.5, 0.0)),
        "maroon" => return Some((0.5, 0.0, 0.0)),
        "aqua" => return Some((0.0, 1.0, 1.0)),
        "silver" => return Some((0.75, 0.75, 0.75)),
        "fuchsia" => return Some((1.0, 0.0, 1.0)),
        _ => {}
    }

    // Hex color (#RGB, #RRGGBB, or #RRGGBBAA)
    if let Some(hex) = color.strip_prefix('#') {
        match hex.len() {
            3 => {
                // #RGB -> expand to #RRGGBB
                let r = u8::from_str_radix(&hex[0..1], 16).ok()?;
                let g = u8::from_str_radix(&hex[1..2], 16).ok()?;
                let b = u8::from_str_radix(&hex[2..3], 16).ok()?;
                return Some((
                    (r * 17) as f32 / 255.0,
                    (g * 17) as f32 / 255.0,
                    (b * 17) as f32 / 255.0,
                ));
            }
            6 | 8 => {
                // #RRGGBB or #RRGGBBAA (ignore alpha)
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                return Some((r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0));
            }
            _ => {}
        }
    }

    None
}
