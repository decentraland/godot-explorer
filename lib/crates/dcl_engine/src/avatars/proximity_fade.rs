//! Object-level proximity fade for avatars (#1814). When the camera gets within arm's reach of
//! an avatar's head, the whole avatar dissolves as one object instead of the camera seeing
//! inside-out head geometry.
//!
//! A per-instance shader parameter `own_fade` drives a dither discard in the avatar shaders.
//! `GeometryInstance3D.transparency` is not used: the mobile renderer ignores it for opaque
//! materials. Only the world camera drives it; previews use close-up cameras of their own.

use godot::classes::{INode, MeshInstance3D, Node};
use godot::prelude::*;

/// Distance from the head at which the avatar starts fading out.
const FADE_START_DISTANCE: f32 = 0.6;
/// Distance at which the avatar is fully transparent.
const FADE_GONE_DISTANCE: f32 = 0.3;
/// Head height above the avatar origin: close to the local player's Mount pivot for any avatar.
const HEAD_HEIGHT: f32 = 1.7;
/// Remote avatars farther than this stop polling for a while: the camera cannot close the gap
/// faster than MAX_APPROACH_SPEED, and the sleep is capped at MAX_SLEEP_SECONDS.
const SLEEP_DISTANCE: f32 = 4.0;
const MAX_APPROACH_SPEED: f32 = 20.0;
const MAX_SLEEP_SECONDS: f32 = 0.5;

/// The own_fade value at `distance` from the head: 0 outside the start, 1 inside the gone one.
pub fn fade_for_distance(distance: f32) -> f32 {
    ((FADE_START_DISTANCE - distance) / (FADE_START_DISTANCE - FADE_GONE_DISTANCE)).clamp(0.0, 1.0)
}

#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct AvatarProximityFade {
    meshes: Vec<Gd<MeshInstance3D>>,
    last_fade: Option<f32>,
    /// Floor for the fade, set by the GPU-ready gate while the renderer warms the pipelines.
    hold: f32,
    /// A deferred recollect is already queued: every child added in the same frame shares it.
    recollect_pending: bool,
    base: Base<Node>,
}

#[godot_api]
impl INode for AvatarProximityFade {
    fn ready(&mut self) {
        self.collect_meshes();
        // Wearables and body shapes load dynamically: collect again on changes.
        if let Some(avatar) = self.base().get_parent() {
            let this = self.to_gd();
            if let Ok(avatar) = avatar
                .clone()
                .try_cast::<crate::godot_classes::dcl_avatar::DclAvatar>()
            {
                avatar
                    .signals()
                    .avatar_loaded()
                    .connect_other(&this, |this: &mut Self| this.recollect_deferred());
            }
            avatar
                .signals()
                .child_entered_tree()
                .connect_other(&this, |this: &mut Self, _child| this.recollect_deferred());
        }
    }

    fn process(&mut self, _delta: f64) {
        let fade = self.camera_fade(true);
        self.apply(fade.max(self.hold));
    }
}

#[godot_api]
impl AvatarProximityFade {
    #[func]
    fn recollect_deferred(&mut self) {
        if self.recollect_pending {
            return;
        }
        self.recollect_pending = true;
        self.run_deferred(|this| {
            this.recollect_pending = false;
            this.collect_meshes();
        });
    }

    #[func]
    fn collect_meshes(&mut self) {
        self.meshes.clear();
        if let Some(avatar) = self.base().get_parent() {
            // One engine call finds the meshes; a recursive try_cast per node costs a class-name
            // lookup under a global lock each, which dominated the frame.
            let found = avatar
                .find_children_ex("*")
                .type_("MeshInstance3D")
                .recursive(true)
                .owned(false)
                .done();
            for node in found.iter_shared() {
                if let Ok(mesh) = node.try_cast::<MeshInstance3D>() {
                    if supports_own_fade(&mesh) {
                        self.meshes.push(mesh);
                    }
                }
            }
        }
        // New meshes start at own_fade 0: anything else must be written again.
        if self.last_fade != Some(0.0) {
            self.last_fade = None;
        }
    }
}

impl AvatarProximityFade {
    pub fn get_hold(&self) -> f32 {
        self.hold
    }

    /// Keeps the avatar faded by at least `value` whatever the camera distance (0 releases it).
    pub fn set_hold(&mut self, value: f32) {
        if self.hold == 0.0 && value > 0.0 {
            self.collect_meshes();
        }
        self.hold = value;
        let fade = self.camera_fade(false);
        self.apply(fade.max(self.hold));
    }

    /// The fade the world camera asks for. `may_sleep` lets a far remote avatar stop polling.
    fn camera_fade(&mut self, may_sleep: bool) -> f32 {
        let Some(avatar) = self
            .base()
            .get_parent()
            .and_then(|p| p.try_cast::<Node3D>().ok())
        else {
            return 0.0;
        };
        // Only avatars in the world fade: previews live in SubViewports with their own cameras.
        let Some(viewport) = self.base().get_viewport() else {
            return 0.0;
        };
        let in_world = self
            .base()
            .get_tree_or_null()
            .and_then(|t| t.get_root())
            .is_some_and(|root| root.instance_id() == viewport.instance_id());
        if !in_world {
            return 0.0;
        }
        let Some(camera) = viewport.get_camera_3d() else {
            return 0.0;
        };
        let head = avatar.get_global_position() + Vector3::new(0.0, HEAD_HEIGHT, 0.0);
        let distance = camera.get_global_position().distance_to(head);
        let fade = fade_for_distance(distance);
        if may_sleep && fade == 0.0 && self.hold == 0.0 && distance > SLEEP_DISTANCE {
            let local = avatar
                .try_cast::<crate::godot_classes::dcl_avatar::DclAvatar>()
                .is_ok_and(|a| a.bind().is_local_player);
            if !local {
                let seconds =
                    ((distance - FADE_START_DISTANCE) / MAX_APPROACH_SPEED).min(MAX_SLEEP_SECONDS);
                self.sleep(seconds as f64);
            }
        }
        fade
    }

    fn apply(&mut self, fade: f32) {
        if self
            .last_fade
            .is_some_and(|last| (last - fade).abs() < 1e-5)
        {
            return;
        }
        self.last_fade = Some(fade);
        // A wearable swap frees meshes before the next recollect (GODOT-EXPLORER-303).
        self.meshes.retain(|m| m.is_instance_valid());
        for mesh in &mut self.meshes {
            mesh.set_instance_shader_parameter("own_fade", &fade.to_variant());
        }
    }

    fn sleep(&mut self, seconds: f64) {
        self.base_mut().set_process(false);
        let Some(mut tree) = self.base().get_tree_or_null() else {
            return;
        };
        let me = self.to_gd();
        tree.create_timer(seconds)
            .signals()
            .timeout()
            .connect_other(&me, |this: &mut Self| this.base_mut().set_process(true));
    }
}

fn material_supports_own_fade(material: Option<Gd<godot::classes::Material>>) -> bool {
    let Some(material) = material else {
        return false;
    };
    let Ok(shader_material) = material.try_cast::<godot::classes::ShaderMaterial>() else {
        return false;
    };
    let Some(shader) = shader_material.get_shader() else {
        return false;
    };
    // The toon and mask shaders are shared by every avatar: read each source once.
    let id = shader.instance_id().to_i64();
    SHADER_FADE.with(|cache| {
        *cache
            .borrow_mut()
            .entry(id)
            .or_insert_with(|| shader.get_code().to_string().contains("own_fade"))
    })
}

thread_local! {
    static SHADER_FADE: std::cell::RefCell<std::collections::HashMap<i64, bool>> = Default::default();
}

/// Only meshes whose shaders declare the `own_fade` uniform (the toon and mask shaders) can fade
/// per instance: setting it on anything else spams errors (e.g. the glider's StandardMaterial3D).
fn supports_own_fade(mesh: &Gd<MeshInstance3D>) -> bool {
    if material_supports_own_fade(mesh.get_material_override()) {
        return true;
    }
    for i in 0..mesh.get_surface_override_material_count() {
        if material_supports_own_fade(mesh.get_surface_override_material(i)) {
            return true;
        }
    }
    if let Some(resource) = mesh.get_mesh() {
        for i in 0..resource.get_surface_count() {
            if material_supports_own_fade(resource.surface_get_material(i)) {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::fade_for_distance;

    #[test]
    fn fade_ramps_between_the_distances() {
        assert_eq!(fade_for_distance(1.0), 0.0);
        assert_eq!(fade_for_distance(0.6), 0.0);
        assert_eq!(fade_for_distance(0.3), 1.0);
        assert_eq!(fade_for_distance(0.0), 1.0);
        assert!((fade_for_distance(0.45) - 0.5).abs() < 1e-5);
    }
}
