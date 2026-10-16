//! One slot of the emote wheel: the emote's thumbnail over a rarity-tinted disc, outlined by state.

use godot::classes::{ITextureButton, Texture2D, TextureButton, TextureRect};
use godot::prelude::*;

use crate::avatars::emotes;
use crate::avatars::item::DclItemEntityDefinition;
use crate::avatars::loaders::fetch_emote_definition;
use crate::content::texture::TextureEntry;
use crate::engine::local_task::{await_promise, spawn_local};
use crate::fetcher::globals;
use crate::godot_classes::promise::PromiseError;

/// The empty-slot disc tint: design spec #24033B at 50%.
const EMPTY_SLOT_INNER_COLOR: Color = Color::from_rgba(0.141, 0.012, 0.231, 0.502);

/// The disc tint for a rarity, as 0-255 channels.
pub fn rarity_rgb(rarity: &str) -> (u8, u8, u8) {
    match rarity {
        "common" => (115, 211, 211),
        "uncommon" => (255, 131, 98),
        "rare" => (52, 206, 118),
        "epic" => (67, 143, 255),
        "legendary" => (161, 75, 243),
        "mythic" => (255, 75, 237),
        "unique" | "exotic" => (254, 162, 23),
        _ => (160, 155, 168),
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum OutlineState {
    Selected,
    Empty,
    Normal,
}

/// Selected wins over empty, which wins over normal.
pub fn outline_state(selected: bool, empty: bool) -> OutlineState {
    if selected {
        OutlineState::Selected
    } else if empty {
        OutlineState::Empty
    } else {
        OutlineState::Normal
    }
}

#[derive(GodotClass)]
#[class(base=TextureButton, init)]
pub struct EmoteItemUi {
    base: Base<TextureButton>,
    /// The default emotes are not URNs.
    #[var]
    #[init(val = "wave".into())]
    pub emote_urn: GString,
    #[var]
    #[init(val = "wave".into())]
    pub emote_name: GString,
    #[export]
    #[init(val = Color::from_html("d3d3d3").unwrap_or(Color::WHITE))]
    outline_color_normal: Color,
    #[export]
    #[init(val = Color::from_html("800000").unwrap_or(Color::WHITE))]
    outline_color_pressed: Color,
    #[export]
    #[init(val = Color::from_html("ae8ad2").unwrap_or(Color::WHITE))]
    outline_color_empty: Color,
    rarity: String,
    picture: Option<Gd<Texture2D>>,
    inside: bool,
    is_empty: bool,
    is_selected: bool,
    dirty: bool,
    #[init(node = "%TextureRect_Picture")]
    picture_rect: OnReady<Gd<TextureRect>>,
    #[init(node = "%TextureRect_Add")]
    add_rect: OnReady<Gd<TextureRect>>,
    #[init(node = "%Outline")]
    outline_rect: OnReady<Gd<TextureRect>>,
    #[init(node = "%Inner")]
    inner_rect: OnReady<Gd<TextureRect>>,
    #[init(node = "%Glow")]
    glow_rect: OnReady<Gd<TextureRect>>,
}

#[godot_api]
impl ITextureButton for EmoteItemUi {
    fn ready(&mut self) {
        self.rarity = "common".to_string();
        self.base_mut()
            .set_meta("attenuated_sound", &true.to_variant());
        let me = self.to_gd();
        self.base()
            .signals()
            .mouse_entered()
            .connect_other(&me, |this: &mut Self| this.on_hover(true));
        self.base()
            .signals()
            .mouse_exited()
            .connect_other(&me, |this: &mut Self| this.on_hover(false));
        self.base()
            .signals()
            .pressed()
            .connect_other(&me, |this: &mut Self| this.on_pressed());
        self.base()
            .signals()
            .button_down()
            .connect_other(&me, |this: &mut Self| {
                if !this.base().is_toggle_mode() {
                    this.set_selected(true);
                }
            });
        self.base()
            .signals()
            .button_up()
            .connect_other(&me, |this: &mut Self| {
                if !this.base().is_toggle_mode() {
                    let inside = this.inside;
                    this.set_selected(inside);
                }
            });
        self.base()
            .signals()
            .toggled()
            .connect_other(&me, |this: &mut Self, on| this.set_selected(on));
        // The thumbnail stays upright whatever the wheel rotates the slot by.
        self.base()
            .signals()
            .item_rect_changed()
            .connect_other(&me, |this: &mut Self| this.keep_upright());
        crate::ui::ui_sounds::install_audio_recursive(&me.upcast::<Node>());
        self.refresh_look();
    }

    fn process(&mut self, _delta: f64) {
        if self.dirty {
            self.dirty = false;
            self.refresh_look();
        }
    }
}

#[godot_api]
impl EmoteItemUi {
    #[signal]
    pub fn play_emote(emote_urn: GString);

    #[signal]
    pub fn select_emote(selected: bool, emote_urn: GString);

    /// Loads an emote by id or URN; short ids are base emotes, fetched remotely.
    #[func]
    pub fn async_load_from_urn(&mut self, emote_urn: GString, _index: i32) {
        self.is_empty = false;
        self.emote_urn = emote_urn.clone();
        let item = self.to_gd();
        spawn_local(load_from_urn(item, emote_urn.to_string()));
    }

    #[func]
    pub fn set_empty(&mut self) {
        self.emote_urn = GString::new();
        self.emote_name = GString::new();
        self.picture = None;
        self.rarity = "common".to_string();
        self.is_empty = true;
        self.dirty = true;
    }
}

impl EmoteItemUi {
    fn keep_upright(&mut self) {
        let rotation = -self.base().get_rotation();
        self.picture_rect.set_rotation(rotation);
        self.add_rect.set_rotation(rotation);
    }

    fn on_hover(&mut self, inside: bool) {
        self.inside = inside;
        let selected = if inside {
            true
        } else {
            self.base().is_pressed()
        };
        self.set_selected(selected);
        let urn = self.emote_urn.clone();
        self.base_mut()
            .emit_signal("select_emote", &[inside.to_variant(), urn.to_variant()]);
    }

    fn on_pressed(&mut self) {
        let urn = self.emote_urn.clone();
        self.base_mut()
            .emit_signal("play_emote", &[urn.to_variant()]);
    }

    fn set_selected(&mut self, selected: bool) {
        self.is_selected = selected;
        self.update_outline();
    }

    fn update_outline(&mut self) {
        let color = match outline_state(self.is_selected, self.is_empty) {
            OutlineState::Selected => self.outline_color_pressed,
            OutlineState::Empty => self.outline_color_empty,
            OutlineState::Normal => self.outline_color_normal,
        };
        self.outline_rect.set_self_modulate(color);
    }

    fn refresh_look(&mut self) {
        let tint = if self.is_empty {
            EMPTY_SLOT_INNER_COLOR
        } else {
            let (r, g, b) = rarity_rgb(&self.rarity);
            Color::from_rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
        };
        self.inner_rect.set_self_modulate(tint);
        self.glow_rect
            .set_visible(!self.is_empty && self.rarity != "common");
        self.picture_rect.set_texture(self.picture.as_ref());
        // An empty slot shows the "+" add icon instead of a thumbnail.
        self.add_rect.set_visible(self.is_empty);
        self.picture_rect.set_visible(!self.is_empty);
        self.update_outline();
    }
}

async fn load_from_urn(item: Gd<EmoteItemUi>, urn: String) {
    let fetch_urn = if urn.starts_with("urn") {
        urn
    } else if emotes::is_emote_default(&urn) {
        emotes::base_emote_urn(&urn)
    } else {
        godot_error!("Unknown emote ID: {urn}");
        return;
    };
    fetch_emote_definition(&fetch_urn).await;
    let data = globals::content_provider()
        .bind_mut()
        .get_wearable(fetch_urn.as_str().into());
    let Some(data) = data else {
        godot_error!("Failed to get emote data: {fetch_urn}");
        return;
    };
    load_from_entity(item, data).await;
}

async fn load_from_entity(mut item: Gd<EmoteItemUi>, data: Gd<DclItemEntityDefinition>) {
    if !item.is_instance_valid() {
        return;
    }
    let (name, rarity, thumbnail, mapping) = {
        let data = data.bind();
        (
            data.get_display_name(),
            data.get_rarity().to_string(),
            data.get_thumbnail(),
            data.get_content_mapping(),
        )
    };
    {
        let mut item = item.bind_mut();
        item.emote_name = name;
        item.rarity = rarity;
        item.dirty = true;
    }
    let promise = globals::content_provider()
        .bind_mut()
        .fetch_texture(thumbnail.clone(), mapping);
    let result = await_promise(promise).await;
    if !item.is_instance_valid() {
        return;
    }
    if let Ok(error) = result.try_to::<Gd<PromiseError>>() {
        godot_error!(
            "Fetch texture error on {thumbnail}: {}",
            error.bind().get_error()
        );
        return;
    }
    if let Ok(entry) = result.try_to::<Gd<TextureEntry>>() {
        let mut item = item.bind_mut();
        item.picture = Some(entry.bind().texture.clone());
        item.dirty = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rarities_map_to_distinct_tints() {
        assert_eq!(rarity_rgb("common"), (115, 211, 211));
        assert_eq!(rarity_rgb("unique"), rarity_rgb("exotic"));
        assert_eq!(rarity_rgb("anything else"), (160, 155, 168));
    }

    #[test]
    fn selection_beats_empty_beats_normal() {
        assert_eq!(outline_state(true, true), OutlineState::Selected);
        assert_eq!(outline_state(false, true), OutlineState::Empty);
        assert_eq!(outline_state(false, false), OutlineState::Normal);
    }
}
