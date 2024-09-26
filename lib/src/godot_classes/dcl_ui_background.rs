use godot::{
    engine::{Control, INinePatchRect, NinePatchRect},
    prelude::*,
};

use crate::{
    content::content_mapping::{ContentMappingAndUrlRef, DclContentMappingAndUrl},
    dcl::components::{
        material::{DclSourceTex, DclTexture},
        proto_components::sdk::components::{BackgroundTextureMode, PbUiBackground},
    },
};

use super::dcl_global::DclGlobal;

#[derive(GodotClass)]
#[class(base=NinePatchRect)]
pub struct DclUiBackground {
    base: Base<NinePatchRect>,

    current_value: PbUiBackground,

    waiting_hash: GString,
    texture_loaded: bool,
    first_texture_load_shot: bool,
}

#[godot_api]
impl INinePatchRect for DclUiBackground {
    fn init(base: Base<NinePatchRect>) -> Self {
        Self {
            base,
            current_value: PbUiBackground::default(),
            waiting_hash: GString::default(),
            texture_loaded: false,
            first_texture_load_shot: false,
        }
    }

    fn ready(&mut self) {
        let mut parent = self
            .base()
            .get_parent()
            .expect("ui_background suppose to have a parent");
        parent.connect("resized".into(), self.base().callable("_on_parent_size"));

        self._set_white_pixel();
    }
}

#[godot_api]
impl DclUiBackground {
    fn update_layout_for_center(&mut self) -> Option<()> {
        tracing::debug!("update_layout_for_center");

        let parent_size = self
            .base()
            .get_parent()
            .expect("ui_background suppose to have a parent")
            .cast::<Control>()
            .get_size();
        let texture_size = self.base().get_texture()?.get_size();
        let size = Vector2 {
            x: f32::min(parent_size.x, texture_size.x),
            y: f32::min(parent_size.y, texture_size.y),
        };
        let diff = texture_size - size;

        self.base_mut().set_region_rect(Rect2 {
            position: diff / 2.0,
            size,
        });
        self.base_mut().set_size(size);
        self.base_mut()
            .set_position((parent_size / 2.0) - (size / 2.0));

        self.base_mut()
            .set_h_axis_stretch_mode(godot::engine::nine_patch_rect::AxisStretchMode::STRETCH);
        self.base_mut()
            .set_v_axis_stretch_mode(godot::engine::nine_patch_rect::AxisStretchMode::STRETCH);
        Some(())
    }

    #[func]
    fn _on_parent_size(&mut self) {
        if !self.texture_loaded {
            return;
        }

        if let BackgroundTextureMode::Center = self.current_value.texture_mode() {
            self.update_layout_for_center();
        }
    }

    #[func]
    fn _on_profile_for_texture_loaded(&mut self) {
        let current_user_id = match self.current_value.texture.as_ref() {
            Some(texture) => match texture.tex.as_ref() {
                Some(crate::dcl::components::proto_components::common::texture_union::Tex::AvatarTexture(user_id)) => user_id.user_id.as_str(),
                _ => return,
            },
            None => return,
        };

        let mut content_provider = DclGlobal::singleton().bind().get_content_provider();
        let Some(profile) = content_provider
            .bind_mut()
            .get_profile(GString::from(current_user_id))
        else {
            return;
        };

        let binded_profile = profile.bind();
        let Some(snapshots) = binded_profile.inner.content.avatar.snapshots.as_ref() else {
            return;
        };
        let face256_url = format!("{}{}", binded_profile.inner.base_url, snapshots.face256);

        let mut promise = content_provider.bind_mut().fetch_texture_by_url(
            GString::from(snapshots.face256.as_str()),
            face256_url.into(),
        );

        self.waiting_hash = GString::from(snapshots.face256.as_str());

        if !promise.bind().is_resolved() {
            promise.connect(
                "on_resolved".into(),
                self.base().callable("_on_texture_loaded"),
            );
        }

        self.first_texture_load_shot = true;
        self.base_mut()
            .call_deferred("_on_texture_loaded".into(), &[]);
    }

    #[func]
    fn _on_texture_loaded(&mut self) {
        let global = DclGlobal::singleton();
        let mut content_provider = global.bind().get_content_provider();
        let Some(godot_texture) = content_provider
            .bind_mut()
            .get_texture_from_hash(self.waiting_hash.clone())
        else {
            if self.first_texture_load_shot {
                self.first_texture_load_shot = false;
            } else {
                tracing::error!("trying to set texture not found: {}", self.waiting_hash);
            }
            return;
        };
        self.texture_loaded = true;
        self.base_mut().set_texture(godot_texture.clone().upcast());

        self._set_texture_params();
    }

    fn _set_texture_params(&mut self) {
        let Some(godot_texture) = self.base().get_texture() else {
            return;
        };
        match self.current_value.texture_mode() {
            BackgroundTextureMode::NineSlices => {
                self.base_mut()
                    .set_anchors_preset(godot::engine::control::LayoutPreset::FULL_RECT);

                let texture_size = godot_texture.get_size();
                let (patch_margin_left, patch_margin_top, patch_margin_right, patch_margin_bottom) =
                    if let Some(slices) = self.current_value.texture_slices.as_ref() {
                        (
                            slices.left * texture_size.x,
                            slices.top * texture_size.y,
                            slices.right * texture_size.x,
                            slices.bottom * texture_size.y,
                        )
                    } else {
                        (
                            texture_size.x / 3.0,
                            texture_size.y / 3.0,
                            texture_size.x / 3.0,
                            texture_size.y / 3.0,
                        )
                    };

                self.base_mut().set_patch_margin(
                    godot::engine::global::Side::BOTTOM,
                    patch_margin_bottom as i32,
                );
                self.base_mut()
                    .set_patch_margin(godot::engine::global::Side::LEFT, patch_margin_left as i32);
                self.base_mut()
                    .set_patch_margin(godot::engine::global::Side::TOP, patch_margin_top as i32);
                self.base_mut().set_patch_margin(
                    godot::engine::global::Side::RIGHT,
                    patch_margin_right as i32,
                );

                // TODO: should be TILE or STRETCH?
                self.base_mut().set_h_axis_stretch_mode(
                    godot::engine::nine_patch_rect::AxisStretchMode::TILE_FIT,
                );
                self.base_mut().set_v_axis_stretch_mode(
                    godot::engine::nine_patch_rect::AxisStretchMode::TILE_FIT,
                );
            }
            BackgroundTextureMode::Center => {
                self.update_layout_for_center();
            }
            BackgroundTextureMode::Stretch => {
                self.base_mut()
                    .set_anchors_preset(godot::engine::control::LayoutPreset::FULL_RECT);
                self.base_mut()
                    .set_patch_margin(godot::engine::global::Side::BOTTOM, 0);
                self.base_mut()
                    .set_patch_margin(godot::engine::global::Side::LEFT, 0);
                self.base_mut()
                    .set_patch_margin(godot::engine::global::Side::TOP, 0);
                self.base_mut()
                    .set_patch_margin(godot::engine::global::Side::RIGHT, 0);
                self.base_mut().set_h_axis_stretch_mode(
                    godot::engine::nine_patch_rect::AxisStretchMode::STRETCH,
                );
                self.base_mut().set_v_axis_stretch_mode(
                    godot::engine::nine_patch_rect::AxisStretchMode::STRETCH,
                );

                if self.current_value.uvs.len() == 8 {
                    let uvs = self.current_value.uvs.as_slice();
                    let image_size = godot_texture.get_size();

                    // default=\[0,0,0,1,1,0,1,0\]: starting from bottom-left vertex clock-wise
                    let sx = uvs[0].min(uvs[4]).clamp(0.0, 1.0);
                    let sw = uvs[0].max(uvs[4]).clamp(0.0, 1.0);

                    let sy = (1.0 - uvs[3].min(uvs[1])).clamp(0.0, 1.0);
                    let sh = (1.0 - uvs[3].max(uvs[1])).clamp(0.0, 1.0);

                    let sx = sx * image_size.x;
                    let sw = sw * image_size.x - sx;
                    let sy = sy * image_size.y;
                    let sh = sh * image_size.y - sy;

                    self.base_mut().set_region_rect(Rect2 {
                        position: Vector2 { x: sx, y: sy },
                        size: Vector2 { x: sw, y: sh },
                    });
                }
            }
        }
    }

    fn _set_white_pixel(&mut self) {
        self.texture_loaded = false;
        self.base_mut()
            .set_texture(load("res://assets/white_pixel.png"));
        self.base_mut().set_region_rect(Rect2 {
            position: Vector2 { x: 0.0, y: 0.0 },
            size: Vector2 { x: 0.0, y: 0.0 },
        });
    }

    pub fn change_value(
        &mut self,
        new_value: PbUiBackground,
        content_mapping: ContentMappingAndUrlRef,
    ) {
        let texture_changed = new_value.texture != self.current_value.texture;
        self.current_value = new_value;

        // texture change if
        if texture_changed {
            self.texture_loaded = false;

            let texture =
                DclTexture::from_proto_with_hash(&self.current_value.texture, &content_mapping);

            if let Some(texture) = texture {
                match &texture.source {
                    DclSourceTex::Texture(texture_hash) => {
                        let global = DclGlobal::singleton();
                        let mut content_provider = global.bind().get_content_provider();
                        let mut promise = content_provider.bind_mut().fetch_texture_by_hash(
                            GString::from(texture_hash),
                            DclContentMappingAndUrl::from_ref(content_mapping),
                        );

                        self.waiting_hash = GString::from(texture_hash);

                        if !promise.bind().is_resolved() {
                            promise.connect(
                                "on_resolved".into(),
                                self.base().callable("_on_texture_loaded"),
                            );
                        }

                        self.first_texture_load_shot = true;
                        self.base_mut()
                            .call_deferred("_on_texture_loaded".into(), &[]);
                    }
                    DclSourceTex::VideoTexture(_) => {
                        // TODO: implement video texture
                    }
                    DclSourceTex::AvatarTexture(user_id) => {
                        let global = DclGlobal::singleton();
                        let mut content_provider = global.bind().get_content_provider();
                        let mut promise = content_provider
                            .bind_mut()
                            .fetch_profile(GString::from(user_id));

                        if !promise.bind().is_resolved() {
                            promise.connect(
                                "on_resolved".into(),
                                self.base().callable("_on_profile_for_texture_loaded"),
                            );
                        } else {
                            self.base_mut()
                                .call_deferred("_on_profile_for_texture_loaded".into(), &[]);
                        }
                    }
                }
            } else {
                self.base_mut()
                    .set_texture(load("res://assets/white_pixel.png"));
            }

            if self.current_value.texture.is_none() {
                let mut base_mut = self.base_mut();
                base_mut.set_anchors_preset(godot::engine::control::LayoutPreset::FULL_RECT);
                base_mut.set_patch_margin(godot::engine::global::Side::BOTTOM, 0);
                base_mut.set_patch_margin(godot::engine::global::Side::LEFT, 0);
                base_mut.set_patch_margin(godot::engine::global::Side::TOP, 0);
                base_mut.set_patch_margin(godot::engine::global::Side::RIGHT, 0);
                base_mut.set_h_axis_stretch_mode(
                    godot::engine::nine_patch_rect::AxisStretchMode::STRETCH,
                );
                base_mut.set_v_axis_stretch_mode(
                    godot::engine::nine_patch_rect::AxisStretchMode::STRETCH,
                );
            }
        } else {
            self._set_texture_params();
        }

        let modulate_color = self
            .current_value
            .color
            .as_ref()
            .map(|v| godot::prelude::Color {
                r: v.r,
                g: v.g,
                b: v.b,
                a: v.a,
            })
            .unwrap_or(godot::prelude::Color::WHITE);

        self.base_mut().set_modulate(modulate_color);
    }
}
