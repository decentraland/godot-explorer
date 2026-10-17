//! The HUD's safe-area frame: margins from the device safe area, floored by a shared profile.

use godot::classes::{IMarginContainer, IResource, MarginContainer, Resource};
use godot::prelude::*;

use super::host;

/// The shared HUD inset rectangle: one resource aligns every frame that uses it. Landscape uses
/// `left/top/right/bottom`, portrait the `portrait_*` set.
#[derive(GodotClass)]
#[class(base=Resource, init)]
pub struct SafeAreaMargins {
    base: Base<Resource>,
    #[export]
    pub left: i32,
    #[export]
    pub top: i32,
    #[export]
    pub right: i32,
    #[export]
    pub bottom: i32,
    #[export]
    pub portrait_left: i32,
    #[export]
    pub portrait_top: i32,
    #[export]
    pub portrait_right: i32,
    #[export]
    pub portrait_bottom: i32,
}

#[godot_api]
impl IResource for SafeAreaMargins {}

/// The four insets (top, left, bottom, right) a margin container resolves to.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Insets {
    pub top: i32,
    pub left: i32,
    pub bottom: i32,
    pub right: i32,
}

pub struct Rules {
    pub floor: Insets,
    pub symmetric_horizontal: bool,
    pub ignore_safe_area_bottom: bool,
}

/// The insets to apply: each side at least its floor; with `symmetric_horizontal` both sides take
/// the larger; with `ignore_safe_area_bottom` the bottom is the floor alone (a sibling bar above
/// it already reserves the OS inset).
pub fn apply_rules(device: Insets, rules: &Rules) -> Insets {
    let mut left = device.left.max(rules.floor.left);
    let mut right = device.right.max(rules.floor.right);
    if rules.symmetric_horizontal {
        let side = left.max(right);
        left = side;
        right = side;
    }
    let bottom = if rules.ignore_safe_area_bottom {
        rules.floor.bottom
    } else {
        device.bottom.max(rules.floor.bottom)
    };
    Insets {
        top: device.top.max(rules.floor.top),
        left,
        bottom,
        right,
    }
}

#[derive(GodotClass)]
#[class(base=MarginContainer, init)]
pub struct SafeMarginContainer {
    base: Base<MarginContainer>,
    #[export]
    default_margin: i32,
    #[export]
    #[init(val = true)]
    use_left: bool,
    #[export]
    #[init(val = true)]
    use_right: bool,
    #[export]
    #[init(val = true)]
    use_top: bool,
    #[export]
    #[init(val = true)]
    use_bottom: bool,
    #[export]
    min_margin_top: i32,
    #[export]
    min_margin_bottom: i32,
    #[export]
    min_margin_left: i32,
    #[export]
    min_margin_right: i32,
    /// When set, its values replace the `min_margin_*` floors.
    #[export]
    margin_profile: Option<Gd<SafeAreaMargins>>,
    #[export]
    symmetric_horizontal: bool,
    #[export]
    ignore_safe_area_bottom: bool,
}

#[godot_api]
impl IMarginContainer for SafeMarginContainer {
    fn ready(&mut self) {
        if let Some(window) = self.base().get_window() {
            window
                .signals()
                .size_changed()
                .connect_other(&self.to_gd(), |this| this.refresh_margins());
        }
        host::connect(
            "orientation_changed",
            &self.to_gd(),
            "on_orientation_changed",
        );
        self.refresh_margins();
    }
}

#[godot_api]
impl SafeMarginContainer {
    #[func]
    fn on_orientation_changed(&mut self, _portrait: bool) {
        self.refresh_margins();
    }

    /// Recomputes the margins from the current window size and safe area.
    #[func]
    pub fn refresh_margins(&mut self) {
        if !self.base().is_inside_tree() {
            return;
        }
        let Some(viewport) = self.base().get_viewport() else {
            return;
        };
        let base = self.default_margin;
        let mut device = Insets {
            top: base,
            left: base,
            bottom: base,
            right: base,
        };
        if host::has_safe_area() {
            let area = host::safe_area();
            let window = godot::classes::DisplayServer::singleton().window_get_size();
            let visible = viewport.get_visible_rect().size;
            let (x, y) = (
                visible.x / window.x.max(1) as f32,
                visible.y / window.y.max(1) as f32,
            );
            device = Insets {
                top: base.max((area.position.y as f32 * y) as i32),
                left: base.max((area.position.x as f32 * x) as i32),
                bottom: base.max(((area.end().y - window.y).abs() as f32 * y) as i32),
                right: base.max(((area.end().x - window.x).abs() as f32 * x) as i32),
            };
        }
        self.apply(device);
    }
}

impl SafeMarginContainer {
    fn floor(&self) -> Insets {
        match &self.margin_profile {
            Some(profile) => {
                let p = profile.bind();
                if host::is_portrait() {
                    Insets {
                        top: p.portrait_top,
                        left: p.portrait_left,
                        bottom: p.portrait_bottom,
                        right: p.portrait_right,
                    }
                } else {
                    Insets {
                        top: p.top,
                        left: p.left,
                        bottom: p.bottom,
                        right: p.right,
                    }
                }
            }
            None => Insets {
                top: self.min_margin_top,
                left: self.min_margin_left,
                bottom: self.min_margin_bottom,
                right: self.min_margin_right,
            },
        }
    }

    fn apply(&mut self, device: Insets) {
        let insets = apply_rules(
            device,
            &Rules {
                floor: self.floor(),
                symmetric_horizontal: self.symmetric_horizontal,
                ignore_safe_area_bottom: self.ignore_safe_area_bottom,
            },
        );
        let sides = [
            (self.use_top, "margin_top", insets.top),
            (self.use_left, "margin_left", insets.left),
            (self.use_right, "margin_right", insets.right),
            (self.use_bottom, "margin_bottom", insets.bottom),
        ];
        for (used, name, value) in sides {
            if used {
                self.base_mut().add_theme_constant_override(name, value);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules(floor: Insets, symmetric: bool, ignore_bottom: bool) -> Rules {
        Rules {
            floor,
            symmetric_horizontal: symmetric,
            ignore_safe_area_bottom: ignore_bottom,
        }
    }

    const ZERO: Insets = Insets {
        top: 0,
        left: 0,
        bottom: 0,
        right: 0,
    };

    #[test]
    fn floors_win_over_smaller_device_insets() {
        let floor = Insets {
            top: 32,
            left: 108,
            bottom: 45,
            right: 108,
        };
        let device = Insets {
            top: 10,
            left: 20,
            bottom: 60,
            right: 0,
        };
        let got = apply_rules(device, &rules(floor, false, false));
        assert_eq!(
            got,
            Insets {
                top: 32,
                left: 108,
                bottom: 60,
                right: 108
            }
        );
    }

    #[test]
    fn symmetric_sides_take_the_larger() {
        let device = Insets {
            top: 0,
            left: 0,
            bottom: 0,
            right: 140,
        };
        let got = apply_rules(device, &rules(ZERO, true, false));
        assert_eq!((got.left, got.right), (140, 140));
    }

    #[test]
    fn ignoring_the_bottom_inset_keeps_only_the_floor() {
        let floor = Insets { bottom: 8, ..ZERO };
        let device = Insets { bottom: 34, ..ZERO };
        assert_eq!(apply_rules(device, &rules(floor, false, true)).bottom, 8);
        assert_eq!(apply_rules(device, &rules(floor, false, false)).bottom, 34);
    }
}
