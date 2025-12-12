use godot::{engine::ConfigFile, prelude::*};

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

#[derive(GodotClass)]
#[class(base=RefCounted)]
pub struct DclConfig {
    _base: Base<RefCounted>,

    #[var]
    pub settings_file: Gd<ConfigFile>,

    #[export(enum = (Low, Medium, High, Source))]
    pub texture_quality: TextureQuality,
}

#[godot_api]
impl IRefCounted for DclConfig {
    fn init(base: Base<RefCounted>) -> Self {
        let mut settings_file: Gd<ConfigFile> = ConfigFile::new_gd();
        settings_file.load(DclConfig::get_settings_file_path());

        let texture_quality = settings_file
            .get_value_ex("config".to_godot(), "texture_quality".to_godot())
            .default(Variant::from(TextureQuality::Medium.to_i32()))
            .done();
        let texture_quality = texture_quality
            .try_to::<i32>()
            .unwrap_or(TextureQuality::Medium.to_i32());

        Self {
            _base: base,
            settings_file,
            texture_quality: TextureQuality::from_i32(texture_quality),
        }
    }
}

#[godot_api]
impl DclConfig {
    #[func]
    pub fn get_settings_file_path() -> GString {
        "user://settings.cfg".to_godot()
    }

    pub fn static_get_texture_quality() -> TextureQuality {
        let mut settings_file: Gd<ConfigFile> = ConfigFile::new_gd();
        settings_file.load(DclConfig::get_settings_file_path());
        let texture_quality = settings_file
            .get_value_ex("config".to_godot(), "texture_quality".to_godot())
            .default(Variant::from(TextureQuality::Medium.to_i32()))
            .done();

        let texture_quality = texture_quality
            .try_to::<i32>()
            .unwrap_or(TextureQuality::Medium.to_i32());
        TextureQuality::from_i32(texture_quality)
    }

    #[func]
    pub fn generate_uuid_v4() -> GString {
        uuid::Uuid::new_v4().to_string().to_godot()
    }
}
