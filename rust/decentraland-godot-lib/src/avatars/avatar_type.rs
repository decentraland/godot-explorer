use godot::{
    bind::{godot_api, GodotClass},
    obj::Gd,
    prelude::*,
};

use crate::{
    comms::profile::{AvatarColor, AvatarEmote, AvatarSnapshots, AvatarWireFormat},
    dcl::scene_apis::Snapshots,
};

const AVATAR_EMOTE_SLOTS_COUNT: usize = 10;
const DEFAULT_EMOTES: [&str; AVATAR_EMOTE_SLOTS_COUNT] = [
    "handsair",
    "wave",
    "fistpump",
    "dance",
    "raiseHand",
    "clap",
    "money",
    "kiss",
    "headexplode",
    "shrug",
];

#[derive(GodotClass)]
#[class(base=RefCounted)]
pub struct DclAvatarWireFormat {
    pub inner: AvatarWireFormat,
}

impl DclAvatarWireFormat {
    pub fn from_gd(inner: AvatarWireFormat) -> Gd<Self> {
        Gd::from_init_fn(|_base| Self { inner })
    }
}

#[godot_api]
impl DclAvatarWireFormat {
    #[func]
    fn get_name(&self) -> GString {
        if let Some(name) = self.inner.name.as_ref() {
            GString::from(name)
        } else {
            GString::from("")
        }
    }
    #[func]
    fn get_body_shape(&self) -> GString {
        if let Some(body_shape) = self.inner.body_shape.as_ref() {
            GString::from(body_shape)
        } else {
            GString::from("")
        }
    }

    #[func]
    fn get_eyes_color(&self) -> Color {
        if let Some(eyes) = self.inner.eyes.as_ref() {
            eyes.into()
        } else {
            Color::WHITE
        }
    }

    #[func]
    fn get_hair_color(&self) -> Color {
        if let Some(hair) = self.inner.hair.as_ref() {
            hair.into()
        } else {
            Color::WHITE
        }
    }

    #[func]
    fn get_skin_color(&self) -> Color {
        if let Some(skin) = self.inner.skin.as_ref() {
            skin.into()
        } else {
            Color::WHITE
        }
    }

    #[func]
    fn get_wearables(&self) -> PackedStringArray {
        let mut wearables = PackedStringArray::new();
        for wearable in self.inner.wearables.iter() {
            wearables.push(GString::from(wearable));
        }
        wearables
    }

    #[func]
    fn get_emotes(&self) -> PackedStringArray {
        let mut emotes = PackedStringArray::new();
        let empty_emotes = vec![];
        let used_emotes = self.inner.emotes.as_ref().unwrap_or(&empty_emotes);

        emotes.resize(AVATAR_EMOTE_SLOTS_COUNT);

        for (i, emote) in DEFAULT_EMOTES.iter().enumerate() {
            if let Some(emote) = used_emotes.iter().find(|e| e.slot == i as u32) {
                emotes.set(i, GString::from(emote.urn.as_str()));
            } else {
                emotes.set(i, GString::from(*emote));
            }
        }
        emotes
    }

    #[func]
    fn get_snapshots_face256(&self) -> GString {
        if let Some(snapshots) = &self.inner.snapshots {
            GString::from(snapshots.face256.clone())
        } else {
            GString::from("")
        }
    }

    #[func]
    fn get_snapshots_body(&self) -> GString {
        if let Some(snapshots) = &self.inner.snapshots {
            GString::from(snapshots.body.clone())
        } else {
            GString::from("")
        }
    }

    #[func]
    fn set_name(&mut self, name: GString) {
        self.inner.name = Some(name.to_string());
    }

    #[func]
    fn set_body_shape(&mut self, body_shape: GString) {
        self.inner.body_shape = Some(body_shape.to_string());
    }

    #[func]
    fn set_eyes_color(&mut self, color: Color) {
        self.inner.eyes = Some(AvatarColor::from(&color));
    }

    #[func]
    fn set_hair_color(&mut self, color: Color) {
        self.inner.hair = Some(AvatarColor::from(&color));
    }

    #[func]
    fn set_skin_color(&mut self, color: Color) {
        self.inner.skin = Some(AvatarColor::from(&color));
    }

    #[func]
    fn set_wearables(&mut self, wearables: PackedStringArray) {
        let mut wearables_vec = Vec::new();
        for i in 0..wearables.len() {
            wearables_vec.push(wearables.get(i).to_string());
        }
        self.inner.wearables = wearables_vec;
    }

    #[func]
    fn set_emotes(&mut self, emotes: PackedStringArray) {
        if emotes.len() != AVATAR_EMOTE_SLOTS_COUNT {
            tracing::error!("Invalid emotes array length");
            return;
        }

        let mut emotes_vec = Vec::new();
        for i in 0..10 {
            emotes_vec.push(AvatarEmote {
                slot: i as u32,
                urn: emotes.get(i).to_string(),
            });
        }
        self.inner.emotes = Some(emotes_vec);
    }

    #[func]
    fn set_snapshots(&mut self, face256: GString, body: GString) {
        self.inner.snapshots = Some(AvatarSnapshots {
            face256: face256.to_string(),
            body: body.to_string(),
        });
    }

    #[func]
    pub fn from_godot_dictionary(dictionary: Dictionary) -> Gd<DclAvatarWireFormat> {
        let value = godot::engine::Json::stringify(dictionary.to_variant());
        DclAvatarWireFormat::from_gd(
            serde_json::from_str(value.to_string().as_str()).unwrap_or_default(),
        )
    }

    #[func]
    pub fn to_godot_dictionary(&self) -> Dictionary {
        let value = serde_json::to_string(&self.inner).unwrap_or_default();
        let value = godot::engine::Json::parse_string(value.into());
        value.to::<Dictionary>()
    }
}
