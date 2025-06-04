use godot::prelude::*;

use crate::comms::profile::{AvatarColor, AvatarEmote, AvatarSnapshots, AvatarWireFormat};

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
#[class(init, base=RefCounted)]
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
    fn equal(&self, other: Gd<DclAvatarWireFormat>) -> bool {
        self.inner == other.bind().inner
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
                emotes[i] = GString::from(emote.urn.as_str());
            } else {
                emotes[i] = GString::from(*emote);
            }
        }
        emotes
    }

    #[func]
    fn get_snapshots_face_hash(&self) -> GString {
        self.inner.snapshots.as_ref().map_or_else(
            || GString::from(""),
            |snapshots| GString::from(snapshots.face256.clone()),
        )
    }

    #[func]
    fn get_snapshots_face_url(&self) -> GString {
        self.inner
            .snapshots
            .as_ref()
            .and_then(|snapshots| snapshots.face_url.as_ref())
            .map_or_else(
                || GString::from(""),
                |face_url| GString::from(face_url.clone()),
            )
    }

    #[func]
    fn get_force_render(&self) -> Array<GString> {
        if let Some(array) = &self.inner.force_render {
            Array::from_iter(array.iter().map(GString::from))
        } else {
            Array::new()
        }
    }

    #[func]
    fn get_snapshots_body_url(&self) -> GString {
        self.inner
            .snapshots
            .as_ref()
            .and_then(|snapshots| snapshots.body_url.as_ref())
            .map_or_else(
                || GString::from(""),
                |body_url| GString::from(body_url.clone()),
            )
    }

    #[func]
    fn set_force_render(&mut self, force_render: Array<Variant>) {
        self.inner.force_render = Some(Vec::from_iter(
            force_render.iter_shared().map(|v| v.to_string()),
        ));
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
            if let Some(wearable) = wearables.get(i).as_ref() {
                wearables_vec.push(wearable.to_string());
            } else {
                tracing::error!("Invalid wearable at index {}", i);
            }
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
                urn: emotes.get(i).as_ref().unwrap().to_string(),
            });
        }
        self.inner.emotes = Some(emotes_vec);
    }

    #[func]
    fn set_snapshots(&mut self, face256: GString, body: GString) {
        self.inner.snapshots = Some(AvatarSnapshots {
            face256: face256.to_string(),
            body: body.to_string(),
            body_url: None,
            face_url: None,
        });
    }

    #[func]
    pub fn from_godot_dictionary(dictionary: Dictionary) -> Gd<DclAvatarWireFormat> {
        // 1) stringify the Godot Dictionary → JSON5-ish string
        let json_str = godot::engine::Json::stringify(dictionary.to_variant()).to_string();

        // 2) parse with json5 (tolerant of trailing commas)
        let avatar: AvatarWireFormat = json5::from_str(&json_str).unwrap_or_default();

        // 3) wrap and return
        DclAvatarWireFormat::from_gd(avatar)
    }

    #[func]
    pub fn to_godot_dictionary(&self) -> Dictionary {
        let value = serde_json::to_string(&self.inner).unwrap_or_default();
        let value = godot::engine::Json::parse_string(value.into());
        value.to::<Dictionary>()
    }
}
