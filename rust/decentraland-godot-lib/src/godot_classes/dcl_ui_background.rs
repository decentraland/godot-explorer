use godot::{
    engine::{Control, NinePatchRect},
    prelude::*,
};

use crate::dcl::components::{
    material::{DclSourceTex, DclTexture},
    proto_components::sdk::components::{BackgroundTextureMode, PbUiBackground},
};

#[derive(GodotClass)]
#[class(base=NinePatchRect)]
pub struct DclUiBackground {
    #[base]
    base: Base<NinePatchRect>,

    current_value: PbUiBackground,

    signal_content_connected: bool,
    waiting_hash: GodotString,
    texture_loaded: bool,
}

#[godot_api]
impl NodeVirtual for DclUiBackground {
    fn init(base: Base<NinePatchRect>) -> Self {
        Self {
            base,
            current_value: PbUiBackground::default(),
            signal_content_connected: false,
            waiting_hash: GodotString::default(),
            texture_loaded: false,
        }
    }

    fn ready(&mut self) {
        let mut parent = self
            .base
            .get_parent()
            .expect("ui_background suppose to have a parent");
        let callable = self.base.get("_on_parent_size".into()).to::<Callable>();
        parent.connect("resized".into(), callable);

        self._set_white_pixel();
    }
}

#[godot_api]
impl DclUiBackground {
    fn update_layout_for_center(&mut self) -> Option<()> {
        tracing::debug!("update_layout_for_center");

        let parent_size = self
            .base
            .get_parent()
            .expect("ui_background suppose to have a parent")
            .cast::<Control>()
            .get_size();
        let texture_size = self.base.get_texture()?.get_size();
        let size = Vector2 {
            x: f32::min(parent_size.x, texture_size.x),
            y: f32::min(parent_size.y, texture_size.y),
        };
        let diff = texture_size - size;

        self.base.set_region_rect(Rect2 {
            position: diff / 2.0,
            size,
        });
        self.base.set_size(size);
        self.base.set_position((parent_size / 2.0) - (size / 2.0));

        self.base.set_h_axis_stretch_mode(
            godot::engine::nine_patch_rect::AxisStretchMode::AXIS_STRETCH_MODE_STRETCH,
        );
        self.base.set_v_axis_stretch_mode(
            godot::engine::nine_patch_rect::AxisStretchMode::AXIS_STRETCH_MODE_STRETCH,
        );
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
    fn _on_texture_loaded(&mut self) {
        self.set_content_connect_signal(false);

        let mut content_manager = self
            .base
            .get_node("/root/content_manager".into())
            .unwrap()
            .clone();

        let resource = content_manager.call(
            "get_resource_from_hash".into(),
            &[self.waiting_hash.to_variant()],
        );

        if resource.is_nil() {
            return;
        }
        let Ok(godot_texture) = resource.try_to::<Gd<godot::engine::ImageTexture>>() else {
            return;
        };

        self.texture_loaded = true;
        self.base.set_texture(godot_texture.clone().upcast());

        match self.current_value.texture_mode() {
            BackgroundTextureMode::NineSlices => {
                self.base
                    .set_anchors_preset(godot::engine::control::LayoutPreset::PRESET_FULL_RECT);

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

                self.base.set_patch_margin(
                    godot::engine::global::Side::SIDE_BOTTOM,
                    patch_margin_bottom as i32,
                );
                self.base.set_patch_margin(
                    godot::engine::global::Side::SIDE_LEFT,
                    patch_margin_left as i32,
                );
                self.base.set_patch_margin(
                    godot::engine::global::Side::SIDE_TOP,
                    patch_margin_top as i32,
                );
                self.base.set_patch_margin(
                    godot::engine::global::Side::SIDE_RIGHT,
                    patch_margin_right as i32,
                );

                // TODO: should be AXIS_STRETCH_MODE_TILE or AXIS_STRETCH_MODE_STRETCH?
                self.base.set_h_axis_stretch_mode(
                    godot::engine::nine_patch_rect::AxisStretchMode::AXIS_STRETCH_MODE_TILE_FIT,
                );
                self.base.set_v_axis_stretch_mode(
                    godot::engine::nine_patch_rect::AxisStretchMode::AXIS_STRETCH_MODE_TILE_FIT,
                );
            }
            BackgroundTextureMode::Center => {
                self.update_layout_for_center();
            }
            BackgroundTextureMode::Stretch => {
                self.base
                    .set_anchors_preset(godot::engine::control::LayoutPreset::PRESET_FULL_RECT);
                self.base
                    .set_patch_margin(godot::engine::global::Side::SIDE_BOTTOM, 0);
                self.base
                    .set_patch_margin(godot::engine::global::Side::SIDE_LEFT, 0);
                self.base
                    .set_patch_margin(godot::engine::global::Side::SIDE_TOP, 0);
                self.base
                    .set_patch_margin(godot::engine::global::Side::SIDE_RIGHT, 0);
                self.base.set_h_axis_stretch_mode(
                    godot::engine::nine_patch_rect::AxisStretchMode::AXIS_STRETCH_MODE_STRETCH,
                );
                self.base.set_v_axis_stretch_mode(
                    godot::engine::nine_patch_rect::AxisStretchMode::AXIS_STRETCH_MODE_STRETCH,
                );

                // TODO: BackgroundTextureMode::Stretch

                // let image_size = godot_texture.get_size();
                // if self.current_value.uvs.len() == 8 {
                //     /// default=\[0,0,0,1,1,0,1,0\]: starting from bottom-left vertex clock-wise
                //     let uvs = self.current_value.uvs.clone().as_slice();
                //     let [x1, y1, x2, y2, x3, y3, x4, y4] = [uvs[0], uvs[1], uvs[2], uvs[3], uvs[4], uvs[5], uvs[6], uvs[7]];

                //     let mut rect = Rect2 {
                //         position: Vector2 { x: 0.0, y: 0.0 },
                //         size: Vector2 { x: 0.0, y: 0.0 },
                //     };
                // }
            }
        }
    }

    #[func]
    fn _on_content_loading_finished(&mut self, file_hash: GodotString) {
        if file_hash != self.waiting_hash {
            return;
        }

        self._on_texture_loaded();
        self.set_content_connect_signal(false);
    }

    fn set_content_connect_signal(&mut self, should_be_connected: bool) {
        if self.signal_content_connected == should_be_connected {
            return;
        }

        let mut content_manager = self
            .base
            .get_node("/root/content_manager".into())
            .unwrap()
            .clone();

        let callable = self
            .base
            .get("_on_content_loading_finished".into())
            .to::<Callable>();
        if should_be_connected {
            content_manager.connect("content_loading_finished".into(), callable);
        } else {
            content_manager.disconnect("content_loading_finished".into(), callable);
        }

        self.signal_content_connected = should_be_connected;
    }

    fn _set_white_pixel(&mut self) {
        self.texture_loaded = false;
        self.base.set_texture(load("res://assets/white_pixel.png"));
        self.base.set_region_rect(Rect2 {
            position: Vector2 { x: 0.0, y: 0.0 },
            size: Vector2 { x: 0.0, y: 0.0 },
        });
    }

    pub fn change_value(
        &mut self,
        new_value: PbUiBackground,
        content_mapping: &godot::prelude::Dictionary,
    ) {
        let texture_changed = new_value.texture != self.current_value.texture;
        self.current_value = new_value;

        // texture change if
        if texture_changed {
            let content_mapping_files = content_mapping.get("content").unwrap().to::<Dictionary>();

            let texture = DclTexture::from_proto_with_hash(
                &self.current_value.texture,
                &content_mapping_files,
            );

            if let Some(texture) = texture {
                match &texture.source {
                    DclSourceTex::Texture(texture_hash) => {
                        let mut content_manager = self
                            .base
                            .get_node("/root/content_manager".into())
                            .unwrap()
                            .clone();

                        let mut promise = content_manager
                            .call(
                                "fetch_texture_by_hash".into(),
                                &[
                                    GodotString::from(texture_hash).to_variant(),
                                    content_mapping.to_variant(),
                                ],
                            )
                            .to::<Gd<RefCounted>>();

                        self.waiting_hash = GodotString::from(texture_hash);

                        let fetching_resource = promise.call("is_resolved".into(), &[]).to::<bool>();
                        if fetching_resource {
                            self.set_content_connect_signal(true);
                        } else {
                            self.base.call_deferred("_on_texture_loaded".into(), &[]);
                        }
                    }
                    DclSourceTex::VideoTexture(_) => {
                        // TODO: implement video texture
                    }
                    DclSourceTex::AvatarTexture(_) => {
                        // TODO: implement avatar texture
                    }
                }
            } else {
                self.base.set_texture(load("res://assets/white_pixel.png"));
            }

            if self.current_value.texture.is_none() {
                self.base
                    .set_anchors_preset(godot::engine::control::LayoutPreset::PRESET_FULL_RECT);
                self.base
                    .set_patch_margin(godot::engine::global::Side::SIDE_BOTTOM, 0);
                self.base
                    .set_patch_margin(godot::engine::global::Side::SIDE_LEFT, 0);
                self.base
                    .set_patch_margin(godot::engine::global::Side::SIDE_TOP, 0);
                self.base
                    .set_patch_margin(godot::engine::global::Side::SIDE_RIGHT, 0);
                self.base.set_h_axis_stretch_mode(
                    godot::engine::nine_patch_rect::AxisStretchMode::AXIS_STRETCH_MODE_STRETCH,
                );
                self.base.set_v_axis_stretch_mode(
                    godot::engine::nine_patch_rect::AxisStretchMode::AXIS_STRETCH_MODE_STRETCH,
                );
            }
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

        self.base.set_modulate(modulate_color);
    }
}
