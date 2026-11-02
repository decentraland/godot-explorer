//! Draws the scenes' raycasts as fading lines, for debugging (`--raycast-debugger`).

use std::collections::HashMap;

use godot::classes::{BaseMaterial3D, INode, MeshInstance3D, Node, PackedScene};
use godot::prelude::*;

const LINE_SCENE: &str = "res://src/tool/raycast_debugger/raycast_line.tscn";
/// Alpha lost per second.
const FADE_PER_SECOND: f32 = 0.01;

#[derive(GodotClass)]
#[class(base=Node, init)]
pub struct RaycastDebugger {
    base: Base<Node>,
    lines: HashMap<i64, Gd<Node3D>>,
}

fn material_of(line: &Gd<Node3D>) -> Option<Gd<BaseMaterial3D>> {
    line.try_get_node_as::<MeshInstance3D>("MeshInstance3D")?
        .get_active_material(0)?
        .try_cast::<BaseMaterial3D>()
        .ok()
}

/// The alpha after fading for `delta` seconds, floored at 0.
pub fn faded_alpha(alpha: f32, delta: f32) -> f32 {
    (alpha - delta * FADE_PER_SECOND).max(0.0)
}

#[godot_api]
impl INode for RaycastDebugger {
    fn process(&mut self, delta: f64) {
        let mut done = Vec::new();
        for (id, line) in &self.lines {
            let Some(mut material) = material_of(line) else {
                continue;
            };
            let mut color = material.get_albedo();
            color.a = faded_alpha(color.a, delta as f32);
            material.set_albedo(color);
            if color.a <= 0.0 {
                done.push(*id);
            }
        }
        for id in done {
            self.remove(id);
        }
    }
}

impl RaycastDebugger {
    fn remove(&mut self, id: i64) {
        if let Some(mut line) = self.lines.remove(&id) {
            if line.is_instance_valid() {
                line.queue_free();
            }
        }
    }

    /// Shows (or refreshes) the line of ray `id` from `from` to `to`, starting at alpha `time`.
    pub fn add_raycast(&mut self, id: i64, time: f32, from: Vector3, to: Vector3) {
        if !self.lines.contains_key(&id) {
            let Some(mut line) = godot::tools::load::<PackedScene>(LINE_SCENE)
                .instantiate()
                .and_then(|n| n.try_cast::<Node3D>().ok())
            else {
                return;
            };
            line.set_name(&format!("raycast_debug_{id}"));
            self.base_mut().add_child(&line);
            self.lines.insert(id, line);
        }
        let Some(line) = self.lines.get_mut(&id) else {
            return;
        };
        if let Some(mut material) = material_of(line) {
            let mut color = material.get_albedo();
            color.a = time;
            material.set_albedo(color);
        }
        let diff = to - from;
        line.set_global_position(from);
        line.set_scale(Vector3::new(1.0, 1.0, -diff.length()));
        let dir = diff.normalized();
        if dir.abs() == Vector3::UP {
            line.set_rotation(Vector3::new(
                std::f32::consts::FRAC_PI_2 * diff.y.signum(),
                0.0,
                0.0,
            ));
        } else {
            line.look_at(to);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::faded_alpha;

    #[test]
    fn alpha_fades_and_stops_at_zero() {
        assert!((faded_alpha(1.0, 10.0) - 0.9).abs() < 1e-6);
        assert_eq!(faded_alpha(0.05, 10.0), 0.0);
    }
}
