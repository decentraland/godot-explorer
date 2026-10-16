use godot::classes::{Area3D, IArea3D};
use godot::prelude::*;

#[derive(Clone, Copy, Var, GodotConvert, Export, PartialEq, Debug)]
#[godot(via = i32)]
pub enum AvatarModifierType {
    HideAvatar = 0,
    DisablePassports = 1,
    HideNameTags = 2,
}

#[derive(GodotClass)]
#[class(base=Area3D)]
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

    /// The scene the area belongs to; -1 until it is placed in one.
    #[var(pub)]
    scene_id: i32,

    _base: Base<Area3D>,
}

#[godot_api]
impl IArea3D for DclAvatarModifierArea3D {
    fn init(base: Base<Area3D>) -> Self {
        Self {
            avatar_modifiers: Array::new(),
            exclude_ids: Array::new(),
            area: Vector3::ZERO,
            scene_id: -1,
            _base: base,
        }
    }
}
