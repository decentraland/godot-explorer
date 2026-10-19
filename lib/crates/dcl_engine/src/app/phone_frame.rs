//! The phone frame drawn over the window while a phone is emulated on the desktop: rounded
//! corners, notch or camera hole, per platform and orientation.

use godot::classes::control::{LayoutPreset, MouseFilter};
use godot::classes::{CanvasLayer, ColorRect, ICanvasLayer, Shader, ShaderMaterial};
use godot::prelude::*;

use crate::godot_classes::dcl_global::DclGlobal;

const SHADER: &str = "res://assets/no-export/phone_frame_overlay.gdshader";
/// On top of everything.
const LAYER: i32 = 100;

#[derive(GodotClass)]
#[class(base=CanvasLayer, init)]
pub struct PhoneFrameOverlay {
    base: Base<CanvasLayer>,
    material: Option<Gd<ShaderMaterial>>,
}

#[godot_api]
impl ICanvasLayer for PhoneFrameOverlay {
    fn ready(&mut self) {
        self.base_mut().set_layer(LAYER);
        self.base_mut().set_follow_viewport(false);
        let mut rect = ColorRect::new_alloc();
        rect.set_name("PhoneFrameRect");
        rect.set_anchors_preset(LayoutPreset::FULL_RECT);
        rect.set_mouse_filter(MouseFilter::IGNORE);
        let mut material = ShaderMaterial::new_gd();
        material.set_shader(&godot::tools::load::<Shader>(SHADER));
        rect.set_material(&material);
        self.base_mut().add_child(&rect);
        self.material = Some(material);
        self.update_params();
        if let Some(window) = self.base().get_window() {
            let me = self.to_gd();
            window
                .signals()
                .size_changed()
                .connect_other(&me, |this: &mut Self| this.update_params());
        }
    }
}

impl PhoneFrameOverlay {
    fn update_params(&mut self) {
        let Some(material) = self.material.as_mut() else {
            return;
        };
        let global = DclGlobal::singleton();
        let is_ios = global.bind().cli.bind().emulate_ios;
        let portrait = crate::ui::host::is_portrait();
        material.set_shader_parameter("is_ios", &is_ios.to_variant());
        material.set_shader_parameter("is_portrait", &portrait.to_variant());
    }
}
