//! Owns the single avatar preview the shell's screens draw through a hole.

use std::cell::{Cell, RefCell};

use godot::classes::control::{LayoutPreset, MouseFilter};
use godot::classes::sub_viewport::UpdateMode;
use godot::classes::texture_rect::{ExpandMode, StretchMode};
use godot::classes::{
    CanvasLayer, Control, DisplayServer, ImageTexture, Node, PackedScene, TextureRect,
};
use godot::prelude::*;

use super::state;
use crate::avatars::avatar::build::update_from_profile;
use crate::avatars::avatar::emote_load::async_play_emote_or_stash;
use crate::avatars::emotes;
use crate::avatars::preview::{AvatarPreview, PreviewSpinner};
use crate::engine::local_task::{await_all, next_frame};
use crate::fetcher::globals;

const PREVIEW_SCENE: &str = "res://src/ui/pages/backpack/avatar_preview.tscn";
/// In explorer mode only the preview is drawn by the main viewport (nameplates and other 2D layers
/// are culled); `disable_3d` stops the world render.
const OVERLAY_CANVAS_LAYER: u32 = 1 << 19;

pub struct Backpack {
    /// The node the preview's canvas layer hangs from.
    host: Gd<Node>,
    layer: RefCell<Option<Gd<CanvasLayer>>>,
    clipper: RefCell<Option<Gd<Control>>>,
    preview: RefCell<Option<Gd<AvatarPreview>>>,
    canvas_cull_mask: Cell<u32>,
    preload_pending: Cell<bool>,
    preloaded_for: RefCell<String>,
    applying: Cell<bool>,
    pending_apply: RefCell<Option<VarDictionary>>,
    still: RefCell<Option<Gd<TextureRect>>>,
    spinner: RefCell<Option<Gd<PreviewSpinner>>>,
}

impl Backpack {
    pub fn new(host: Gd<Node>) -> Self {
        Self {
            host,
            layer: RefCell::new(None),
            clipper: RefCell::new(None),
            preview: RefCell::new(None),
            canvas_cull_mask: Cell::new(0),
            preload_pending: Cell::new(false),
            preloaded_for: RefCell::new(String::new()),
            applying: Cell::new(false),
            pending_apply: RefCell::new(None),
            still: RefCell::new(None),
            spinner: RefCell::new(None),
        }
    }

    fn preview(&self) -> Option<Gd<AvatarPreview>> {
        self.preview.borrow().clone()
    }

    fn address() -> String {
        globals::player_identity()
            .bind()
            .get_address_str()
            .to_string()
    }

    /// Builds the avatar in a hidden preview, so the screen that shows it only has to place it.
    pub async fn preload(&self) {
        if self.preload_pending.get() || self.preview().is_some() {
            return;
        }
        self.preload_pending.set(true);
        let preview = self.ensure_preview();
        preview.clone().set_visible(false);
        set_update_mode(&preview, UpdateMode::DISABLED);
        let address = Self::address();
        if self.refresh_avatar().await {
            *self.preloaded_for.borrow_mut() = address;
            // The shell can dress the first look before it opens the preview.
            self.emit_avatar();
        }
        self.preload_pending.set(false);
    }

    pub async fn open(&self, rect: Rect2) {
        let preview = self.ensure_preview();
        // A look being dressed is what the screen shows, not the one it replaces.
        while self.applying.get() {
            next_frame().await;
        }
        self.show_preview(&preview, true);
        self.place(rect);
        while self.preload_pending.get() {
            next_frame().await;
        }
        if *self.preloaded_for.borrow() == Self::address() {
            self.preloaded_for.borrow_mut().clear();
        } else {
            self.refresh_avatar().await;
        }
        self.place(rect);
        self.emit_avatar();
    }

    fn ensure_preview(&self) -> Gd<AvatarPreview> {
        if let Some(preview) = self.preview() {
            return preview;
        }
        let mut layer = CanvasLayer::new_alloc();
        layer.set_layer(1);
        self.host.clone().add_child(&layer);
        let mut preview = load::<PackedScene>(PREVIEW_SCENE).instantiate_as::<AvatarPreview>();
        {
            let mut p = preview.bind_mut();
            p.set_fit_avatar(true);
            // The shell draws the name itself: no plate over the avatar, not even for a frame.
            p.hide_name = true;
        }
        preview.set_stretch(true);
        // The preview keeps its size while a card covers part of it: the clipper cuts the drawing.
        let mut clipper = Control::new_alloc();
        clipper.set_anchors_and_offsets_preset(LayoutPreset::FULL_RECT);
        clipper.set_clip_contents(true);
        clipper.set_mouse_filter(MouseFilter::IGNORE);
        clipper.set_visibility_layer(OVERLAY_CANVAS_LAYER);
        layer.add_child(&clipper);
        clipper.add_child(&preview);
        preview.set_visibility_layer(OVERLAY_CANVAS_LAYER);
        *self.clipper.borrow_mut() = Some(clipper);
        *self.layer.borrow_mut() = Some(layer);
        *self.preview.borrow_mut() = Some(preview.clone());
        preview
    }

    pub fn is_shown(&self) -> bool {
        self.preview().is_some_and(|p| p.is_visible())
    }

    /// RN reports device pixels; Godot positions UI in its scaled viewport units.
    pub fn place(&self, rect: Rect2) {
        let Some(mut preview) = self.preview() else {
            return;
        };
        preview.set_anchors_preset(LayoutPreset::TOP_LEFT);
        let window_width = DisplayServer::singleton().window_get_size().x;
        let scale = match self.host.get_viewport() {
            Some(v) if window_width > 0 => v.get_visible_rect().size.x / window_width as f32,
            _ => 1.0,
        };
        preview.set_position(rect.position * scale);
        preview.set_size(rect.size * scale);
    }

    /// Draws the preview only above this screen row, in device pixels; negative lifts the limit.
    pub fn clip_bottom(&self, bottom: i32) {
        let Some(mut clipper) = self.clipper.borrow().clone() else {
            return;
        };
        if bottom < 0 {
            clipper.set_anchors_and_offsets_preset(LayoutPreset::FULL_RECT);
            return;
        }
        let window_width = DisplayServer::singleton().window_get_size().x;
        let scale = match self.host.get_viewport() {
            Some(v) if window_width > 0 => v.get_visible_rect().size.x / window_width as f32,
            _ => 1.0,
        };
        let width = self
            .host
            .get_viewport()
            .map_or(0.0, |v| v.get_visible_rect().size.x);
        clipper.set_anchors_preset(LayoutPreset::TOP_LEFT);
        clipper.set_position(Vector2::ZERO);
        clipper.set_size(Vector2::new(width, bottom as f32 * scale));
    }

    pub fn close(&self) {
        self.clip_bottom(-1);
        if let Some(preview) = self.preview() {
            self.show_preview(&preview, false);
        }
    }

    /// Renders the avatar the core computed. The core already holds it, so the engine reports back
    /// only when it ended up with something else (it forces defaults for missing categories).
    /// One edit at a time: builds overlapping on the same loader drop the wearables still
    /// downloading, so a burst of edits waits for the one in flight and only the last one runs.
    pub async fn apply_avatar(&self, args: &VarDictionary) {
        if self.applying.replace(true) {
            *self.pending_apply.borrow_mut() = Some(args.clone());
            return;
        }
        let mut next = Some(args.clone());
        while let Some(args) = next {
            self.apply_avatar_now(&args).await;
            next = self.pending_apply.borrow_mut().take();
        }
        self.applying.set(false);
    }

    async fn apply_avatar_now(&self, args: &VarDictionary) {
        let strings = |key: &str| {
            args.get(key)
                .and_then(|v| v.try_to::<PackedStringArray>().ok())
                .unwrap_or_default()
        };
        let text = |key: &str| args.get(key).map(|v| v.to_string()).unwrap_or_default();
        let color = |key: &str| Color::from_html(text(key).as_str()).unwrap_or(Color::WHITE);
        let wearables = strings("wearables");
        self.show_spinner();
        let fetches = {
            let base_url = globals::realm().bind().get_profile_content_url();
            globals::content_provider()
                .bind_mut()
                .fetch_wearables(wearables.to_var_array(), base_url)
        };
        await_all(fetches.iter_shared().collect()).await;
        self.freeze();
        // Written right before the build, with no wait in between: the saved profile replaces the
        // identity's avatar when it arrives, and a look written earlier would be lost to it.
        let mut avatar = globals::player_identity().bind().get_mutable_avatar();
        {
            let mut avatar = avatar.bind_mut();
            avatar.set_body_shape(text("body_shape").as_str().into());
            avatar.set_wearables(wearables.clone());
            avatar.set_emotes(strings("emotes"));
            avatar.set_skin_color(color("skin"));
            avatar.set_hair_color(color("hair"));
            avatar.set_eyes_color(color("eyes"));
        }
        self.refresh_avatar().await;
        next_frame().await;
        self.thaw();
        if !same_wearables(
            &string_list(&avatar.bind().get_wearables()),
            &string_list(&wearables),
        ) {
            self.emit_avatar();
        }
    }

    pub fn focus_camera(&self, target: &str) {
        let Some(mut preview) = self.preview() else {
            return;
        };
        let category = match target {
            "head" => "hair",
            "torso" => "upper_body",
            "legs" => "lower_body",
            "feet" => "feet",
            "hands" => "hands",
            _ => "",
        };
        preview.bind_mut().focus_camera_on(category.into(), false);
    }

    pub fn play_emote(&self, urn: &str) {
        let Some(preview) = self.preview() else {
            return;
        };
        let Some(avatar) = preview.bind().avatar.clone() else {
            return;
        };
        let id = if emotes::is_base_emote_urn(urn) {
            emotes::base_emote_id(urn).to_string()
        } else {
            urn.to_string()
        };
        crate::engine::local_task::spawn_local(async_play_emote_or_stash(avatar, id, -1, -1));
    }

    pub async fn set_profile_name(&self, name: &str) {
        let mut profile = globals::player_identity().bind().get_mutable_profile();
        profile.bind_mut().set_name(name.into());
        if self.preview().is_some() {
            self.refresh_avatar().await;
        }
    }

    fn show_preview(&self, preview: &Gd<AvatarPreview>, on: bool) {
        let mut preview = preview.clone();
        preview.set_visible(on);
        set_update_mode(
            &preview,
            if on {
                UpdateMode::WHEN_VISIBLE
            } else {
                UpdateMode::DISABLED
            },
        );
        let world = state::current();
        // While the preview shows, the avatar draws over the shell's content: the host puts the
        // Godot surface above the shell's view, and in explorer mode the root is transparent too.
        if let (Some(world), Some(mut viewport)) = (&world, self.host.get_viewport()) {
            viewport.set_transparent_background(on);
            viewport.set_disable_3d(on);
            let culling = viewport.get_canvas_cull_mask() == OVERLAY_CANVAS_LAYER;
            if on && !culling {
                self.canvas_cull_mask.set(viewport.get_canvas_cull_mask());
                viewport.set_canvas_cull_mask(OVERLAY_CANVAS_LAYER);
            } else if !on && culling {
                viewport.set_canvas_cull_mask(self.canvas_cull_mask.get());
            }
            world.ui_root.clone().set_visible(!on);
        }
    }

    /// Rebuilds the preview's avatar from the mutable profile; false when there is nothing to show.
    async fn refresh_avatar(&self) -> bool {
        let identity = globals::player_identity();
        let (mut profile, avatar) = {
            let identity = identity.bind();
            (
                identity.get_mutable_profile(),
                identity.get_mutable_avatar(),
            )
        };
        let Some(preview) = self.preview() else {
            return false;
        };
        let Some(preview_avatar) = preview.bind().avatar.clone() else {
            return false;
        };
        profile.bind_mut().set_avatar(avatar);
        update_from_profile(preview_avatar, profile).await;
        true
    }

    /// The avatar is hidden while a new look assembles: a still of the old one stays up meanwhile.
    fn freeze(&self) {
        if let Some(mut old) = self.still.borrow_mut().take() {
            old.queue_free();
        }
        if !self.is_shown() {
            return;
        }
        let Some(preview) = self.preview() else {
            return;
        };
        let Some(viewport) = preview.bind().subviewport.clone() else {
            return;
        };
        let Some(image) = viewport.get_texture().and_then(|t| t.get_image()) else {
            return;
        };
        let Some(texture) = ImageTexture::create_from_image(&image) else {
            return;
        };
        let mut still = TextureRect::new_alloc();
        still.set_texture(&texture);
        still.set_expand_mode(ExpandMode::IGNORE_SIZE);
        still.set_stretch_mode(StretchMode::SCALE);
        still.set_anchors_preset(LayoutPreset::FULL_RECT);
        still.set_mouse_filter(MouseFilter::IGNORE);
        still.set_visibility_layer(OVERLAY_CANVAS_LAYER);
        preview.clone().add_child(&still);
        *self.still.borrow_mut() = Some(still);
    }

    /// A ring over the avatar from the edit until the new look is drawn.
    fn show_spinner(&self) {
        if !self.is_shown() || self.spinner.borrow().is_some() {
            return;
        }
        let Some(mut preview) = self.preview() else {
            return;
        };
        let mut spinner = PreviewSpinner::centred();
        spinner.set_visibility_layer(OVERLAY_CANVAS_LAYER);
        preview.add_child(&spinner);
        *self.spinner.borrow_mut() = Some(spinner);
    }

    fn thaw(&self) {
        if let Some(mut still) = self.still.borrow_mut().take() {
            if still.is_instance_valid() {
                still.queue_free();
            }
        }
        if let Some(mut spinner) = self.spinner.borrow_mut().take() {
            if spinner.is_instance_valid() {
                spinner.queue_free();
            }
        }
    }

    fn emit_avatar(&self) {
        let avatar = globals::player_identity().bind().get_mutable_avatar();
        let avatar = avatar.bind();
        let hex = |color: Color| format!("#{}", color.to_html_without_alpha());
        globals::engine().bind().report_avatar(
            avatar.get_body_shape(),
            avatar.get_wearables(),
            avatar.get_emotes(),
            hex(avatar.get_skin_color()).as_str().into(),
            hex(avatar.get_hair_color()).as_str().into(),
            hex(avatar.get_eyes_color()).as_str().into(),
        );
    }
}

fn set_update_mode(preview: &Gd<AvatarPreview>, mode: UpdateMode) {
    if let Some(mut viewport) = preview.bind().subviewport.clone() {
        viewport.set_update_mode(mode);
    }
}

fn string_list(array: &PackedStringArray) -> Vec<String> {
    array.as_slice().iter().map(|s| s.to_string()).collect()
}

/// Whether both lists hold the same wearables, in any order.
fn same_wearables(a: &[String], b: &[String]) -> bool {
    let (mut left, mut right) = (a.to_vec(), b.to_vec());
    left.sort();
    right.sort();
    left == right
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn wearable_lists_compare_without_order() {
        assert!(same_wearables(
            &list(&["hat", "shirt"]),
            &list(&["shirt", "hat"])
        ));
        assert!(!same_wearables(&list(&["hat", "shirt"]), &list(&["shirt"])));
    }
}
