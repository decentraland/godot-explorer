//! `AvatarAttach`: follows an avatar anchor point (a bone, the position or the name tag) with its
//! parent entity, and keeps the colliders under that entity from pushing the avatar around.

use godot::classes::{INode, Node};
use godot::prelude::*;

use crate::fetcher::globals;
use crate::godot_classes::dcl_avatar::DclAvatar;
use crate::godot_classes::player_collider_filter::{scene_of, ColliderFilter};

/// Anchor ids with a skeleton bone (POSITION 0 and NAME_TAG 1 are not skeletal).
const SKELETAL_ANCHORS: std::ops::RangeInclusive<i32> = 2..=25;

#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct DclAvatarAttach {
    #[var(set = set_user_id)]
    user_id: GString,

    /// `AvatarAnchorPointType` (avatar_attach.proto).
    #[var(set = set_attach_point)]
    attach_point: i32,

    player_avatar: Option<Gd<DclAvatar>>,
    /// The anchor this instance is registered with on `player_avatar`, so the matching
    /// unregister fires even when `attach_point` or `user_id` change mid-flight. -1 when not.
    registered_anchor: i32,

    /// The scene-authored scale: transform_and_parent.rs sets the parent's scale whenever the
    /// SDK Transform updates, and it is adopted when it differs from what was last written.
    scene_scale: Vector3,
    last_written_scale: Vector3,
    scale_initialized: bool,

    filter: ColliderFilter,
    base: Base<Node>,
}

#[godot_api]
impl INode for DclAvatarAttach {
    fn exit_tree(&mut self) {
        self.unregister_from_current_avatar();
        let owner = self.base().clone();
        self.filter.release_all(&owner);
    }

    fn process(&mut self, _delta: f64) {
        self.follow_anchor();
    }
}

#[godot_api]
impl DclAvatarAttach {
    #[func]
    pub fn set_user_id(&mut self, value: GString) {
        if self.user_id == value {
            return;
        }
        self.unregister_from_current_avatar();
        self.player_avatar = None;
        self.user_id = value;
    }

    #[func]
    pub fn set_attach_point(&mut self, value: i32) {
        if self.attach_point == value {
            return;
        }
        self.unregister_from_current_avatar();
        self.attach_point = value;
        self.register_with_current_avatar();
    }

    /// The scene's collider tree changed.
    #[func]
    fn on_tree_changed(&mut self) {
        // During scene teardown this node can outlive its parent: tree_changed still fires after
        // removal, and the parent is gone then.
        let Some(parent) = self.base().get_parent() else {
            return;
        };
        let owner = self.base().clone();
        self.filter.rescan(&parent, &owner);
    }
}

impl DclAvatarAttach {
    /// Starts following: must run once the node is in the entity. Colliders under the entity are
    /// filtered, and the first transform is placed.
    pub fn start(&mut self) {
        let Some(parent) = self.base().get_parent() else {
            return;
        };
        let owner = self.base().clone();
        if let Some(scene) = scene_of(&parent) {
            scene
                .signals()
                .tree_changed()
                .connect_other(&self.to_gd(), |this: &mut Self| this.on_tree_changed());
        }
        self.filter.rescan(&parent, &owner);

        self.follow_anchor();
        self.base_mut().set_process_priority(0);
    }

    fn follow_anchor(&mut self) {
        let Some(mut parent) = self
            .base()
            .get_parent()
            .and_then(|p| p.try_cast::<Node3D>().ok())
        else {
            return;
        };

        if !self
            .player_avatar
            .as_ref()
            .is_some_and(|a| a.is_instance_valid())
        {
            self.player_avatar = None;
            self.registered_anchor = -1;
            self.look_up_player();
            if self.player_avatar.is_none() {
                return;
            }
        }

        // The scene's Transform handler updated the scale if it differs from what was written.
        if !self.scale_initialized {
            self.scene_scale = Vector3::ONE;
            self.scale_initialized = true;
            self.last_written_scale = Vector3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
        }
        if (parent.get_scale() - self.last_written_scale).length() > 1e-4 {
            self.scene_scale = parent.get_scale();
        }

        if let Some(avatar) = self.player_avatar.as_ref() {
            let anchor = avatar
                .bind()
                .get_anchor_point_global_transform(self.attach_point);
            parent.set_global_transform(anchor);
        }
        parent.set_scale(self.scene_scale);
        self.last_written_scale = self.scene_scale;
    }

    fn look_up_player(&mut self) {
        let primary = globals::player_identity().bind().get_address_str();
        let wanted = if self.user_id.is_empty() {
            primary.clone()
        } else {
            self.user_id.clone()
        };
        self.player_avatar = if primary == wanted {
            // The local player's avatar.
            crate::avatars::impostor_capture::registered()
                .into_iter()
                .find(|a| a.bind().is_local_player)
        } else {
            globals::avatars_scene()
                .bind()
                .get_avatar_by_address(self.user_id.clone())
        };
        self.register_with_current_avatar();
    }

    /// Registers against the current avatar for the current attach point. Non-skeletal anchors
    /// and out-of-range ids do not use the avatar's anchor cache.
    fn register_with_current_avatar(&mut self) {
        let me = self.base().instance_id();
        let Some(avatar) = self
            .player_avatar
            .as_mut()
            .filter(|a| a.is_instance_valid())
        else {
            return;
        };
        if !SKELETAL_ANCHORS.contains(&self.attach_point) {
            return;
        }
        avatar.bind_mut().add_anchor_user(self.attach_point, me);
        self.registered_anchor = self.attach_point;
    }

    fn unregister_from_current_avatar(&mut self) {
        if self.registered_anchor == -1 {
            return;
        }
        let me = self.base().instance_id();
        if let Some(avatar) = self
            .player_avatar
            .as_mut()
            .filter(|a| a.is_instance_valid())
        {
            avatar
                .bind_mut()
                .remove_anchor_user(self.registered_anchor, me);
        }
        self.registered_anchor = -1;
    }
}
