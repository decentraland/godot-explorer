//! Typed commands from the shell to the running engine and the facts the engine reports back.
//! The core forwards them; the engine-side code executes and observes (D-3).

/// The physical pixels the HUD keeps clear on each side (notch, gesture bar, joypad).
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct HudInsets {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

/// What the avatar preview camera frames: the whole body or one wearable category.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum CameraFocus {
    Overall,
    Head,
    Torso,
    Legs,
    Feet,
    Hands,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum ColorTarget {
    Skin,
    Hair,
    Eyes,
}

/// What the shell asks of the engine once a world or the avatar preview is up. All of them are
/// ignored by an engine that has nothing to apply them to (no world, no preview).
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum Control {
    /// Move the player inside the current world.
    Teleport {
        parcel: super::Parcel,
    },
    SendChat {
        text: String,
    },
    /// An emote the player does in the world.
    Emote {
        urn: String,
    },
    SetHudInsets {
        insets: HudInsets,
    },
    SetChatWidth {
        width: f32,
    },
    ChatOpen {
        open: bool,
    },
    HideUi {
        hidden: bool,
    },
    FocusCamera {
        focus: CameraFocus,
    },
    Equip {
        urn: String,
    },
    Unequip {
        urn: String,
    },
    SetBodyShape {
        urn: String,
    },
    /// `color` is `#rrggbb`.
    SetColor {
        target: ColorTarget,
        color: String,
    },
    /// An emote played on the preview avatar.
    PlayPreviewEmote {
        urn: String,
    },
    SetEmoteSlot {
        slot: u32,
        urn: String,
    },
    RandomizeAvatar,
    SetName {
        name: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// The player's whereabouts, sent a few times a second while in a world.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct HudSnapshot {
    pub parcel: super::Parcel,
    pub realm: String,
    pub fps: f32,
    pub position: Vec3,
}

/// Who wrote a chat line, when the engine could resolve them.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct ChatProfile {
    pub name: String,
    pub has_claimed_name: bool,
    pub profile_picture_url: String,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct ChatLine {
    pub address: String,
    pub message: String,
    pub timestamp: f64,
    pub profile: Option<ChatProfile>,
}

/// Visibility of the engine's own UI, which the shell mirrors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct UiState {
    pub hidden: bool,
    pub chat_open: bool,
}

/// The equipped avatar; colors are `#rrggbb`, an empty emote slot is `""`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct AvatarState {
    pub body_shape: String,
    pub wearables: Vec<String>,
    pub emotes: Vec<String>,
    pub skin: String,
    pub hair: String,
    pub eyes: String,
}

/// How far the world's content has downloaded.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct LoadingDetail {
    pub loaded: u64,
    pub total: u64,
    pub mbps: f32,
    /// 0..100
    pub percent: f32,
}
