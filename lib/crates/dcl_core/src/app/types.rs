use std::fmt;

/// A scene parcel on the world grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct Parcel {
    pub x: i32,
    pub y: i32,
}

impl Parcel {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum SignInMethod {
    Guest,
    Email,
    Browser,
    Wallet,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum SignInStep {
    Starting,
    AwaitingCode,
    AwaitingBrowser,
    AwaitingWallet,
}

impl SignInStep {
    pub(super) fn first_for(method: SignInMethod) -> Self {
        match method {
            SignInMethod::Guest => Self::Starting,
            SignInMethod::Email => Self::Starting,
            SignInMethod::Browser => Self::AwaitingBrowser,
            SignInMethod::Wallet => Self::AwaitingWallet,
        }
    }
}

/// Pixels in the host's surface, where the avatar preview should render.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct Destination {
    /// Realm URL; `None` is the default realm.
    pub realm: Option<String>,
    pub parcel: Option<Parcel>,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum State {
    Booting,
    SignedOut,
    SigningIn {
        method: SignInMethod,
        step: SignInStep,
    },
    SignedIn,
    AvatarPreview {
        rect: Rect,
    },
    Loading {
        destination: Destination,
        progress: f32,
    },
    InWorld {
        realm: String,
        parcel: Parcel,
    },
}

impl State {
    pub fn is_signed_in(&self) -> bool {
        matches!(
            self,
            Self::SignedIn
                | Self::AvatarPreview { .. }
                | Self::Loading { .. }
                | Self::InWorld { .. }
        )
    }

    pub fn is_preview(&self) -> bool {
        matches!(self, Self::AvatarPreview { .. })
    }

    pub fn in_world_flow(&self) -> bool {
        matches!(self, Self::Loading { .. } | Self::InWorld { .. })
    }

    /// The states in which the engine surface is visible and rendering.
    pub fn shows_engine(&self) -> bool {
        self.is_preview() || self.in_world_flow()
    }
}

/// Core → engine. The engine executes; it never decides.
#[derive(Clone, Debug, PartialEq)]
pub enum EngineCommand {
    EnterWorld(Destination),
    ExitWorld,
    ShowAvatarPreview(Rect),
    HideAvatarPreview,
    SetRendering(bool),
    Control(super::Control),
}

/// Engine → core.
#[derive(Clone, Debug, PartialEq)]
pub enum EngineEvent {
    LoadingProgress(f32),
    LoadingFinished { realm: String, parcel: Parcel },
    SceneCrashed,
    Hud(super::HudSnapshot),
    Chat(super::ChatLine),
    UnreadChat(u32),
    UiState(super::UiState),
    OpenBackpack { emotes: bool },
    LoadingDetail(super::LoadingDetail),
    Avatar(super::AvatarState),
}

/// What the attached engine implements.
pub trait EngineSink: Send + Sync {
    fn send(&self, command: EngineCommand);
}

/// What the host and the shell observe (the FFI callback interface forwards these).
pub trait AppEvents: Send + Sync {
    fn state_changed(&self, state: &State);
    /// The host creates the engine view; the engine then attaches itself (D-42).
    fn engine_required(&self);
    /// The host flips the surface order.
    fn avatar_preview_mode(&self, on: bool);
    fn scene_crashed(&self);
    /// A sign-in step failed; `code` is stable and the shell maps it to copy.
    fn auth_error(&self, _code: &str) {}
    /// The shell shows `request.url` in a browser or webview.
    fn open_url(&self, _request: &crate::OpenUrlRequest) {}
    /// Friends, requests or blocks changed; the shell refetches what it shows.
    fn friends_changed(&self) {}
    fn settings_changed(&self, _entry: &crate::settings::SettingEntry) {}
    fn hud(&self, _snapshot: &super::HudSnapshot) {}
    fn chat(&self, _line: &super::ChatLine) {}
    fn unread_chat(&self, _count: u32) {}
    fn ui_state(&self, _state: &super::UiState) {}
    fn open_backpack(&self, _emotes: bool) {}
    fn loading_detail(&self, _detail: &super::LoadingDetail) {}
    fn avatar_changed(&self, _avatar: &super::AvatarState) {}
}

#[derive(Debug, PartialEq)]
pub struct AppError {
    pub call: &'static str,
    pub state: String,
}

impl AppError {
    pub(super) fn invalid(call: &'static str, state: &State) -> Self {
        Self {
            call,
            state: format!("{state:?}"),
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} is not valid in state {}", self.call, self.state)
    }
}

impl std::error::Error for AppError {}
