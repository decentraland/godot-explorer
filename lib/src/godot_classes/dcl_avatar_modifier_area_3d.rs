use godot::engine::Area3D;
use godot::prelude::*;

#[repr(i32)]
#[derive(Property, Export, PartialEq, Debug)]
pub enum AvatarModifierType {
    HideAvatar = 0,
    DisablePassports = 1,
}

#[derive(GodotClass)]
#[class(init, base=Area3D)]
pub struct DclAvatarModifierArea3D {
    #[export(enum = (HideAvatar, DisablePassports))]
    avatar_modifiers: Array<i32>,

    #[export]
    exclude_ids: Array<GString>,

    #[export]
    area: Vector3,

    #[base]
    _base: Base<Area3D>,
}

#[godot_api]
impl DclAvatarModifierArea3D {}
