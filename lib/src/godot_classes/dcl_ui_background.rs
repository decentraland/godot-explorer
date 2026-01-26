use godot::{
    classes::{
        Control, INinePatchRect, Material, NinePatchRect, Node, ResourceLoader, Shader,
        ShaderMaterial, Texture2D, TextureRect,
    },
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

    // Child TextureRect with custom shader for proper UV interpolation
    uv_child: Option<Gd<TextureRect>>,
    uv_shader_material: Option<Gd<ShaderMaterial>>,
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
            uv_child: None,
            uv_shader_material: None,
        }
    }

    fn ready(&mut self) {
        let mut parent = self
            .base()
            .get_parent()
            .expect("ui_background suppose to have a parent");
        let parent_size = parent.clone().cast::<Control>().get_size();
        tracing::debug!(
            "[UI_BKG] ready: connecting to parent resize signal, parent_size={:?}",
            parent_size
        );
        parent.connect("resized", &self.base().callable("_on_parent_size"));

        self._set_white_pixel();
    }
}

#[godot_api]
impl DclUiBackground {
    fn update_layout_for_center(&mut self) -> Option<()> {
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
            .set_h_axis_stretch_mode(godot::classes::nine_patch_rect::AxisStretchMode::STRETCH);
        self.base_mut()
            .set_v_axis_stretch_mode(godot::classes::nine_patch_rect::AxisStretchMode::STRETCH);
        Some(())
    }

    #[func]
    fn _on_parent_size(&mut self) {
        let parent_size = self
            .base()
            .get_parent()
            .map(|p| p.cast::<Control>().get_size())
            .unwrap_or_default();
        let my_size = self.base().get_size();
        let my_pos = self.base().get_position();
        tracing::debug!(
            "[UI_BKG] _on_parent_size: texture_loaded={}, texture_mode={:?}, parent_size={:?}, my_size={:?}, my_pos={:?}",
            self.texture_loaded,
            self.current_value.texture_mode(),
            parent_size,
            my_size,
            my_pos
        );

        if !self.texture_loaded {
            tracing::debug!("[UI_BKG] _on_parent_size: texture_loaded=false, skipping");
            return;
        }

        if let BackgroundTextureMode::Center = self.current_value.texture_mode() {
            tracing::debug!(
                "[UI_BKG] _on_parent_size: CENTER mode, calling update_layout_for_center"
            );
            self.update_layout_for_center();
        } else {
            tracing::debug!(
                "[UI_BKG] _on_parent_size: mode={:?}, NOT Center, doing nothing",
                self.current_value.texture_mode()
            );
        }
    }

    /// Check if UVs are non-trivial (rotated, skewed, or non-rectangular)
    /// Default UVs are [0,0, 1,0, 1,1, 0,1] (bottom-left, bottom-right, top-right, top-left)
    fn has_custom_uvs(&self) -> bool {
        let uvs = &self.current_value.uvs;
        if uvs.len() != 8 {
            return false;
        }

        // Check if it's an axis-aligned rectangle
        // For axis-aligned: all X coords of left vertices should match, all X coords of right should match
        // and all Y coords of bottom should match, all Y coords of top should match
        let bl_x = uvs[0];
        let bl_y = uvs[1];
        let br_x = uvs[2];
        let br_y = uvs[3];
        let tr_x = uvs[4];
        let tr_y = uvs[5];
        let tl_x = uvs[6];
        let tl_y = uvs[7];

        const EPSILON: f32 = 0.0001;

        // For an axis-aligned rectangle:
        // - bl_x == tl_x (left edge is vertical)
        // - br_x == tr_x (right edge is vertical)
        // - bl_y == br_y (bottom edge is horizontal)
        // - tl_y == tr_y (top edge is horizontal)
        let is_axis_aligned = (bl_x - tl_x).abs() < EPSILON
            && (br_x - tr_x).abs() < EPSILON
            && (bl_y - br_y).abs() < EPSILON
            && (tl_y - tr_y).abs() < EPSILON;

        !is_axis_aligned
    }

    /// Create or update the UV child TextureRect with custom shader
    fn ensure_uv_child(&mut self, texture: &Gd<Texture2D>) {
        if self.uv_child.is_none() {
            // Load the shader
            let mut resource_loader = ResourceLoader::singleton();
            let shader_res = resource_loader
                .load("res://assets/shaders/dcl_ui_background_uv.gdshader")
                .map(|r| r.cast::<Shader>());

            let Some(shader) = shader_res else {
                tracing::error!("Failed to load dcl_ui_background_uv.gdshader");
                return;
            };

            // Create shader material
            let mut shader_material = ShaderMaterial::new_gd();
            shader_material.set_shader(&shader);
            self.uv_shader_material = Some(shader_material.clone());

            // Create TextureRect child
            let mut texture_rect = TextureRect::new_alloc();
            texture_rect.set_name("UVChild");
            texture_rect.set_anchors_preset(godot::classes::control::LayoutPreset::FULL_RECT);
            texture_rect.set_expand_mode(godot::classes::texture_rect::ExpandMode::IGNORE_SIZE);
            texture_rect.set_stretch_mode(godot::classes::texture_rect::StretchMode::SCALE);
            texture_rect.set_material(&shader_material.upcast::<Material>());
            texture_rect.set_mouse_filter(godot::classes::control::MouseFilter::IGNORE);

            self.base_mut()
                .add_child(&texture_rect.clone().upcast::<Node>());
            self.uv_child = Some(texture_rect);
        }

        // Apply texture and UV parameters
        self.apply_uv_shader_params(texture);

        // Show the UV child
        if let Some(child) = &mut self.uv_child {
            child.set_visible(true);
        }
    }

    /// Apply UV coordinates to the shader
    fn apply_uv_shader_params(&mut self, texture: &Gd<Texture2D>) {
        let Some(shader_mat) = &mut self.uv_shader_material else {
            return;
        };

        let uvs = &self.current_value.uvs;
        if uvs.len() != 8 {
            return;
        }

        // DCL format: [bl_x, bl_y, br_x, br_y, tr_x, tr_y, tl_x, tl_y]
        // Remap with 90° CW rotation to fix orientation
        // Also flip X to preserve animation direction (CW vs CCW)
        // DCL bl → shader top_left, DCL br → shader bottom_left,
        // DCL tr → shader bottom_right, DCL tl → shader top_right
        shader_mat.set_shader_parameter(
            "uv_top_left",
            &Vector2::new(1.0 - uvs[0], uvs[1]).to_variant(),
        );
        shader_mat.set_shader_parameter(
            "uv_bottom_left",
            &Vector2::new(1.0 - uvs[2], uvs[3]).to_variant(),
        );
        shader_mat.set_shader_parameter(
            "uv_bottom_right",
            &Vector2::new(1.0 - uvs[4], uvs[5]).to_variant(),
        );
        shader_mat.set_shader_parameter(
            "uv_top_right",
            &Vector2::new(1.0 - uvs[6], uvs[7]).to_variant(),
        );

        // Set texture on the UV child and also as shader parameter
        if let Some(child) = &mut self.uv_child {
            child.set_texture(texture);
        }

        // Also apply modulate color to the UV child
        if let Some(child) = &mut self.uv_child {
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
            child.set_modulate(modulate_color);
        }
    }

    /// Hide the UV child when not using custom UVs
    fn hide_uv_child(&mut self) {
        if let Some(child) = &mut self.uv_child {
            child.set_visible(false);
        }
    }

    #[func]
    fn _on_avatar_texture_loaded(&mut self) {
        // Extract user_id from waiting_hash (stored with "avatar:" prefix)
        let user_id = self.waiting_hash.to_string();
        let user_id = user_id.strip_prefix("avatar:").unwrap_or(&user_id);

        let global = DclGlobal::singleton();
        let mut content_provider = global.bind().get_content_provider();

        // Check if loading is complete (resolved or rejected)
        let is_loaded = content_provider
            .bind_mut()
            .is_avatar_texture_loaded(user_id.to_godot());

        let texture_result = content_provider
            .bind_mut()
            .get_avatar_texture(user_id.to_godot());

        let Some(godot_texture) = texture_result else {
            // If loading is complete (promise resolved or rejected) but no texture,
            // reset to white pixel immediately
            if is_loaded {
                tracing::error!(
                    "UI Avatar texture failed for user: {}, resetting to white pixel",
                    user_id
                );
                self._set_white_pixel();
            } else if self.first_texture_load_shot {
                // Still loading, wait for signal
                self.first_texture_load_shot = false;
            } else {
                tracing::error!(
                    "UI Avatar texture not found for user: {}, resetting to white pixel",
                    user_id
                );
                self._set_white_pixel();
            }
            return;
        };

        self.texture_loaded = true;
        self.base_mut()
            .set_texture(&godot_texture.clone().upcast::<Texture2D>());

        self._set_texture_params();
    }

    #[func]
    fn _on_texture_loaded(&mut self) {
        let parent_size = self
            .base()
            .get_parent()
            .map(|p| p.cast::<Control>().get_size())
            .unwrap_or_default();
        tracing::debug!(
            "[UI_BKG] _on_texture_loaded START: waiting_hash={}, parent_size={:?}",
            self.waiting_hash,
            parent_size
        );
        let global = DclGlobal::singleton();
        let mut content_provider = global.bind().get_content_provider();
        let Some(godot_texture) = content_provider
            .bind_mut()
            .get_texture_from_hash(self.waiting_hash.clone())
        else {
            if self.first_texture_load_shot {
                tracing::debug!("[UI_BKG] _on_texture_loaded: first shot, will retry");
                self.first_texture_load_shot = false;
            } else {
                tracing::error!("trying to set texture not found: {}", self.waiting_hash);
            }
            return;
        };
        tracing::debug!(
            "[UI_BKG] _on_texture_loaded: texture found, size={:?}",
            godot_texture.get_size()
        );
        self.texture_loaded = true;
        self.base_mut()
            .set_texture(&godot_texture.clone().upcast::<Texture2D>());

        self._set_texture_params();

        let final_size = self.base().get_size();
        let final_pos = self.base().get_position();
        tracing::debug!(
            "[UI_BKG] _on_texture_loaded END: final_size={:?}, final_pos={:?}",
            final_size,
            final_pos
        );
    }

    fn _set_texture_params(&mut self) {
        let Some(godot_texture) = self.base().get_texture() else {
            tracing::debug!("[UI_BKG] _set_texture_params: NO TEXTURE, returning early");
            return;
        };
        let parent_size = self
            .base()
            .get_parent()
            .map(|p| p.cast::<Control>().get_size())
            .unwrap_or_default();
        let my_size_before = self.base().get_size();

        // When there's no actual texture (just using white_pixel placeholder),
        // force STRETCH mode to fill the parent area. CENTER mode only makes sense
        // when there's a real image to center.
        let effective_texture_mode = if self.current_value.texture.is_none() {
            BackgroundTextureMode::Stretch
        } else {
            self.current_value.texture_mode()
        };

        tracing::debug!(
            "[UI_BKG] _set_texture_params: requested_mode={:?}, effective_mode={:?}, has_texture={}, parent_size={:?}, my_size_before={:?}, texture_size={:?}",
            self.current_value.texture_mode(),
            effective_texture_mode,
            self.current_value.texture.is_some(),
            parent_size,
            my_size_before,
            godot_texture.get_size()
        );

        match effective_texture_mode {
            BackgroundTextureMode::NineSlices => {
                self.hide_uv_child();
                self.base_mut().set_self_modulate(Color::WHITE);
                self.base_mut()
                    .set_anchors_preset(godot::classes::control::LayoutPreset::FULL_RECT);

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

                self.base_mut()
                    .set_patch_margin(godot::builtin::Side::BOTTOM, patch_margin_bottom as i32);
                self.base_mut()
                    .set_patch_margin(godot::builtin::Side::LEFT, patch_margin_left as i32);
                self.base_mut()
                    .set_patch_margin(godot::builtin::Side::TOP, patch_margin_top as i32);
                self.base_mut()
                    .set_patch_margin(godot::builtin::Side::RIGHT, patch_margin_right as i32);

                // TODO: should be TILE or STRETCH?
                self.base_mut().set_h_axis_stretch_mode(
                    godot::classes::nine_patch_rect::AxisStretchMode::TILE_FIT,
                );
                self.base_mut().set_v_axis_stretch_mode(
                    godot::classes::nine_patch_rect::AxisStretchMode::TILE_FIT,
                );

                let slices_info = if self.current_value.texture_slices.is_some() {
                    format!("{:?}", self.current_value.texture_slices)
                } else {
                    "default (1/3)".to_string()
                };
                tracing::debug!(
                    "[UI_BKG] _set_texture_params NINE_SLICES: margins L={}, T={}, R={}, B={}, slices={}",
                    patch_margin_left, patch_margin_top, patch_margin_right, patch_margin_bottom,
                    slices_info
                );
            }
            BackgroundTextureMode::Center => {
                self.hide_uv_child();
                self.base_mut().set_self_modulate(Color::WHITE);
                self.update_layout_for_center();
            }
            BackgroundTextureMode::Stretch => {
                self.base_mut()
                    .set_anchors_preset(godot::classes::control::LayoutPreset::FULL_RECT);
                self.base_mut()
                    .set_patch_margin(godot::builtin::Side::BOTTOM, 0);
                self.base_mut()
                    .set_patch_margin(godot::builtin::Side::LEFT, 0);
                self.base_mut()
                    .set_patch_margin(godot::builtin::Side::TOP, 0);
                self.base_mut()
                    .set_patch_margin(godot::builtin::Side::RIGHT, 0);
                self.base_mut().set_h_axis_stretch_mode(
                    godot::classes::nine_patch_rect::AxisStretchMode::STRETCH,
                );
                self.base_mut().set_v_axis_stretch_mode(
                    godot::classes::nine_patch_rect::AxisStretchMode::STRETCH,
                );

                if self.current_value.uvs.len() == 8 {
                    if self.has_custom_uvs() {
                        // Use UV child with shader for non-axis-aligned UVs (rotations, skews, etc.)
                        self.ensure_uv_child(&godot_texture);
                        // Hide the base texture by making it transparent
                        self.base_mut()
                            .set_self_modulate(Color::from_rgba(1.0, 1.0, 1.0, 0.0));
                    } else {
                        // Use existing simple region rect approach for axis-aligned UVs
                        self.hide_uv_child();
                        self.base_mut().set_self_modulate(Color::WHITE);

                        let uvs = self.current_value.uvs.as_slice();
                        let image_size = godot_texture.get_size();

                        // default=[0,0,0,1,1,0,1,0]: starting from bottom-left vertex clock-wise
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
                } else {
                    // No UVs specified, hide UV child if it exists
                    self.hide_uv_child();
                    self.base_mut().set_self_modulate(Color::WHITE);
                }
            }
        }
    }

    fn _set_white_pixel(&mut self) {
        self.texture_loaded = false;
        self.hide_uv_child();
        self.base_mut().set_self_modulate(Color::WHITE);
        self.base_mut()
            .set_texture(&load::<Texture2D>("res://assets/white_pixel.png"));
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
        let parent_size = self
            .base()
            .get_parent()
            .map(|p| p.cast::<Control>().get_size())
            .unwrap_or_default();
        let my_size = self.base().get_size();
        let my_pos = self.base().get_position();
        let texture_changed = new_value.texture != self.current_value.texture;
        tracing::debug!(
            "[UI_BKG] change_value START: texture_changed={}, texture_mode={:?}, parent_size={:?}, my_size={:?}, my_pos={:?}, texture_loaded={}, has_texture={:?}",
            texture_changed,
            new_value.texture_mode(),
            parent_size,
            my_size,
            my_pos,
            self.texture_loaded,
            new_value.texture.is_some()
        );
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
                        let mut promise =
                            content_provider.bind_mut().fetch_texture_by_hash_original(
                                texture_hash.to_godot(),
                                DclContentMappingAndUrl::from_ref(content_mapping),
                            );

                        // Use the _original suffix to match the cache key used by fetch_texture_by_hash_original
                        self.waiting_hash = format!("{}_original", texture_hash).to_godot();

                        if !promise.bind().is_resolved() {
                            promise.connect(
                                "on_resolved",
                                &self.base().callable("_on_texture_loaded"),
                            );
                        }

                        self.first_texture_load_shot = true;
                        self.base_mut().call_deferred("_on_texture_loaded", &[]);
                    }
                    DclSourceTex::VideoTexture(_) => {
                        // TODO: implement video texture
                    }
                    DclSourceTex::AvatarTexture(user_id) => {
                        let global = DclGlobal::singleton();
                        let mut content_provider = global.bind().get_content_provider();
                        let mut promise = content_provider
                            .bind_mut()
                            .fetch_avatar_texture(user_id.to_godot());

                        // Store user_id with prefix so callback knows it's an avatar texture
                        self.waiting_hash = format!("avatar:{}", user_id).to_godot();

                        if !promise.bind().is_resolved() {
                            promise.connect(
                                "on_resolved",
                                &self.base().callable("_on_avatar_texture_loaded"),
                            );
                        }

                        self.first_texture_load_shot = true;
                        self.base_mut()
                            .call_deferred("_on_avatar_texture_loaded", &[]);
                    }
                }
            } else {
                self.base_mut()
                    .set_texture(&load::<Texture2D>("res://assets/white_pixel.png"));
            }

            if self.current_value.texture.is_none() {
                let mut base_mut = self.base_mut();
                base_mut.set_anchors_preset(godot::classes::control::LayoutPreset::FULL_RECT);
                base_mut.set_patch_margin(godot::builtin::Side::BOTTOM, 0);
                base_mut.set_patch_margin(godot::builtin::Side::LEFT, 0);
                base_mut.set_patch_margin(godot::builtin::Side::TOP, 0);
                base_mut.set_patch_margin(godot::builtin::Side::RIGHT, 0);
                base_mut.set_h_axis_stretch_mode(
                    godot::classes::nine_patch_rect::AxisStretchMode::STRETCH,
                );
                base_mut.set_v_axis_stretch_mode(
                    godot::classes::nine_patch_rect::AxisStretchMode::STRETCH,
                );
            }
        } else {
            tracing::debug!(
                "[UI_BKG] change_value: texture NOT changed, calling _set_texture_params"
            );
            self._set_texture_params();
        }

        let final_size = self.base().get_size();
        let final_pos = self.base().get_position();
        tracing::debug!(
            "[UI_BKG] change_value END: final_size={:?}, final_pos={:?}",
            final_size,
            final_pos
        );

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
