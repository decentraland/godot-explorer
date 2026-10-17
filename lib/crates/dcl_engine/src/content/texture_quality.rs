use godot::prelude::*;

#[derive(Clone, Var, GodotConvert, Export, PartialEq, Debug)]
#[godot(via=i32)]
pub enum TextureQuality {
    Low = 0,
    Medium = 1,
    High = 2,
    Source = 3,
}

impl TextureQuality {
    pub fn from_i32(value: i32) -> Self {
        match value {
            0 => Self::Low,
            1 => Self::Medium,
            2 => Self::High,
            3 => Self::Source,
            _ => Self::Medium,
        }
    }

    pub fn to_i32(&self) -> i32 {
        match self {
            Self::Low => 0,
            Self::Medium => 1,
            Self::High => 2,
            Self::Source => 3,
        }
    }

    pub fn to_max_size(&self) -> i32 {
        match self {
            Self::Low => 256,
            Self::Medium => 512,
            Self::High => 1024,
            Self::Source => i32::MAX, // should we limit to 4k?
        }
    }
}
