use godot::engine::Node3D;
use godot::prelude::*;

use crate::dcl::SceneId;

#[derive(GodotClass)]
#[class(base=Node3D)]
pub struct DclAvatar {
    #[var]
    current_parcel_scene_id: i32,

    #[base]
    _base: Base<Node3D>,
}

#[godot_api]
impl NodeVirtual for DclAvatar {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            current_parcel_scene_id: SceneId::INVALID as i32,
            _base: base,
        }
    }
}

#[godot_api]
impl DclAvatar {}
