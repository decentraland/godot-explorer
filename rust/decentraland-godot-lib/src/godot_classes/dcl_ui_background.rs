use godot::engine::{ImageTexture, NinePatchRect};
use godot::prelude::*;

use crate::dcl::components::proto_components::sdk::components::PbUiBackground;

#[derive(GodotClass)]
#[class(init, base=NinePatchRect)]
pub struct DclUiBackground {
    #[base]
    _base: Base<NinePatchRect>,

    last_value: PbUiBackground,
}

#[godot_api]
impl DclUiBackground {
    fn init(base: Base<NinePatchRect>) -> Self {
        Self {
            _base: base,
            last_value: PbUiBackground::default(),
        }
    }

    pub fn change_value(&mut self, new_value: PbUiBackground) {
        // texture change if
        if new_value.texture != self.last_value.texture {}
    }
}
