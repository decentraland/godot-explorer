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

/// The own_fade value at `distance` from the head: 0 outside the start, 1 inside the gone one.
pub fn fade_for_distance(distance: f32) -> f32 {
    ((FADE_START_DISTANCE - distance) / (FADE_START_DISTANCE - FADE_GONE_DISTANCE)).clamp(0.0, 1.0)
}

#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct AvatarProximityFade {
    meshes: Vec<Gd<MeshInstance3D>>,
    last_fade: Option<f32>,
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
        let Some(avatar) = self
            .base()
            .get_parent()
            .and_then(|p| p.try_cast::<Node3D>().ok())
        else {
            return;
        };
        // Only avatars in the world fade: previews live in SubViewports with their own cameras.
        let Some(viewport) = self.base().get_viewport() else {
            return;
        };
        let in_world = self
            .base()
            .get_tree_or_null()
            .and_then(|t| t.get_root())
            .is_some_and(|root| root.instance_id() == viewport.instance_id());
        if !in_world {
            return;
        }
        let Some(camera) = viewport.get_camera_3d() else {
            return;
        };
        let head = avatar.get_global_position() + Vector3::new(0.0, HEAD_HEIGHT, 0.0);
        let fade = fade_for_distance(camera.get_global_position().distance_to(head));
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
}

#[godot_api]
impl AvatarProximityFade {
    #[func]
    fn recollect_deferred(&mut self) {
        self.run_deferred(|this| this.collect_meshes());
    }

    #[func]
    fn recollect_on_child(&mut self, _child: Gd<Node>) {
        self.run_deferred(|this| this.collect_meshes());
    }

    #[func]
    fn collect_meshes(&mut self) {
        self.meshes.clear();
        if let Some(avatar) = self.base().get_parent() {
            self.walk(&avatar);
        }
    }
}

impl AvatarProximityFade {
    fn walk(&mut self, node: &Gd<Node>) {
        for child in node.get_children().iter_shared() {
            if let Ok(mesh) = child.clone().try_cast::<MeshInstance3D>() {
                if supports_own_fade(&mesh) {
                    self.meshes.push(mesh);
                }
            }
            self.walk(&child);
        }
    }
}

fn material_supports_own_fade(material: Option<Gd<godot::classes::Material>>) -> bool {
    let Some(material) = material else {
        return false;
    };
    let Ok(shader_material) = material.try_cast::<godot::classes::ShaderMaterial>() else {
        return false;
    };
    shader_material
        .get_shader()
        .is_some_and(|shader| shader.get_code().to_string().contains("own_fade"))
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
