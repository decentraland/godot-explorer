pub mod internal_player_data;
pub mod material;
pub mod proto_components;
pub mod transform_and_parent;

use std::hash::Hash;

use godot::{engine::global::PropertyHint, prelude::*, register::property::PropertyHintInfo};

#[derive(PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Clone, Copy, Default)]
pub struct SceneEntityId {
    pub number: u16,
    pub version: u16,
}

impl GodotConvert for SceneEntityId {
    type Via = i32;
}

impl Var for SceneEntityId {
    fn get_property(&self) -> Self::Via {
        self.as_i32()
    }

    fn set_property(&mut self, value: Self::Via) {
        *self = Self::from_i32(value);
    }
}

impl Export for SceneEntityId {
    fn default_export_info() -> PropertyHintInfo {
        PropertyHintInfo {
            hint: PropertyHint::NONE,
            hint_string: "Entity ID in the owner scene".into(),
        }
    }
}

impl SceneEntityId {
    pub fn new(number: u16, version: u16) -> Self {
        Self { number, version }
    }

    pub fn from_i32(value: i32) -> Self {
        Self {
            number: ((value as u32) & 0xffff) as u16,
            version: (((value as u32) & 0xffff0000) >> 16) as u16,
        }
    }

    const fn reserved(number: u16) -> Self {
        Self { number, version: 0 }
    }

    // Note: this is not actually an invalid id, but it's used to represent an invalid id in some cases
    pub const INVALID: SceneEntityId = Self {
        number: 0xFFFF,
        version: 0xFFFF,
    };
    pub const ROOT: SceneEntityId = Self::reserved(0);
    pub const PLAYER: SceneEntityId = Self::reserved(1);
    pub const CAMERA: SceneEntityId = Self::reserved(2);

    pub fn as_i32(&self) -> i32 {
        ((self.version as u32) << 16 | self.number as u32) as i32
    }

    pub fn as_usize(&self) -> usize {
        (self.version as usize) << 16 | self.number as usize
    }
}

impl std::fmt::Display for SceneEntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("dcl_{}v{}", self.number, self.version))
    }
}

impl SceneComponentId {
    pub const TRANSFORM: SceneComponentId = SceneComponentId(1);

    // Custom GODOT implementation components
    pub const INTERNAL_PLAYER_DATA: SceneComponentId = SceneComponentId(101);
}

include!(concat!(env!("OUT_DIR"), "/components_enum.gen.rs"));

#[derive(PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Clone, Copy)]
pub struct SceneComponentId(pub u32);

#[derive(PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Clone, Copy)]
pub struct SceneCrdtTimestamp(pub u32);
