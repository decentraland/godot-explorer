use std::collections::HashMap;

use ethers::types::H160;
use godot::prelude::*;

use crate::{
    comms::profile::SerializedProfile,
    dcl::{
        components::{
            proto_components::kernel::comms::rfc4, transform_and_parent::DclTransformAndParent,
            SceneEntityId,
        },
        crdt::{last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState},
    },
    wallet::AsH160,
};

#[derive(GodotClass)]
#[class(base=Node)]
pub struct AvatarScene {
    #[base]
    base: Base<Node>,

    // map alias to the entity_id
    avatar_entity: HashMap<u32, SceneEntityId>,
    avatar_godot_scene: HashMap<SceneEntityId, Gd<Node3D>>,
    avatar_address: HashMap<H160, u32>,

    // scenes_dirty: HashMap<SceneId, HashMap<SceneEntityId, SceneComponentId>>,
    //
    crdt: SceneCrdtState,

    last_updated_profile: HashMap<SceneEntityId, SerializedProfile>,
}

#[godot_api]
impl NodeVirtual for AvatarScene {
    fn init(base: Base<Node>) -> Self {
        AvatarScene {
            base,
            avatar_entity: HashMap::new(),
            crdt: SceneCrdtState::from_proto(),
            avatar_godot_scene: HashMap::new(),
            avatar_address: HashMap::new(),
            last_updated_profile: HashMap::new(),
        }
    }
}

#[godot_api]
impl AvatarScene {
    #[func]
    pub fn update_avatar_profile(&mut self, alias: u32, profile: Dictionary) {
        let entity_id = if let Some(entity_id) = self.avatar_entity.get(&alias) {
            *entity_id
        } else {
            // TODO: handle this condition
            return;
        };

        self.avatar_godot_scene
            .get_mut(&entity_id)
            .unwrap()
            .call("update_avatar".into(), &[profile.to_variant()]);
    }

    #[func]
    pub fn update_avatar_transform(&mut self, alias: u32, transform: Transform3D) {
        let entity_id = if let Some(entity_id) = self.avatar_entity.get(&alias) {
            *entity_id
        } else {
            // TODO: handle this condition
            return;
        };

        let dcl_transform = DclTransformAndParent::from_godot(&transform, Vector3::ZERO);

        // let avatar_scene = self.avatar_godot_scene.get_mut(&entity_id).unwrap();

        // // TODO: the scale seted in the transform is local
        // avatar_scene.set_transform(dcl_transform.to_godot_transform_3d());
        self.avatar_godot_scene.get_mut(&entity_id).unwrap().call(
            "set_target".into(),
            &[dcl_transform.to_godot_transform_3d().to_variant()],
        );

        self.crdt
            .get_transform_mut()
            .put(entity_id, Some(dcl_transform));
    }

    #[func]
    pub fn add_avatar(&mut self, alias: u32, address: GodotString) {
        // TODO: the entity Self::MAX_ENTITY_ID + 1 would be a buggy avatar
        let entity_id = self
            .get_next_entity_id()
            .unwrap_or(SceneEntityId::new(Self::MAX_ENTITY_ID + 1, 0));
        self.crdt.entities.try_init(entity_id);

        self.avatar_entity.insert(alias, entity_id);

        let new_avatar =
            godot::engine::load::<PackedScene>("res://src/decentraland_components/avatar.tscn")
                .instantiate()
                .unwrap()
                .cast::<Node3D>();

        if let Some(address) = address.to_string().as_h160() {
            self.avatar_address.insert(address, alias);
        }

        self.base.add_child(new_avatar.clone().upcast());
        self.avatar_godot_scene.insert(entity_id, new_avatar);
    }

    #[func]
    pub fn get_avatar_by_address(&self, address: GodotString) -> Option<Gd<Node3D>> {
        if let Some(address) = address.to_string().as_h160() {
            if let Some(alias) = self.avatar_address.get(&address) {
                if let Some(entity_id) = self.avatar_entity.get(alias) {
                    return self.avatar_godot_scene.get(entity_id).cloned();
                }
            }
        }
        None
    }
}

impl AvatarScene {
    const FROM_ENTITY_ID: u16 = 32;
    const MAX_ENTITY_ID: u16 = 256;
    // const AVATAR_COMPONENTS: &[SceneComponentId] = &[SceneComponentId::AVATAR_ATTACH];

    // This function is not optimized, it will iterate over all the entities but this happens only when add an player
    fn get_next_entity_id(&self) -> Result<SceneEntityId, &'static str> {
        for entity_number in Self::FROM_ENTITY_ID..Self::MAX_ENTITY_ID {
            let (version, live) = self.crdt.entities.get_entity_stat(entity_number);

            if !live {
                let entity_id = SceneEntityId::new(entity_number, *version);
                return Ok(entity_id);
            }
        }

        Err("No more entity ids available")
    }

    pub fn clean(&mut self) {
        self.avatar_entity.clear();

        let avatars = std::mem::take(&mut self.avatar_godot_scene);
        for (_, avatar) in avatars {
            self.base.remove_child(avatar.upcast());
        }
    }

    pub fn remove_avatar(&mut self, alias: u32) {
        if let Some(entity_id) = self.avatar_entity.remove(&alias) {
            self.crdt.kill_entity(&entity_id);
            let mut avatar = self.avatar_godot_scene.remove(&entity_id).unwrap();
            self.base.remove_child(avatar.clone().upcast());

            self.avatar_address.retain(|_, v| *v != alias);

            avatar.queue_free();
        }
    }

    pub fn update_transform(&mut self, alias: u32, transform: &rfc4::Position) {
        let entity_id = if let Some(entity_id) = self.avatar_entity.get(&alias) {
            *entity_id
        } else {
            // TODO: handle this condition
            return;
        };

        let dcl_transform = DclTransformAndParent {
            translation: godot::prelude::Vector3 {
                x: transform.position_x,
                y: transform.position_y,
                z: transform.position_z,
            },
            rotation: godot::prelude::Quaternion {
                x: transform.rotation_x,
                y: transform.rotation_y,
                z: transform.rotation_z,
                w: transform.rotation_w,
            },
            scale: godot::prelude::Vector3::ONE,
            parent: SceneEntityId::ROOT,
        };

        // let avatar_scene = self.avatar_godot_scene.get_mut(&entity_id).unwrap();

        // // TODO: the scale seted in the transform is local
        // avatar_scene.set_transform(dcl_transform.to_godot_transform_3d());
        self.avatar_godot_scene.get_mut(&entity_id).unwrap().call(
            "set_target".into(),
            &[dcl_transform.to_godot_transform_3d().to_variant()],
        );

        self.crdt
            .get_transform_mut()
            .put(entity_id, Some(dcl_transform));
    }

    pub fn update_avatar(&mut self, alias: u32, profile: &SerializedProfile, base_url: &str) {
        let entity_id = if let Some(entity_id) = self.avatar_entity.get(&alias) {
            *entity_id
        } else {
            // TODO: handle this condition
            return;
        };

        // Avoid updating avatar with the same data
        match self.last_updated_profile.get(&entity_id) {
            Some(val) => {
                if profile.eq(val) {
                    return;
                }
            }
            None => {}
        }
        self.last_updated_profile.insert(entity_id, profile.clone());

        self.avatar_godot_scene.get_mut(&entity_id).unwrap().call(
            "update_avatar".into(),
            &[profile.to_godot_dictionary(base_url).to_variant()],
        );
    }

    pub fn spawn_voice_channel(
        &mut self,
        alias: u32,
        sample_rate: u32,
        num_channels: u32,
        samples_per_channel: u32,
    ) {
        let entity_id = if let Some(entity_id) = self.avatar_entity.get(&alias) {
            *entity_id
        } else {
            // TODO: handle this condition
            return;
        };

        let (sample_rate, num_channels, samples_per_channel) = (
            sample_rate.to_variant(),
            num_channels.to_variant(),
            samples_per_channel.to_variant(),
        );

        self.avatar_godot_scene.get_mut(&entity_id).unwrap().call(
            "spawn_voice_channel".into(),
            &[sample_rate, num_channels, samples_per_channel],
        );
    }

    pub fn push_voice_frame(&mut self, alias: u32, frame: PackedVector2Array) {
        let entity_id = if let Some(entity_id) = self.avatar_entity.get(&alias) {
            *entity_id
        } else {
            // TODO: handle this condition
            return;
        };

        self.avatar_godot_scene
            .get_mut(&entity_id)
            .unwrap()
            .call("push_voice_frame".into(), &[frame.to_variant()]);
    }
}
