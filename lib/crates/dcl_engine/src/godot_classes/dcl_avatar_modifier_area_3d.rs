use godot::classes::Area3D;
use godot::prelude::*;

#[derive(Clone, Copy, Var, GodotConvert, Export, PartialEq, Debug)]
#[godot(via = i32)]
pub enum AvatarModifierType {
    HideAvatar = 0,
    DisablePassports = 1,
    HideNameTags = 2,
}

#[derive(GodotClass)]
#[class(init, base=Area3D)]
pub struct DclAvatarModifierArea3D {
    #[export(enum = (HideAvatar, DisablePassports, HideNameTags))]
    #[var(pub)]
    avatar_modifiers: Array<i32>,

    #[export]
    #[var(pub)]
    exclude_ids: Array<GString>,

    #[export]
    #[var(pub)]
    area: Vector3,

    _base: Base<Area3D>,
}

#[godot_api]
impl DclAvatarModifierArea3D {}
