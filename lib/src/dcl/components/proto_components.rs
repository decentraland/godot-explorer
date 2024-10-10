use sdk::components::TextWrap;

pub mod sdk {
    #[allow(clippy::all)]
    pub mod components {
        include!(concat!(env!("OUT_DIR"), "/decentraland.sdk.components.rs"));

        pub mod common {
            include!(concat!(
                env!("OUT_DIR"),
                "/decentraland.sdk.components.common.rs"
            ));
        }
    }
}

pub mod common {
    include!(concat!(env!("OUT_DIR"), "/decentraland.common.rs"));

    impl Color4 {
        pub fn black() -> Self {
            Self {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            }
        }
        pub fn white() -> Self {
            Self {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 1.0,
            }
        }
        pub fn to_godot(&self) -> godot::prelude::Color {
            godot::prelude::Color::from_rgba(self.r, self.g, self.b, self.a)
        }

        pub fn to_color_string(&self) -> String {
            format!(
                "#{:02x}{:02x}{:02x}{:02x}",
                (self.r * 255.0) as u8,
                (self.g * 255.0) as u8,
                (self.b * 255.0) as u8,
                (self.a * 255.0) as u8
            )
        }

        pub fn multiply(&mut self, factor: f32) -> Self {
            Self {
                r: self.r * factor,
                g: self.g * factor,
                b: self.b * factor,
                a: self.a * factor,
            }
        }
    }

    impl Color3 {
        pub fn black() -> Self {
            Self {
                r: 0.0,
                g: 0.0,
                b: 0.0,
            }
        }
        pub fn white() -> Self {
            Self {
                r: 1.0,
                g: 1.0,
                b: 1.0,
            }
        }
        pub fn to_godot(&self) -> godot::prelude::Color {
            godot::prelude::Color::from_rgba(self.r, self.g, self.b, 1.0)
        }

        pub fn to_color_string(&self) -> String {
            format!(
                "#{:02x}{:02x}{:02x}",
                (self.r * 255.0) as u8,
                (self.g * 255.0) as u8,
                (self.b * 255.0) as u8
            )
        }

        pub fn multiply(&mut self, factor: f32) -> Self {
            Self {
                r: self.r * factor,
                g: self.g * factor,
                b: self.b * factor,
            }
        }
    }

    impl Vector3 {
        pub fn to_godot(&self) -> godot::prelude::Vector3 {
            godot::prelude::Vector3::new(self.x, self.y, self.z)
        }
    }

    impl Position {
        pub fn to_godot(&self) -> godot::prelude::Vector3 {
            godot::prelude::Vector3::new(self.x, self.y, self.z)
        }
    }

    impl Vector2 {
        pub fn to_godot(&self) -> godot::prelude::Vector2 {
            godot::prelude::Vector2::new(self.x, self.y)
        }
    }

    impl Quaternion {
        pub fn to_godot(&self) -> godot::prelude::Quaternion {
            godot::prelude::Quaternion::new(self.x, self.y, self.z, self.w)
        }
    }
}

pub trait WrapToGodot<T> {
    fn to_godot_or_else(&self, default: T) -> T;
}

impl WrapToGodot<godot::prelude::Color> for Option<common::Color4> {
    fn to_godot_or_else(&self, default: godot::prelude::Color) -> godot::prelude::Color {
        self.as_ref().map(|v| v.to_godot()).unwrap_or(default)
    }
}

impl WrapToGodot<godot::prelude::Color> for Option<common::Color3> {
    fn to_godot_or_else(&self, default: godot::prelude::Color) -> godot::prelude::Color {
        self.as_ref().map(|v| v.to_godot()).unwrap_or(default)
    }
}

impl sdk::components::common::TextAlignMode {
    pub fn to_godot(
        &self,
    ) -> (
        godot::engine::global::HorizontalAlignment,
        godot::engine::global::VerticalAlignment,
    ) {
        match self {
            sdk::components::common::TextAlignMode::TamTopLeft => (
                godot::engine::global::HorizontalAlignment::LEFT,
                godot::engine::global::VerticalAlignment::TOP,
            ),
            sdk::components::common::TextAlignMode::TamTopCenter => (
                godot::engine::global::HorizontalAlignment::CENTER,
                godot::engine::global::VerticalAlignment::TOP,
            ),
            sdk::components::common::TextAlignMode::TamTopRight => (
                godot::engine::global::HorizontalAlignment::RIGHT,
                godot::engine::global::VerticalAlignment::TOP,
            ),
            sdk::components::common::TextAlignMode::TamMiddleLeft => (
                godot::engine::global::HorizontalAlignment::LEFT,
                godot::engine::global::VerticalAlignment::CENTER,
            ),
            sdk::components::common::TextAlignMode::TamMiddleCenter => (
                godot::engine::global::HorizontalAlignment::CENTER,
                godot::engine::global::VerticalAlignment::CENTER,
            ),
            sdk::components::common::TextAlignMode::TamMiddleRight => (
                godot::engine::global::HorizontalAlignment::RIGHT,
                godot::engine::global::VerticalAlignment::CENTER,
            ),
            sdk::components::common::TextAlignMode::TamBottomLeft => (
                godot::engine::global::HorizontalAlignment::LEFT,
                godot::engine::global::VerticalAlignment::BOTTOM,
            ),
            sdk::components::common::TextAlignMode::TamBottomCenter => (
                godot::engine::global::HorizontalAlignment::CENTER,
                godot::engine::global::VerticalAlignment::BOTTOM,
            ),
            sdk::components::common::TextAlignMode::TamBottomRight => (
                godot::engine::global::HorizontalAlignment::RIGHT,
                godot::engine::global::VerticalAlignment::BOTTOM,
            ),
        }
    }
}

impl sdk::components::PbAnimationState {
    pub fn playing_backward(&self) -> bool {
        self.speed() < 0.0
    }
}

impl sdk::components::PbUiText {
    pub fn text_wrap_compat(&self) -> TextWrap {
        if self.text_wrap.is_none() {
            return TextWrap::TwNoWrap;
        }

        self.text_wrap()
    }
}

pub mod kernel {
    #[allow(clippy::all)]
    pub mod comms {
        pub mod rfc5 {
            include!(concat!(
                env!("OUT_DIR"),
                "/decentraland.kernel.comms.rfc5.rs"
            ));
        }
        pub mod rfc4 {
            include!(concat!(
                env!("OUT_DIR"),
                "/decentraland.kernel.comms.rfc4.rs"
            ));
        }
        pub mod v3 {
            include!(concat!(env!("OUT_DIR"), "/decentraland.kernel.comms.v3.rs"));
        }
    }
}
