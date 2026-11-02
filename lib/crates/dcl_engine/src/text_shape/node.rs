//! `DclTextShape`: one `Label3D` per TextShape entity.
//!
//! On a stock engine the label renders plain text (tags stripped, the first inline color applied
//! to the whole label). Our Godot fork adds `bbcode_enabled`, a bold face, `character_spacing`,
//! drop shadow and a float outline to `Label3D`: they are feature-detected so the same code runs
//! on a stock 4.6.2 that lacks them, and reached by name because the bindings are generated from
//! the stock API.

use std::cell::RefCell;
use std::collections::HashMap;

use godot::classes::label_3d::AlphaCutMode;
use godot::classes::node::AutoTranslateMode;
use godot::classes::text_server::{AutowrapMode, JustificationFlag, LineBreakFlag, SpacingType};
use godot::classes::{Font, FontFile, FontVariation, INode3D, Label3D};
use godot::global::HorizontalAlignment;
use godot::prelude::*;

use super::fallbacks::with_glyph_fallbacks;
use super::glyph_prewarm::{self, GlyphRequest};
use super::layout::{
    anchor_offset, line_spacing_px, resolve, Face, ResolvedText, TextShapeParams, PIXEL_SIZE,
};
use super::markup::to_bbcode;
use crate::content::content_mapping::ContentMappingAndUrlRef;
use crate::content::font::faces_from_promise;
use crate::godot_classes::font::{custom_font_variation, fetch_font, FontStyle};

thread_local! {
    /// Fonts with extra glyph spacing baked in, by base face and spacing.
    static STYLED_FONTS: RefCell<HashMap<(String, i32), Gd<FontVariation>>> =
        RefCell::new(HashMap::new());
}

/// `base` with `glyph_spacing` extra px between glyphs, cached (`base` itself for 0). One
/// FontVariation carries it: nesting variations would drop the inner one's properties.
fn spaced_font(base: &Gd<Font>, glyph_spacing: i32) -> Gd<Font> {
    if glyph_spacing == 0 {
        return base.clone();
    }
    let key = (base.get_path().to_string(), glyph_spacing);
    STYLED_FONTS.with(|cache| {
        cache
            .borrow_mut()
            .entry(key)
            .or_insert_with(|| {
                let mut variation = FontVariation::new_gd();
                variation.set_base_font(base);
                variation.set_spacing(SpacingType::GLYPH, glyph_spacing);
                variation
            })
            .clone()
            .upcast()
    })
}

fn face_of(font: Option<&Gd<Font>>) -> Option<Face> {
    let path = font?.get_path().to_string();
    Face::from_file_name(path.rsplit('/').next().unwrap_or(&path))
}

/// Trims every line's edge whitespace: Label3D's own trimming is off so blank lines survive,
/// and trailing padding spaces must not skew alignment.
fn trim_line_edges(text: &str) -> String {
    text.split('\n')
        .map(str::trim)
        .collect::<Vec<_>>()
        .join("\n")
}

/// The scene font a `font_src` resolved to: the regular face and, when the font has one, the
/// real bold face (faux bold mangles MSDF glyphs).
struct CustomFont {
    regular: Gd<Font>,
    bold: Gd<Font>,
}

#[derive(GodotClass)]
#[class(init, base=Node3D)]
pub struct DclTextShape {
    label: Option<Gd<Label3D>>,
    font: Option<Gd<Font>>,
    bold_font: Option<Gd<Font>>,
    font_index: Option<i32>,
    font_src: String,
    custom: Option<CustomFont>,
    params: Option<TextShapeParams>,
    content_mapping: Option<ContentMappingAndUrlRef>,
    /// The text to show once its glyphs are rendered, and the one a running job will show.
    resolved: Option<ResolvedText>,
    job_resolved: Option<ResolvedText>,
    glyph_task: Option<i64>,
    base: Base<Node3D>,
}

#[godot_api]
impl INode3D for DclTextShape {}

impl DclTextShape {
    /// Applies the TextShape fields; called each time the component changes.
    pub fn apply(&mut self, params: &TextShapeParams, content_mapping: &ContentMappingAndUrlRef) {
        self.resolve_font(
            params.font.unwrap_or(0),
            params.font_src.as_deref().unwrap_or_default(),
            content_mapping,
        );
        self.params = Some(params.clone());
        self.content_mapping = Some(content_mapping.clone());
        self.layout();
    }

    /// Resolves the params against the current font and shows them once their glyphs exist.
    fn layout(&mut self) {
        let Some(params) = self.params.clone() else {
            return;
        };
        let face = self.face();
        let font = self.font.clone();
        let resolved = resolve(&params, face, |text, size| match font.as_ref() {
            Some(font) => font
                .get_multiline_string_size_ex(text)
                .alignment(HorizontalAlignment::LEFT)
                .width(-1.0)
                .font_size(size)
                .max_lines(-1)
                .brk_flags(LineBreakFlag::MANDATORY)
                .justification_flags(JustificationFlag::NONE)
                .done(),
            None => Vector2::ZERO,
        });
        self.ensure_label();
        self.resolved = Some(resolved);
        self.show_when_rendered();
    }

    fn face(&self) -> Option<Face> {
        if self.custom.is_some() {
            Some(Face::Custom)
        } else {
            face_of(self.font.as_ref())
        }
    }

    /// The glyphs the latest text needs, in the fonts it draws with.
    fn glyph_request(&self) -> Option<GlyphRequest> {
        let resolved = self.resolved.as_ref()?;
        let mut fonts: Vec<Gd<Font>> = self.font.iter().cloned().collect();
        if resolved.raw_text.to_lowercase().contains("<b>") {
            fonts.extend(self.bold_font.iter().cloned());
        }
        Some(GlyphRequest {
            fonts,
            text: resolved.plain_text.clone(),
            size: resolved.godot_font_size,
        })
    }

    /// Shows the latest text right away when its glyphs are rendered, otherwise after a worker
    /// renders them.
    fn show_when_rendered(&mut self) {
        let Some(request) = self.glyph_request() else {
            return;
        };
        let owner = self.base().instance_id();
        if glyph_prewarm::request(owner, &mut self.glyph_task, &request) {
            if let Some(resolved) = self.resolved.clone() {
                self.show(&resolved);
            }
        } else if self.glyph_task.is_some() && self.job_resolved.is_none() {
            self.job_resolved = self.resolved.clone();
        }
    }

    /// A glyph worker (or the disk cache restore) finished.
    pub fn glyphs_rendered(&mut self) {
        if let Some(task) = self.glyph_task.take() {
            glyph_prewarm::join(task);
        }
        let job = self.job_resolved.take();
        // Text changing faster than glyphs render would never be shown: show the text the job
        // rendered while the newer one renders.
        if let (Some(job), Some(request)) = (job, self.glyph_request()) {
            if !glyph_prewarm::is_ready(&request) {
                self.show(&job);
            }
        }
        self.show_when_rendered();
    }

    /// Loads the faces when the font index or `font_src` changed, re-seeding an existing label's
    /// bold face.
    fn resolve_font(&mut self, index: i32, src: &str, content_mapping: &ContentMappingAndUrlRef) {
        let src = src.trim();
        if self.font_index == Some(index) && self.font_src == src && self.font.is_some() {
            // Retry a failed load (within the scene's font budget).
            if !src.is_empty() && self.custom.is_none() {
                self.request_custom_font(src, content_mapping);
                if self.custom.is_some() {
                    self.apply_fonts();
                }
            }
            return;
        }
        self.font_index = Some(index);
        if src != self.font_src {
            self.font_src = src.to_owned();
            self.custom = None;
            if !src.is_empty() {
                self.request_custom_font(src, content_mapping);
            }
        }
        self.apply_fonts();
    }

    /// The scene font once loaded (the built-in face stays as glyph fallback), otherwise the
    /// built-in face for the `font` enum.
    fn apply_fonts(&mut self) {
        let face = Face::from_index(self.font_index.unwrap_or(0));
        let builtin = |uid: &str, bold: bool| {
            try_load::<Font>(uid)
                .ok()
                .map(|font| with_glyph_fallbacks(font, bold))
        };
        match self.custom.as_ref() {
            Some(custom) => {
                self.font = Some(custom.regular.clone());
                self.bold_font = Some(custom.bold.clone());
            }
            None => {
                self.font = builtin(face.font_uid(), false);
                self.bold_font = builtin(face.bold_font_uid(), true);
            }
        }
        if let Some(label) = self.label.as_mut() {
            label.set_bold_font(self.bold_font.as_ref());
        }
    }

    fn request_custom_font(&mut self, src: &str, content_mapping: &ContentMappingAndUrlRef) {
        let promise = fetch_font(src, content_mapping, true);
        if promise.bind().is_resolved() {
            self.custom = self.custom_from(&promise);
        } else {
            let src = src.to_owned();
            promise
                .signals()
                .on_resolved()
                .connect_other(&self.to_gd(), {
                    let promise = promise.clone();
                    move |this: &mut Self| this.custom_font_loaded(&src, &promise)
                });
        }
    }

    fn custom_font_loaded(
        &mut self,
        src: &str,
        promise: &Gd<crate::godot_classes::promise::Promise>,
    ) {
        if src != self.font_src {
            return;
        }
        self.custom = self.custom_from(promise);
        if self.custom.is_none() {
            return;
        }
        self.apply_fonts();
        self.layout();
    }

    fn custom_from(
        &self,
        promise: &Gd<crate::godot_classes::promise::Promise>,
    ) -> Option<CustomFont> {
        let faces = faces_from_promise(promise)?;
        let face = |style: FontStyle| -> Option<Gd<Font>> {
            let file = faces.get(style.key())?.try_to::<Gd<FontFile>>().ok()?;
            let builtin =
                try_load::<Font>(Face::from_index(self.font_index.unwrap_or(0)).font_uid())
                    .ok()
                    .map(|font| with_glyph_fallbacks(font, false))?;
            Some(custom_font_variation(file, FontStyle::Regular, builtin).upcast())
        };
        let regular = face(FontStyle::Regular)?;
        let bold = face(FontStyle::Bold).unwrap_or_else(|| regular.clone());
        Some(CustomFont { regular, bold })
    }

    fn ensure_label(&mut self) {
        if self.label.is_some() {
            return;
        }
        let mut label = Label3D::new_alloc();
        label.set_name("Label3D");
        // Scene-authored content, not UI copy: Label3D translates its text, which would put
        // every creator's TextShape through a translation lookup.
        label.set_auto_translate_mode(AutoTranslateMode::DISABLED);
        // DISCARD with outline_render_priority 1: the fork orders the shadow, outline and fill
        // layers under DISCARD through a small per-priority Z shift, so the outline ring sits
        // right relative to the fill (no z-fight, no outline over the glyph).
        label.set_alpha_cut_mode(AlphaCutMode::DISCARD);
        // Hard-cutoff alpha: the text stays visible as opacity drops, then vanishes entirely
        // once coverage times opacity falls below this (a TextShape opacity under ~0.1).
        label.set_alpha_scissor_threshold(0.1);
        label.set_bbcode_enabled(true);
        label.set_bold_font(self.bold_font.as_ref());
        label.set_outline_render_priority(1);
        // Label3D's own edge trimming would drop a "\n\n" blank line: trimming is ours.
        label.set_autowrap_trim_flags(LineBreakFlag::NONE);
        self.base_mut().add_child(&label);
        self.label = Some(label);
    }

    fn show(&mut self, resolved: &ResolvedText) {
        let face = self.face();
        self.apply_resolved(resolved, face);
    }

    fn apply_resolved(&mut self, resolved: &ResolvedText, face: Option<Face>) {
        let font = self.font.clone();
        let Some(label) = self.label.as_mut() else {
            return;
        };
        label.set_pixel_size(PIXEL_SIZE);
        let spacing = resolved.glyph_spacing_px;
        if let Some(font) = font.as_ref() {
            // The fork's native character_spacing carries the spacing: baking it into the font
            // too would double it.
            label.set_font(&spaced_font(font, 0));
        }
        label.set_character_spacing(spacing);
        // A transparent shadow color turns the shadow off.
        label.set_shadow_color(resolved.shadow_color);
        label.set_shadow_offset(resolved.shadow_offset);
        label.set_shadow_outline_size(resolved.shadow_outline_size);
        let text = trim_line_edges(&to_bbcode(&resolved.raw_text, resolved.godot_font_size));
        label.set_text(&text);
        label.set_modulate(resolved.fill_color);
        label.set_outline_modulate(resolved.outline_color);
        label.set_font_size(resolved.godot_font_size);
        label.set_line_spacing(line_spacing_px(face, resolved.godot_font_size));
        // The fork falls back to outline_size when the float is 0, so a lingering integer
        // would draw an outline on the no-outline case.
        label.set_outline_size(0);
        label.set_outline_size_float(resolved.outline_size_f);
        label.set_autowrap_mode(if resolved.text_wrapping {
            AutowrapMode::WORD_SMART
        } else {
            AutowrapMode::OFF
        });
        label.set_width(resolved.label_width);
        label.set_horizontal_alignment(resolved.h_align);
        label.set_vertical_alignment(resolved.v_align);
        // Unity TMP anchors the first line a little lower than Label3D hangs it (#2371).
        label.set_position(Vector3::new(
            resolved.width_meter * resolved.x_pos,
            resolved.height_meter * resolved.y_pos
                + anchor_offset(face, resolved.godot_font_size, resolved.v_align),
            0.0,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::trim_line_edges;

    #[test]
    fn lines_are_trimmed_but_blank_lines_survive() {
        assert_eq!(trim_line_edges("  a  \n\n b"), "a\n\nb");
        assert_eq!(trim_line_edges("x"), "x");
    }
}
