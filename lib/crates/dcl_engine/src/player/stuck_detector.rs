use std::collections::HashMap;

use godot::classes::{
    Area3D, CollisionObject3D, CollisionShape3D, IArea3D, PhysicsShapeQueryParameters3D,
};
use godot::prelude::*;

/// CL_PHYSICS, the layer the player collides with.
const CL_PHYSICS: u32 = 2;
const MAX_RESULTS: i32 = 32;

/// Detects when the player teleports inside a collider and temporarily disables the collider's
/// physics layer until the player exits.
#[derive(GodotClass)]
#[class(init, base=Area3D)]
pub struct StuckDetector {
    base: Base<Area3D>,
    /// Disabled colliders and the collision layer each had.
    disabled: HashMap<InstanceId, u32>,
}

#[godot_api]
impl IArea3D for StuckDetector {
    fn ready(&mut self) {
        self.base_mut().set_collision_layer(0); // not detected
                                                // Every layer, so colliders can be tracked after CL_PHYSICS is disabled on them.
        self.base_mut().set_collision_mask(0xFFFF_FFFF);
        self.base_mut().set_monitoring(true);
        self.base_mut().set_monitorable(false);
        let this = self.to_gd();
        self.base_mut().signals().body_exited().connect_other(
            &this,
            |this: &mut Self, body: Gd<Node3D>| {
                this.on_body_exited(body.instance_id());
            },
        );
    }

    fn physics_process(&mut self, _delta: f64) {
        if self.disabled.is_empty() {
            return;
        }
        // Restore CL_PHYSICS on every disabled collider so the query can detect them.
        for (id, layer) in &self.disabled {
            if let Some(mut collider) = collider_of(*id) {
                collider.set_collision_layer(*layer);
            }
        }
        let overlapping = self.overlapping_colliders();

        // Re-enable the colliders the player has exited, keep the others disabled.
        let ids: Vec<InstanceId> = self.disabled.keys().copied().collect();
        for id in ids {
            if overlapping.contains(&id) {
                if let Some(mut collider) = collider_of(id) {
                    let layer = collider.get_collision_layer();
                    collider.set_collision_layer(layer & !CL_PHYSICS);
                }
            } else {
                self.enable(id);
            }
        }
    }
}

#[godot_api]
impl StuckDetector {
    /// Called after a teleport to detect overlapping colliders.
    #[func]
    pub fn check_stuck(&mut self) {
        for id in self.overlapping_colliders() {
            self.disable(id);
        }
    }
}

fn collider_of(id: InstanceId) -> Option<Gd<CollisionObject3D>> {
    Gd::<CollisionObject3D>::try_from_instance_id(id).ok()
}

impl StuckDetector {
    fn overlapping_colliders(&self) -> Vec<InstanceId> {
        let Some(world) = self.base().get_world_3d() else {
            return Vec::new();
        };
        let Some(mut space) = world.get_direct_space_state() else {
            return Vec::new();
        };
        let Some(body) = self
            .base()
            .try_get_node_as::<CollisionShape3D>("CollisionShape3D_Body")
        else {
            return Vec::new();
        };
        let mut query = PhysicsShapeQueryParameters3D::new_gd();
        if let Some(shape) = body.get_shape() {
            query.set_shape(&shape);
        }
        query.set_transform(self.base().get_global_transform() * body.get_transform());
        query.set_collision_mask(CL_PHYSICS);
        query.set_collide_with_bodies(true);
        query.set_collide_with_areas(false);
        space
            .intersect_shape_ex(&query)
            .max_results(MAX_RESULTS)
            .done()
            .iter_shared()
            .filter_map(|result| result.get("collider"))
            .filter_map(|collider| collider.try_to::<Gd<Object>>().ok())
            .map(|collider| collider.instance_id())
            .collect()
    }

    fn disable(&mut self, id: InstanceId) {
        if self.disabled.contains_key(&id) {
            return;
        }
        let Some(mut collider) = collider_of(id) else {
            return;
        };
        let layer = collider.get_collision_layer();
        self.disabled.insert(id, layer);
        // Disable the CL_PHYSICS bit (layer 2, bit index 1).
        collider.set_collision_layer(layer & !CL_PHYSICS);
        let this = self.to_gd();
        collider
            .signals()
            .tree_exiting()
            .connect_other(&this, move |this: &mut Self| this.forget(id));
    }

    fn enable(&mut self, id: InstanceId) {
        if let Some(layer) = self.disabled.remove(&id) {
            if let Some(mut collider) = collider_of(id) {
                collider.set_collision_layer(layer);
            }
        }
    }

    fn forget(&mut self, id: InstanceId) {
        self.disabled.remove(&id);
    }

    fn on_body_exited(&mut self, id: InstanceId) {
        if self.disabled.contains_key(&id) {
            self.enable(id);
        }
    }
}
