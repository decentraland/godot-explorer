//! The interaction prompts around the crosshair: one pill per pointer event the cursor is over,
//! fanned on a ring when there are several.

use godot::classes::{Control, IControl};
use godot::prelude::*;

use super::tooltip_label::PointerTooltipPill;

/// Horizontal gap between the crosshair and a pill's left edge. It places a lone pill outright and
/// is the floor for the ring: a pill is a wide left-anchored box, not a point.
const MIN_OFFSET_X: f32 = 40.0;
/// Breathing room between stacked pills. The ring radius derives from the pill's measured height
/// so resizing the pill can never reintroduce an overlap.
const PILL_GAP: f32 = 8.0;
const ANGLES: [f32; 5] = [0.0, 60.0, 90.0, 120.0, 180.0];
const PILL_SCENE: &str = "res://src/ui/components/molecules/pointer_tooltip/tooltip_label.tscn";

/// The ring angles used for `count` pills, centered on the ring.
pub fn centered_angles(count: usize) -> &'static [f32] {
    let count = count.min(ANGLES.len());
    let start = ANGLES.len() / 2 - count / 2;
    &ANGLES[start..start + count]
}

/// Where pill `index` of `count` goes: to its center, relative to the crosshair. A lone pill sits
/// beside it; several fan out on a ring whose radius is twice the pill height plus the gap, since
/// neighbors are half a radius apart vertically.
pub fn pill_center_offset(count: usize, index: usize, pill_height: f32) -> Vector2 {
    if count <= 1 {
        return Vector2::new(MIN_OFFSET_X, 0.0);
    }
    let radius = 2.0 * (pill_height + PILL_GAP);
    let angle = centered_angles(count)[index].to_radians();
    let mut offset = Vector2::new(0.0, -radius).rotated(angle);
    offset.x = offset.x.max(MIN_OFFSET_X);
    offset
}

#[derive(GodotClass)]
#[class(base=Control, init)]
pub struct PointerTooltip {
    base: Base<Control>,
    #[init(node = "%Control_Center")]
    center: OnReady<Gd<Control>>,
    #[init(node = "%Panel_Crosshair")]
    crosshair_panel: OnReady<Gd<Control>>,
}

#[godot_api]
impl IControl for PointerTooltip {}

#[godot_api]
impl PointerTooltip {
    #[func]
    pub fn set_global_cursor_position(&mut self, position: Vector2) {
        self.center.set_global_position(position);
        let half = self.crosshair_panel.get_size() / 2.0;
        self.crosshair_panel.set_global_position(position - half);
    }

    /// `interacts` is an array of `{text_pet_down, text_pet_up, action}`.
    #[func]
    pub fn set_pointer_data(&mut self, interacts: VarArray) {
        for mut child in self.center.get_children().iter_shared() {
            child.queue_free();
        }
        let count = interacts.len().min(ANGLES.len());
        let scene = godot::tools::load::<godot::classes::PackedScene>(PILL_SCENE);
        for (index, interact) in interacts.iter_shared().take(count).enumerate() {
            let Ok(interact) = interact.try_to::<VarDictionary>() else {
                continue;
            };
            let Some(mut pill) = scene
                .instantiate()
                .and_then(|n| n.try_cast::<PointerTooltipPill>().ok())
            else {
                continue;
            };
            // The honored minimum, not `size.y`, which only reads 60 because the scene's offsets
            // happen to agree with `custom_minimum_size`.
            let pill_height = pill.get_combined_minimum_size().y;
            let mut offset = pill_center_offset(count, index, pill_height);
            if count > 1 {
                // Neighbors clear each other by only PILL_GAP: a vertical tap ring would overlap
                // one and let a tap fire the wrong prompt. Only a lone pill grows vertically.
                pill.bind_mut().tap_grow_y = 0.0;
            }
            // `set_position` takes the top-left corner; the offsets are to the pill's center.
            offset.y -= pill_height / 2.0;
            pill.set_position(offset);
            self.center.add_child(&pill);
            let text = |key: &str| {
                interact
                    .get(key)
                    .and_then(|v| v.try_to::<GString>().ok())
                    .unwrap_or_default()
            };
            pill.bind_mut().set_tooltip_data(
                text("text_pet_down"),
                text("text_pet_up"),
                text("action"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn angles_center_on_the_ring() {
        assert_eq!(centered_angles(1), &[90.0]);
        assert_eq!(centered_angles(3), &[60.0, 90.0, 120.0]);
        assert_eq!(centered_angles(5).len(), 5);
        assert_eq!(centered_angles(9).len(), 5);
    }

    #[test]
    fn a_lone_pill_sits_beside_the_crosshair() {
        assert_eq!(pill_center_offset(1, 0, 60.0), Vector2::new(40.0, 0.0));
    }

    #[test]
    fn stacked_pills_clear_each_other_vertically() {
        let a = pill_center_offset(3, 0, 60.0);
        let b = pill_center_offset(3, 1, 60.0);
        let c = pill_center_offset(3, 2, 60.0);
        // Neighbors are at least a pill height plus the gap apart.
        assert!((a.y - b.y).abs() >= 60.0 + 8.0 - 1.0, "{a:?} {b:?}");
        assert!((b.y - c.y).abs() >= 60.0 + 8.0 - 1.0, "{b:?} {c:?}");
        assert!(a.x >= 40.0 && b.x >= 40.0 && c.x >= 40.0);
    }
}
