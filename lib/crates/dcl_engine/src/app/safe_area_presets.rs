//! Reference phones for the desktop emulation of a device safe area (`--emulate-ios`,
//! `--emulate-android`): the window takes the phone's proportions and the safe area is its notch
//! and home-indicator insets, scaled from the phone's native resolution.

use godot::builtin::{Rect2i, Vector2i};

/// Insets in pixels at the phone's native resolution.
#[derive(Clone, Copy)]
struct Insets {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

struct Phone {
    landscape_size: Vector2i,
    portrait_size: Vector2i,
    landscape: Insets,
    portrait: Insets,
}

/// An iPhone 14 Pro (2556x1179). Landscape insets 177/0/177/63, portrait 0/177/0/102.
const IOS: Phone = Phone {
    landscape_size: Vector2i::new(2556, 1179),
    portrait_size: Vector2i::new(1179, 2556),
    landscape: Insets {
        left: 177,
        top: 0,
        right: 177,
        bottom: 63,
    },
    portrait: Insets {
        left: 0,
        top: 177,
        right: 0,
        bottom: 102,
    },
};

/// A Motorola Edge 60 Pro (2712x1220). The camera notch is 128 px on the left in landscape and
/// the top in portrait.
const ANDROID: Phone = Phone {
    landscape_size: Vector2i::new(2712, 1220),
    portrait_size: Vector2i::new(1220, 2712),
    landscape: Insets {
        left: 128,
        top: 0,
        right: 0,
        bottom: 0,
    },
    portrait: Insets {
        left: 0,
        top: 128,
        right: 0,
        bottom: 0,
    },
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Emulated {
    Ios,
    Android,
}

impl Emulated {
    fn phone(self) -> &'static Phone {
        match self {
            Emulated::Ios => &IOS,
            Emulated::Android => &ANDROID,
        }
    }
}

/// The safe area of `window_size` for the phone, insets scaled proportionally.
pub fn safe_area(phone: Emulated, portrait: bool, window_size: Vector2i) -> Rect2i {
    let p = phone.phone();
    let (reference, insets) = if portrait {
        (p.portrait_size, p.portrait)
    } else {
        (p.landscape_size, p.landscape)
    };
    let scale_x = window_size.x as f32 / reference.x as f32;
    let scale_y = window_size.y as f32 / reference.y as f32;
    let left = (insets.left as f32 * scale_x) as i32;
    let top = (insets.top as f32 * scale_y) as i32;
    let right = (insets.right as f32 * scale_x) as i32;
    let bottom = (insets.bottom as f32 * scale_y) as i32;
    Rect2i::new(
        Vector2i::new(left, top),
        Vector2i::new(window_size.x - left - right, window_size.y - top - bottom),
    )
}

/// The emulation window: 1280 tall in portrait and 720 in landscape, at the phone's aspect ratio
/// (fixed sizes, so a rounding of the reference resolution never shifts them by a pixel).
pub fn window_size(phone: Emulated, portrait: bool) -> Vector2i {
    match (phone, portrait) {
        (Emulated::Ios, true) => Vector2i::new(590, 1280),
        (Emulated::Ios, false) => Vector2i::new(1561, 720),
        (Emulated::Android, true) => Vector2i::new(576, 1280),
        (Emulated::Android, false) => Vector2i::new(1600, 720),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_keep_the_phones_aspect_ratio() {
        assert_eq!(window_size(Emulated::Ios, true), Vector2i::new(590, 1280));
        assert_eq!(window_size(Emulated::Ios, false), Vector2i::new(1561, 720));
        assert_eq!(
            window_size(Emulated::Android, true),
            Vector2i::new(576, 1280)
        );
        assert_eq!(
            window_size(Emulated::Android, false),
            Vector2i::new(1600, 720)
        );
    }

    #[test]
    fn insets_scale_with_the_window() {
        // At the phone's native resolution the insets are the reference ones.
        let native = safe_area(Emulated::Ios, false, Vector2i::new(2556, 1179));
        assert_eq!(
            native,
            Rect2i::new(Vector2i::new(177, 0), Vector2i::new(2202, 1116))
        );
        // Halved window, halved insets.
        let half = safe_area(Emulated::Ios, false, Vector2i::new(1278, 589));
        assert_eq!(half.position, Vector2i::new(88, 0));
    }

    #[test]
    fn android_has_only_the_camera_notch() {
        let landscape = safe_area(Emulated::Android, false, Vector2i::new(2712, 1220));
        assert_eq!(
            landscape,
            Rect2i::new(Vector2i::new(128, 0), Vector2i::new(2584, 1220))
        );
        let portrait = safe_area(Emulated::Android, true, Vector2i::new(1220, 2712));
        assert_eq!(
            portrait,
            Rect2i::new(Vector2i::new(0, 128), Vector2i::new(1220, 2584))
        );
    }
}
