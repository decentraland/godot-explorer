use godot::classes::{
    sub_viewport::UpdateMode, Camera3D, INode3D, MeshInstance3D, Node3D, ShaderMaterial,
    SubViewport,
};
use godot::prelude::*;

/// Layer 20 holds outlined objects (bit 19, value 524288).
const OUTLINE_LAYER: u32 = 20;

/// The outline is suppressed while the outlined entity's surface is this close to the camera: at
/// point-blank range the object fills the view and its outline is screen-flooding noise.
const OUTLINE_SUPPRESS_DISTANCE: f32 = 0.4;

#[derive(GodotClass)]
#[class(init, base=Node3D)]
pub struct OutlineSystem {
    base: Base<Node3D>,
    main_camera: Option<Gd<Camera3D>>,
    current_outlined_avatar: Option<Gd<Node3D>>,
    current_outlined_entity: Option<Gd<Node3D>>,
    /// Local-space merged AABB of the outlined entity, cached on target change so the per-frame
    /// suppression check is a cheap transform plus box distance.
    entity_local_aabb: Aabb,
    entity_has_aabb: bool,
    /// Only pushed to the shader on size changes.
    last_pushed_viewport_size: Vector2i,
}

#[godot_api]
impl INode3D for OutlineSystem {
    fn process(&mut self, _delta: f64) {
        let Some(camera) = self.main_camera.clone() else {
            return;
        };
        let (Some(mut sub_viewport), Some(mut depth_camera)) = (
            self.base().try_get_node_as::<SubViewport>("SubViewport"),
            self.base()
                .try_get_node_as::<Camera3D>("SubViewport/DepthCamera"),
        ) else {
            return;
        };
        let outline_quad = self.base().try_get_node_as::<MeshInstance3D>("OutlineQuad");

        // Match the viewport size to the window size.
        let window_size = camera
            .get_viewport()
            .map(|viewport| viewport.get_visible_rect().size)
            .unwrap_or(Vector2::ZERO);
        let window_size_i = Vector2i::new(window_size.x as i32, window_size.y as i32);
        if sub_viewport.get_size() != window_size_i {
            sub_viewport.set_size(window_size_i);
        }

        // Push the viewport size to the outline shader so it doesn't depend on
        // textureSize(extracted_texture, 0), which returns inconsistent values in Godot 4.6 for
        // ViewportTextures inside SubViewports with non-default scaling_3d_scale (e.g. the avatar
        // preview's 2.0).
        if let Some(quad) = &outline_quad {
            if window_size_i != self.last_pushed_viewport_size {
                if let Some(material) = quad
                    .get_material_override()
                    .and_then(|m| m.try_cast::<ShaderMaterial>().ok())
                {
                    let mut material = material;
                    material.set_shader_parameter(
                        "viewport_size",
                        &Vector2::new(window_size_i.x as f32, window_size_i.y as f32).to_variant(),
                    );
                    self.last_pushed_viewport_size = window_size_i;
                }
            }
        }

        // Sync the depth camera with the main camera (including the ortho size for the avatar
        // preview).
        depth_camera.set_global_transform(camera.get_global_transform());
        depth_camera.set_projection(camera.get_projection());
        depth_camera.set_fov(camera.get_fov());
        depth_camera.set_size(camera.get_size());
        depth_camera.set_near(camera.get_near());
        depth_camera.set_far(camera.get_far());

        // Proximity suppression: an outlined entity whose surface is inside the band would flood
        // the Sobel with edges. (Avatars don't need this: their materials have no scene fade.)
        let active = (self.current_outlined_avatar.is_some()
            || self.current_outlined_entity.is_some())
            && !self.is_entity_outline_suppressed(&camera);
        if let Some(mut quad) = outline_quad {
            if quad.is_visible() != active {
                quad.set_visible(active);
            }
        }
        let mode = if active {
            UpdateMode::ALWAYS
        } else {
            UpdateMode::DISABLED
        };
        if sub_viewport.get_update_mode() != mode {
            sub_viewport.set_update_mode(mode);
        }
    }

    fn exit_tree(&mut self) {
        if let Some(avatar) = self.current_outlined_avatar.clone() {
            if avatar.is_instance_valid() {
                set_avatar_layers(&avatar, false);
            }
        }
        if let Some(entity) = self.current_outlined_entity.clone() {
            if entity.is_instance_valid() {
                set_layers_recursive(&entity.upcast(), false);
            }
        }
    }
}

#[godot_api]
impl OutlineSystem {
    #[func]
    pub fn setup(&mut self, camera: Gd<Camera3D>) {
        self.main_camera = Some(camera);
        if let Some(mut depth_quad) = self
            .base()
            .try_get_node_as::<MeshInstance3D>("SubViewport/DepthCamera/DepthQuad")
        {
            depth_quad.set_visible(true);
        }
        if let Some(mut quad) = self.base().try_get_node_as::<MeshInstance3D>("OutlineQuad") {
            quad.set_visible(false);
        }
        if let Some(mut sub_viewport) = self.base().try_get_node_as::<SubViewport>("SubViewport") {
            sub_viewport.set_update_mode(UpdateMode::DISABLED);
        }
    }

    #[func]
    pub fn set_outlined_avatar(&mut self, avatar: Option<Gd<Node3D>>) {
        if let Some(previous) = self.current_outlined_avatar.take() {
            if previous.is_instance_valid() {
                set_avatar_layers(&previous, false);
            }
        }
        if let Some(avatar) = &avatar {
            set_avatar_layers(avatar, true);
        }
        self.current_outlined_avatar = avatar;
        self.update_effect_visibility();
    }

    #[func]
    pub fn set_outlined_entity(&mut self, entity: Option<Gd<Node3D>>) {
        if self.current_outlined_entity == entity {
            return;
        }
        if let Some(previous) = self.current_outlined_entity.take() {
            if previous.is_instance_valid() {
                set_layers_recursive(&previous.upcast(), false);
            }
        }
        self.entity_has_aabb = false;
        if let Some(entity) = &entity {
            set_layers_recursive(&entity.clone().upcast(), true);
            self.entity_local_aabb = compute_local_aabb(entity);
            self.entity_has_aabb = true;
        }
        self.current_outlined_entity = entity;
        self.update_effect_visibility();
    }
}

impl OutlineSystem {
    fn is_entity_outline_suppressed(&self, camera: &Gd<Camera3D>) -> bool {
        let Some(entity) = &self.current_outlined_entity else {
            return false;
        };
        if !self.entity_has_aabb || !entity.is_instance_valid() {
            return false;
        }
        let world_aabb = entity.get_global_transform() * self.entity_local_aabb;
        aabb_surface_distance(camera.get_global_position(), world_aabb) < OUTLINE_SUPPRESS_DISTANCE
    }

    /// The outline post-process is shared by the avatar and entity paths; it keeps rendering
    /// while either has a target (the crosshair hits only one at a time).
    fn update_effect_visibility(&mut self) {
        let active =
            self.current_outlined_avatar.is_some() || self.current_outlined_entity.is_some();
        if let Some(mut quad) = self.base().try_get_node_as::<MeshInstance3D>("OutlineQuad") {
            quad.set_visible(active);
        }
        if let Some(mut sub_viewport) = self.base().try_get_node_as::<SubViewport>("SubViewport") {
            sub_viewport.set_update_mode(if active {
                UpdateMode::ALWAYS
            } else {
                UpdateMode::DISABLED
            });
        }
    }
}

/// Merged AABB of every MeshInstance3D under `root`, in `root`'s local space.
pub fn compute_local_aabb(root: &Gd<Node3D>) -> Aabb {
    let mut result: Option<Aabb> = None;
    let mut stack: Vec<Gd<Node>> = vec![root.clone().upcast()];
    let inverse_root = root.get_global_transform().affine_inverse();
    while let Some(node) = stack.pop() {
        if let Ok(mesh_instance) = node.clone().try_cast::<MeshInstance3D>() {
            if mesh_instance.get_mesh().is_some() {
                let local = (inverse_root * mesh_instance.get_global_transform())
                    * mesh_instance.get_aabb();
                result = Some(match result {
                    Some(merged) => merged.merge(local),
                    None => local,
                });
            }
        }
        stack.extend(node.get_children().iter_shared());
    }
    result.unwrap_or_default()
}

/// Distance from a point to the closest point on the AABB (0 when inside).
pub fn aabb_surface_distance(point: Vector3, aabb: Aabb) -> f32 {
    let end = aabb.position + aabb.size;
    let closest = Vector3::new(
        point.x.clamp(aabb.position.x, end.x),
        point.y.clamp(aabb.position.y, end.y),
        point.z.clamp(aabb.position.z, end.z),
    );
    closest.distance_to(point)
}

fn set_avatar_layers(avatar: &Gd<Node3D>, add_outline: bool) {
    // The outline goes on the meshes of the avatar's skeleton.
    if let Some(skeleton) = avatar
        .find_child_ex("Skeleton3D")
        .recursive(true)
        .owned(false)
        .done()
    {
        set_layers_recursive(&skeleton, add_outline);
    }
}

fn set_layers_recursive(node: &Gd<Node>, add_outline: bool) {
    let bit = 1u32 << (OUTLINE_LAYER - 1);
    for child in node.get_children().iter_shared() {
        match child.clone().try_cast::<MeshInstance3D>() {
            Ok(mut mesh) => {
                let layers = mesh.get_layer_mask();
                mesh.set_layer_mask(if add_outline {
                    layers | bit
                } else {
                    layers & !bit
                });
            }
            Err(child) => set_layers_recursive(&child, add_outline),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aabb(position: Vector3, size: Vector3) -> Aabb {
        Aabb { position, size }
    }

    #[test]
    fn a_point_inside_the_box_is_at_distance_zero() {
        let box_ = aabb(Vector3::new(-1.0, -1.0, -1.0), Vector3::new(2.0, 2.0, 2.0));
        assert_eq!(aabb_surface_distance(Vector3::ZERO, box_), 0.0);
    }

    #[test]
    fn the_distance_is_to_the_closest_face_edge_or_corner() {
        let box_ = aabb(Vector3::ZERO, Vector3::new(1.0, 1.0, 1.0));
        assert!((aabb_surface_distance(Vector3::new(3.0, 0.5, 0.5), box_) - 2.0).abs() < 1e-6);
        let corner = aabb_surface_distance(Vector3::new(2.0, 2.0, 1.0), box_);
        assert!((corner - 2.0_f32.sqrt()).abs() < 1e-6);
    }

    #[test]
    fn the_suppression_band_is_point_blank() {
        let box_ = aabb(Vector3::ZERO, Vector3::new(1.0, 1.0, 1.0));
        assert!(
            aabb_surface_distance(Vector3::new(1.3, 0.5, 0.5), box_) < OUTLINE_SUPPRESS_DISTANCE
        );
        assert!(
            aabb_surface_distance(Vector3::new(1.5, 0.5, 0.5), box_) >= OUTLINE_SUPPRESS_DISTANCE
        );
    }
}
