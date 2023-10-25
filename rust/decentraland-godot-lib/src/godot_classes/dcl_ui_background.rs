use godot::{engine::NinePatchRect, prelude::*};

use crate::dcl::components::{
    material::{DclSourceTex, DclTexture},
    proto_components::sdk::components::PbUiBackground,
};

#[derive(GodotClass)]
#[class(base=NinePatchRect)]
pub struct DclUiBackground {
    #[base]
    base: Base<NinePatchRect>,

    current_value: PbUiBackground,
    current_texture: DclTexture,

    signal_content_connected: bool,
    waiting_hash: GodotString,
}

#[godot_api]
impl NodeVirtual for DclUiBackground {
    fn init(base: Base<NinePatchRect>) -> Self {
        Self {
            base,
            current_value: PbUiBackground::default(),
            current_texture: DclTexture::default(),
            signal_content_connected: false,
            waiting_hash: GodotString::default(),
        }
    }
}

#[godot_api]
impl DclUiBackground {
    #[func]
    fn _on_texture_loaded(&mut self) {
        self.set_content_connect_signal(false);
        // self.base.set_texture(texture);

        let mut content_manager = self
            .base
            .get_node("/root/content_manager".into())
            .unwrap()
            .clone();

        let resource = content_manager.call(
            "get_resource_from_hash".into(),
            &[self.waiting_hash.to_variant()],
        );

        if !resource.is_nil() {
            if let Ok(godot_texture) = resource.try_to::<Gd<godot::engine::ImageTexture>>() {
                self.base.set_texture(godot_texture.upcast());
                
            if self.current_value.texture.is_some() {

                match self.current_value.texture_mode() {
                    crate::dcl::components::proto_components::sdk::components::BackgroundTextureMode::NineSlices => {
                        self.base.set_anchors_preset(godot::engine::control::LayoutPreset::PRESET_FULL_RECT);

                        // TODO: define with self.current_value.texture_slices and size of texture
                        self.base
                            .set_patch_margin(godot::engine::global::Side::SIDE_BOTTOM, 0);
                        self.base
                            .set_patch_margin(godot::engine::global::Side::SIDE_LEFT, 0);
                        self.base
                            .set_patch_margin(godot::engine::global::Side::SIDE_TOP, 0);
                        self.base
                            .set_patch_margin(godot::engine::global::Side::SIDE_RIGHT, 0);        
                        
                        // TODO: should be AXIS_STRETCH_MODE_TILE_FIT or AXIS_STRETCH_MODE_STRETCH?
                        self.base.set_h_axis_stretch_mode(
                            godot::engine::nine_patch_rect::AxisStretchMode::AXIS_STRETCH_MODE_TILE,
                        );
                        self.base.set_v_axis_stretch_mode(
                            godot::engine::nine_patch_rect::AxisStretchMode::AXIS_STRETCH_MODE_TILE,
                        );

                    }
                    crate::dcl::components::proto_components::sdk::components::BackgroundTextureMode::Center => {
                        self.base.set_anchors_preset(godot::engine::control::LayoutPreset::PRESET_CENTER);
                        self.base.set_h_axis_stretch_mode(
                            godot::engine::nine_patch_rect::AxisStretchMode::AXIS_STRETCH_MODE_STRETCH,
                        );
                        self.base.set_v_axis_stretch_mode(
                            godot::engine::nine_patch_rect::AxisStretchMode::AXIS_STRETCH_MODE_STRETCH,
                        );
                    }
                    crate::dcl::components::proto_components::sdk::components::BackgroundTextureMode::Stretch => {
                        self.base.set_anchors_preset(godot::engine::control::LayoutPreset::PRESET_FULL_RECT);
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
            } 
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

    pub fn change_value(
        &mut self,
        new_value: PbUiBackground,
        content_mapping: &godot::prelude::Dictionary,
    ) {
        let texture_changed = new_value.texture != self.current_value.texture;
        self.current_value = new_value;
        // res://assets/white_pixel.png
        self.base.set_texture(load("res://assets/white_pixel.png"));

        // TODO: define in function of new_value.uvs (has to be 8)
        self.base.set_region_rect(Rect2 {
            position: Vector2 { x: 0.0, y: 0.0 },
            size: Vector2 { x: 0.0, y: 0.0 },
        });

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

                        let fetching_resource = content_manager
                            .call(
                                "fetch_texture_by_hash".into(),
                                &[
                                    GodotString::from(texture_hash).to_variant(),
                                    content_mapping.to_variant(),
                                ],
                            )
                            .to::<bool>();

                        if fetching_resource {
                            self.waiting_hash = GodotString::from(texture_hash);
                            self.set_content_connect_signal(true);
                        } else {
                            self.base.call_deferred("_on_texture_loaded".into(), &[]);
                        }
                    }
                    DclSourceTex::VideoTexture(_) => {
                        // self.base.set_texture(t);
                    }
                    DclSourceTex::AvatarTexture(_) => {
                        // self.base.set_texture(t);
                    }
                }
            } else {
                self.base.set_texture(load("res://assets/white_pixel.png"));
            }

            if !self.current_value.texture.is_some() {
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
