//! The one ticker of every light: fades each frame, and recomputes range and budget state when
//! the player moved or a light asked for it.

use godot::classes::{INode, Node};
use godot::prelude::*;

use super::component::{
    is_recompute_pending, registered_lights, set_view, settings, take_recompute_pending,
};
use super::logic::rank_by_distance;
use crate::godot_classes::dcl_global::DclGlobal;

const UPDATE_INTERVAL: f32 = 0.25;

#[derive(GodotClass)]
#[class(base=Node)]
pub struct DclLightSourceManager {
    base: Base<Node>,
    update_timer: f32,
}

#[godot_api]
impl INode for DclLightSourceManager {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            update_timer: 0.0,
        }
    }

    fn process(&mut self, delta: f64) {
        let delta = delta as f32;
        for mut light in registered_lights() {
            if light.bind().is_fading() {
                light.bind_mut().tick_fade(delta);
            }
        }

        self.update_timer -= delta;
        let due = self.update_timer <= 0.0;
        if !due && !is_recompute_pending() {
            return;
        }
        if due {
            self.update_timer = UPDATE_INTERVAL;
        }
        take_recompute_pending();
        set_view(reference_position(), camera_view());
        update_budget();
    }
}

fn reference_position() -> Option<Vector3> {
    crate::world::state::player()
        .filter(|p| p.is_instance_valid())
        .map(|p| p.get_global_position())
}

fn camera_view() -> Option<(Vector3, Vector3)> {
    let camera = DclGlobal::singleton()
        .bind()
        .player_camera_node
        .clone()
        .filter(|c| c.is_instance_valid())?;
    let forward = -camera.get_global_transform().basis.col_c().normalized();
    Some((camera.get_global_position(), forward))
}

fn update_budget() {
    let lights = registered_lights();
    if !settings().use_budget {
        for mut light in lights {
            light.bind_mut().open_budget();
        }
        return;
    }
    let mut candidates: Vec<(usize, f32)> = Vec::new();
    for (index, light) in lights.iter().enumerate() {
        if let Some(distance) = light.clone().bind_mut().begin_budget_pass() {
            candidates.push((index, distance));
        }
    }
    for (index, rank, enabled) in rank_by_distance(&candidates, settings().max_lights) {
        lights[index]
            .clone()
            .bind_mut()
            .set_budget(Some(rank), enabled);
    }
    if settings().debug_gizmo {
        for mut light in lights {
            if light.bind().has_gizmo() {
                light.bind_mut().refresh_gizmo();
            }
        }
    }
}
