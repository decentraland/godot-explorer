//! Entities parented to the player or the camera must not collide with them: the colliders under
//! such an entity lose the PHYSICS layer (raycasts keep working) while the entity is attached.

use godot::classes::{CollisionObject3D, INode, Node};
use godot::prelude::*;

use crate::godot_classes::dcl_scene_node::DclSceneNode;

/// Seconds after which a collider returns to its own physics layer once it left the filter.
const TIME_TO_RETURN_BACK_LAYER: f64 = 0.5;
/// The physics layer bit removed from filtered colliders.
const PHYSICS_LAYER_MASK: u32 = 0xFFFFFFFD;

/// The colliders currently filtered under one entity.
#[derive(Default)]
pub struct ColliderFilter {
    computed: Vec<Gd<Node>>,
}

impl ColliderFilter {
    /// Filters what is new under `parent` and gives what left the filter its layer back later.
    pub fn rescan(&mut self, parent: &Gd<Node>, owner: &Gd<Node>) {
        let mut current = Vec::new();
        collect_colliders(parent, &mut current);
        self.update(current, owner);
    }

    /// Releases every filtered collider (the filter node is leaving).
    pub fn release_all(&mut self, owner: &Gd<Node>) {
        self.update(Vec::new(), owner);
    }

    fn update(&mut self, current: Vec<Gd<Node>>, owner: &Gd<Node>) {
        let mut left_over = self.computed.clone();
        for collider in &current {
            match left_over
                .iter()
                .position(|c| c.instance_id() == collider.instance_id())
            {
                Some(index) => {
                    left_over.remove(index);
                }
                None => {
                    if let Ok(mut body) = collider.clone().try_cast::<CollisionObject3D>() {
                        let layer = body.get_collision_layer() & PHYSICS_LAYER_MASK;
                        body.set_collision_layer(layer);
                    }
                }
            }
        }
        restore_after_delay(left_over, owner);
        self.computed = current;
    }
}

fn restore_after_delay(colliders: Vec<Gd<Node>>, owner: &Gd<Node>) {
    if colliders.is_empty() {
        return;
    }
    let Some(mut tree) = owner.get_tree_or_null() else {
        restore_layers(&colliders);
        return;
    };
    let timer = tree.create_timer(TIME_TO_RETURN_BACK_LAYER);
    timer
        .signals()
        .timeout()
        .connect(move || restore_layers(&colliders));
}

fn restore_layers(colliders: &[Gd<Node>]) {
    for collider in colliders {
        if !collider.is_instance_valid() {
            continue;
        }
        if let Ok(mut body) = collider.clone().try_cast::<CollisionObject3D>() {
            if body.has_meta("dcl_col") {
                // Moving between filtered owners (avatar attach, player or camera entity) is not
                // handled: the filter is never still needed.
                let layer: u32 = body.get_meta("dcl_col").try_to().unwrap_or(0);
                body.set_collision_layer(layer);
            }
        }
    }
}

/// The nodes under `node` that carry the `dcl_col` meta (the scene's physics colliders).
fn collect_colliders(node: &Gd<Node>, out: &mut Vec<Gd<Node>>) {
    for child in node.get_children().iter_shared() {
        collect_colliders(&child, out);
        if child.has_meta("dcl_col") {
            out.push(child);
        }
    }
}

/// Connects `owner` to its scene's tree changes, so the filter follows the entity's colliders.
pub fn watch_scene(owner: &Gd<Node>, parent: &Gd<Node>, method: &str) {
    if let Some(scene) = parent
        .clone()
        .try_cast::<Node3D>()
        .ok()
        .and_then(|p| DclSceneNode::search(&p))
    {
        let on_changed = owner.callable(method);
        scene
            .clone()
            .upcast::<Object>()
            .connect("tree_changed", &on_changed);
    }
}

/// The filter as a node of the player and camera entities.
#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct PlayerColliderFilter {
    filter: ColliderFilter,
    base: Base<Node>,
}

#[godot_api]
impl INode for PlayerColliderFilter {
    fn exit_tree(&mut self) {
        let owner = self.base().clone();
        self.filter.release_all(&owner);
    }
}

#[godot_api]
impl PlayerColliderFilter {
    /// Starts filtering: call once the node is under its entity.
    #[func]
    pub fn init_player_collider_filter(&mut self) {
        let Some(parent) = self.base().get_parent() else {
            return;
        };
        let owner = self.base().clone();
        watch_scene(&owner, &parent, "on_tree_changed");
        self.filter.rescan(&parent, &owner);
    }

    #[func]
    fn on_tree_changed(&mut self) {
        // During scene teardown this node can outlive its parent: tree_changed still fires
        // after removal, and the parent is gone then.
        let Some(parent) = self.base().get_parent() else {
            return;
        };
        let owner = self.base().clone();
        self.filter.rescan(&parent, &owner);
    }
}
