//! Owns the single avatar preview the shell's screens draw through a hole.

use std::cell::{Cell, RefCell};

use godot::classes::control::LayoutPreset;
use godot::classes::sub_viewport::UpdateMode;
use godot::classes::{CanvasLayer, DisplayServer, Node, PackedScene};
use godot::prelude::*;

use super::state;
use crate::avatars::avatar::build::update_from_profile;
use crate::avatars::avatar::emote_load::async_play_emote_or_stash;
use crate::avatars::emotes;
use crate::avatars::preview::AvatarPreview;
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
    preview: RefCell<Option<Gd<AvatarPreview>>>,
    canvas_cull_mask: Cell<u32>,
    preload_pending: Cell<bool>,
    preloaded_for: RefCell<String>,
}

impl Backpack {
    pub fn new(host: Gd<Node>) -> Self {
        Self {
            host,
            layer: RefCell::new(None),
            preview: RefCell::new(None),
            canvas_cull_mask: Cell::new(0),
            preload_pending: Cell::new(false),
            preloaded_for: RefCell::new(String::new()),
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
        }
        self.preload_pending.set(false);
    }

    pub async fn open(&self, rect: Rect2) {
        let preview = self.ensure_preview();
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
        }
        preview.set_stretch(true);
        layer.add_child(&preview);
        preview.set_visibility_layer(OVERLAY_CANVAS_LAYER);
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

    pub fn close(&self) {
        if let Some(preview) = self.preview() {
            self.show_preview(&preview, false);
        }
    }

    /// Renders the avatar the core computed. The core already holds it, so the engine reports back
    /// only when it ended up with something else (it forces defaults for missing categories).
    pub async fn apply_avatar(&self, args: &VarDictionary) {
        let identity = globals::player_identity();
        let mut avatar = identity.bind().get_mutable_avatar();
        let strings = |key: &str| {
            args.get(key)
                .and_then(|v| v.try_to::<PackedStringArray>().ok())
                .unwrap_or_default()
        };
        let text = |key: &str| args.get(key).map(|v| v.to_string()).unwrap_or_default();
        let color = |key: &str| Color::from_html(text(key).as_str()).unwrap_or(Color::WHITE);
        let wearables = strings("wearables");
        {
            let mut avatar = avatar.bind_mut();
            avatar.set_body_shape(text("body_shape").as_str().into());
            avatar.set_wearables(wearables.clone());
            avatar.set_emotes(strings("emotes"));
            avatar.set_skin_color(color("skin"));
            avatar.set_hair_color(color("hair"));
            avatar.set_eyes_color(color("eyes"));
        }
        let fetches = {
            let wearables = avatar.bind().get_wearables();
            let base_url = globals::realm().bind().get_profile_content_url();
            globals::content_provider()
                .bind_mut()
                .fetch_wearables(wearables.to_var_array(), base_url)
        };
        await_all(fetches.iter_shared().collect()).await;
        self.refresh_avatar().await;
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
