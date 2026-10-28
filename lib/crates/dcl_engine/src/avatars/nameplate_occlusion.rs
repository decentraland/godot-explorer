//! Nameplate occlusion driven by one node: attached avatars sit in OCCLUSION_PERIOD buckets and
//! only plates found on screen are raycast, one bucket per physics step. A plate entering the
//! screen is tested on the next step and held at alpha 0 until then.

use std::cell::RefCell;
use std::collections::HashMap;

use godot::classes::{Engine, INode, Node};
use godot::prelude::*;

use super::nameplate::DEBUG_DISABLE_OCCLUSION;
use crate::godot_classes::dcl_avatar::DclAvatar;

/// Physics steps between occlusion raycasts per avatar.
pub const OCCLUSION_PERIOD: usize = 6;

#[derive(Default)]
struct Schedule {
    buckets: Vec<Vec<Gd<DclAvatar>>>,
    on_screen: HashMap<InstanceId, Gd<DclAvatar>>,
    untested: HashMap<InstanceId, Gd<DclAvatar>>,
}

/// A visible avatar's body as an analytic vertical cylinder, captured once per physics step.
pub struct Occluder {
    pub id: InstanceId,
    pub world: Option<InstanceId>,
    pub origin: Vector3,
    pub radius: f32,
    pub top: f32,
}

thread_local! {
    static SCHEDULE: RefCell<Schedule> = RefCell::new(Schedule::default());
    static OCCLUDERS: RefCell<Vec<Occluder>> = const { RefCell::new(Vec::new()) };
}

fn with<R>(f: impl FnOnce(&mut Schedule) -> R) -> R {
    SCHEDULE.with(|s| f(&mut s.borrow_mut()))
}

/// Puts the avatar in the emptiest bucket.
pub fn add(avatar: Gd<DclAvatar>) {
    with(|s| {
        if s.buckets.is_empty() {
            s.buckets = vec![Vec::new(); OCCLUSION_PERIOD];
        }
        if let Some(bucket) = s.buckets.iter_mut().min_by_key(|b| b.len()) {
            bucket.push(avatar);
        }
    });
}

pub fn remove(id: InstanceId) {
    with(|s| {
        for bucket in &mut s.buckets {
            bucket.retain(|a| a.instance_id() != id);
        }
        s.on_screen.remove(&id);
        s.untested.remove(&id);
    });
}

pub fn set_on_screen(avatar: &Gd<DclAvatar>, on_screen: bool) {
    let id = avatar.instance_id();
    with(|s| {
        if on_screen == s.on_screen.contains_key(&id) {
            return;
        }
        if on_screen {
            s.on_screen.insert(id, avatar.clone());
            s.untested.insert(id, avatar.clone());
        } else {
            s.on_screen.remove(&id);
            s.untested.remove(&id);
        }
    });
}

pub fn is_untested(id: InstanceId) -> bool {
    with(|s| s.untested.contains_key(&id))
}

pub fn with_occluders<R>(f: impl FnOnce(&[Occluder]) -> R) -> R {
    OCCLUDERS.with(|o| f(&o.borrow()))
}

/// Runs the occlusion step every physics frame; a child of the nameplate layer.
#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct NameplateOcclusionDriver {
    base: Base<Node>,
}

#[godot_api]
impl INode for NameplateOcclusionDriver {
    fn physics_process(&mut self, _delta: f64) {
        step();
    }
}

/// The current bucket's on-screen plates plus the plates that just came on screen. Must run in a
/// physics step: the direct space state crashes when queried from an idle frame.
fn step() {
    let disabled = DEBUG_DISABLE_OCCLUSION.load(std::sync::atomic::Ordering::Relaxed);
    let due: Vec<Gd<DclAvatar>> = with(|s| {
        if s.buckets.is_empty() {
            return Vec::new();
        }
        if disabled {
            s.untested.clear();
            return s.buckets.iter().flatten().cloned().collect();
        }
        let mut due: Vec<Gd<DclAvatar>> = s.untested.drain().map(|(_, a)| a).collect();
        let index = Engine::singleton().get_physics_frames() as usize % OCCLUSION_PERIOD;
        for avatar in &s.buckets[index] {
            let id = avatar.instance_id();
            if s.on_screen.contains_key(&id) && !due.iter().any(|a| a.instance_id() == id) {
                due.push(avatar.clone());
            }
        }
        due
    });
    if due.is_empty() {
        return;
    }
    if disabled {
        for mut avatar in due.into_iter().filter(|a| a.is_instance_valid()) {
            avatar.bind_mut().nametag.occluded = false;
        }
        return;
    }
    snapshot_occluders();
    for mut avatar in due {
        if avatar.is_instance_valid() && avatar.is_inside_tree() {
            avatar.bind_mut().nameplate_update_occlusion();
        }
    }
}

/// Every visible avatar (comms avatars, the local player, scene NPCs): radius from its head-zone
/// meshes, top at its computed bounds top (capped).
fn snapshot_occluders() {
    let occluders: Vec<Occluder> = crate::avatars::impostor_capture::registered()
        .into_iter()
        .filter(|a| a.is_visible())
        .map(|avatar| {
            let world = avatar.get_world_3d().map(|w| w.instance_id());
            let id = avatar.instance_id();
            let this = avatar.bind();
            let origin = this.base().get_global_position();
            Occluder {
                id,
                world,
                origin,
                radius: this.occlusion_radius,
                top: this.get_bounds_top_y().min(origin.y + 3.0),
            }
        })
        .collect();
    OCCLUDERS.with(|o| *o.borrow_mut() = occluders);
}
