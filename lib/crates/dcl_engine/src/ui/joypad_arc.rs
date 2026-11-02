//! Places the joypad's satellite buttons on an arc around the central button: the first sits
//! tangent to the joypad's bottom edge, the last to the right edge, the rest evenly between.
//! Only the three diameters drive it, so nothing is tuned per button count.

use godot::classes::{Control, IControl};
use godot::prelude::*;

/// The orbit's end angles in radians, sweeping counter-clockwise through the upper left.
pub fn endpoint_angles(
    press_diameter: f32,
    satellite_diameter: f32,
    center_distance: f32,
) -> (f32, f32) {
    // An endpoint node touches the edge the central button touches, so its center sits
    // `press_r - sat_r` inside that edge.
    let edge = press_diameter * 0.5 - satellite_diameter * 0.5;
    let span = (center_distance * center_distance - edge * edge)
        .max(0.0)
        .sqrt();
    let first = edge.atan2(-span);
    let mut last = (-span).atan2(edge);
    if last < first {
        last += std::f32::consts::TAU;
    }
    (first, last)
}

/// The angle of node `index` of `count`: a fixed step anchored at the bottom, so fewer nodes do
/// not stretch across the arc and more compress to fit between the edges.
pub fn node_angle(
    index: usize,
    count: usize,
    first: f32,
    last: f32,
    full_arc_count: usize,
    single_node_t: f32,
) -> f32 {
    if count == 1 {
        return first + (last - first) * single_node_t;
    }
    let slots = count.max(full_arc_count);
    first + (last - first) / (slots - 1) as f32 * index as f32
}

#[derive(GodotClass)]
#[class(base=Control, init)]
pub struct JoypadArc {
    base: Base<Control>,
    /// The central button's diameter: its center is half of it from the corner edges.
    #[export]
    #[init(val = 156.0)]
    press_diameter: f32,
    #[export]
    #[init(val = 80.0)]
    satellite_diameter: f32,
    /// The orbit radius, center to center.
    #[export]
    #[init(val = 167.0)]
    center_distance: f32,
    /// Where a lone satellite sits along the arc: 0 bottom, 1 right.
    #[export(range = (0.0, 1.0))]
    #[init(val = 0.5)]
    single_node_t: f32,
    /// The count at which the arc reaches the right edge.
    #[export]
    #[init(val = 3)]
    full_arc_count: i32,
}

#[godot_api]
impl IControl for JoypadArc {}

#[godot_api]
impl JoypadArc {
    /// Positions `nodes` (first at the bottom edge, last at the right) on the arc.
    #[func]
    pub fn arrange(&mut self, nodes: VarArray) {
        let count = nodes.len();
        let Some(press) = self
            .base()
            .get_parent()
            .and_then(|p| p.try_cast::<Control>().ok())
        else {
            return;
        };
        if count == 0 {
            return;
        }
        let (first, last) = endpoint_angles(
            self.press_diameter,
            self.satellite_diameter,
            self.center_distance,
        );
        let center = press.get_global_position() + press.get_size() * 0.5
            - self.base().get_global_position();
        let full = self.full_arc_count.max(2) as usize;
        for (i, node) in nodes.iter_shared().enumerate() {
            let Ok(mut node) = node.try_to::<Gd<Control>>() else {
                continue;
            };
            let theta = node_angle(i, count, first, last, full, self.single_node_t);
            let node_center =
                center + Vector2::new(theta.cos(), theta.sin()) * self.center_distance;
            let size = node.get_size();
            node.set_position(node_center - size * 0.5);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_arc_runs_from_the_bottom_to_the_right_edge() {
        let (first, last) = endpoint_angles(156.0, 80.0, 167.0);
        // Down and to the left, then up and to the right through the upper left.
        assert!(first > 2.5 && first < 3.2, "{first}");
        assert!(last > first);
        assert!(last > 4.5 && last < 5.0, "{last}");
    }

    #[test]
    fn nodes_keep_a_fixed_step_until_the_arc_is_full() {
        let (first, last) = endpoint_angles(156.0, 80.0, 167.0);
        let two = node_angle(1, 2, first, last, 3, 0.5);
        let three = node_angle(1, 3, first, last, 3, 0.5);
        assert!((two - three).abs() < 1e-6);
        assert!((node_angle(2, 3, first, last, 3, 0.5) - last).abs() < 1e-5);
        // Four compress between the same edges.
        assert!((node_angle(3, 4, first, last, 3, 0.5) - last).abs() < 1e-5);
    }

    #[test]
    fn a_lone_node_sits_at_the_chosen_point() {
        assert_eq!(node_angle(0, 1, 1.0, 3.0, 3, 0.0), 1.0);
        assert_eq!(node_angle(0, 1, 1.0, 3.0, 3, 0.5), 2.0);
    }
}
