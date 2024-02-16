use std::collections::HashMap;

use ethers::types::H160;
use godot::prelude::*;

use crate::{
    auth::wallet::AsH160,
    avatars::dcl_user_profile::DclUserProfile,
    comms::profile::UserProfile,
    dcl::{
        components::{
            internal_player_data::InternalPlayerData, proto_components::kernel::comms::rfc4,
            transform_and_parent::DclTransformAndParent, SceneEntityId,
        },
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
        SceneId,
    },
    godot_classes::{
        dcl_avatar::{AvatarMovementType, DclAvatar},
        dcl_global::DclGlobal,
    },
};

type AvatarAlias = u32;

#[derive(GodotClass)]
#[class(base=Node)]
pub struct AvatarScene {
    #[base]
    base: Base<Node>,

    // map alias to the entity_id
    avatar_entity: HashMap<AvatarAlias, SceneEntityId>,
    avatar_godot_scene: HashMap<SceneEntityId, Gd<DclAvatar>>,
    avatar_address: HashMap<H160, AvatarAlias>,

    crdt_state: SceneCrdtState,

    last_updated_profile: HashMap<SceneEntityId, UserProfile>,
}

#[godot_api]
impl INode for AvatarScene {
    fn init(base: Base<Node>) -> Self {
        AvatarScene {
            base,
            avatar_entity: HashMap::new(),
            crdt_state: SceneCrdtState::from_proto(),
            avatar_godot_scene: HashMap::new(),
            avatar_address: HashMap::new(),
            last_updated_profile: HashMap::new(),
        }
    }

    fn ready(&mut self) {
        DclGlobal::singleton().bind_mut().scene_runner.connect(
            "scene_spawned".into(),
            self.base.callable("on_scene_spawned"),
        );
    }
}

macro_rules! sync_crdt_lww_component {
    ($entity_id:ident, $target_component:ident, $local_component:ident) => {
        let local_value_entry = $local_component.get($entity_id);

        if let Some(value_entry) = local_value_entry {
            let diff_timestamp = local_value_entry.map(|v| v.timestamp)
                != $target_component.get($entity_id).map(|v| v.timestamp);

            if diff_timestamp {
                $target_component.set(
                    *$entity_id,
                    value_entry.timestamp,
                    value_entry.value.clone(),
                );
            }
        }
    };
}

#[godot_api]
impl AvatarScene {
    #[func]
    pub fn update_primary_player_profile(&mut self, profile: Gd<DclUserProfile>) {
        self.update_avatar(SceneEntityId::PLAYER, &profile.bind().inner);
    }

    #[func]
    pub fn update_avatar_transform_with_godot_transform(
        &mut self,
        alias: u32,
        transform: Transform3D,
    ) {
        let entity_id = if let Some(entity_id) = self.avatar_entity.get(&alias) {
            *entity_id
        } else {
            // TODO: handle this condition
            return;
        };

        let dcl_transform = DclTransformAndParent::from_godot(&transform, Vector3::ZERO);
        self._update_avatar_transform(&entity_id, dcl_transform);
    }

    #[func]
    pub fn add_avatar(&mut self, alias: u32, address: GString) {
        // TODO: the entity Self::MAX_ENTITY_ID + 1 would be a buggy avatar
        let entity_id = self
            .get_next_entity_id()
            .unwrap_or(SceneEntityId::new(Self::MAX_ENTITY_ID + 1, 0));
        self.crdt_state.entities.try_init(entity_id);

        self.avatar_entity.insert(alias, entity_id);

        let mut new_avatar: Gd<DclAvatar> = godot::engine::load::<PackedScene>(
            "res://src/decentraland_components/avatar/avatar.tscn",
        )
        .instantiate()
        .unwrap()
        .cast::<DclAvatar>();

        if let Some(address) = address.to_string().as_h160() {
            self.avatar_address.insert(address, alias);
        }

        new_avatar
            .bind_mut()
            .set_movement_type(AvatarMovementType::LerpTwoPoints as i32);

        let instance_id = self.base.instance_id();
        let avatar_entity_id = entity_id;
        let callable = Callable::from_fn("on_avatar_changed_scene", move |args: &[&Variant]| {
            if args.len() != 2 {
                return Err(());
            }

            let scene_id = args[0].try_to::<i32>().map_err(|_e| ())?;
            let prev_scene_id = args[1].try_to::<i32>().map_err(|_e| ())?;

            if let Ok(mut avatar_scene) = Gd::<AvatarScene>::try_from_instance_id(instance_id) {
                avatar_scene.call_deferred(
                    "on_avatar_changed_scene".into(),
                    &[
                        scene_id.to_variant(),
                        prev_scene_id.to_variant(),
                        avatar_entity_id.as_i32().to_variant(),
                    ],
                );
            }

            Ok(Variant::nil())
        });

        new_avatar.connect("change_scene_id".into(), callable);

        self.base.add_child(new_avatar.clone().upcast());
        self.avatar_godot_scene.insert(entity_id, new_avatar);
    }

    #[func]
    pub fn get_avatar_by_address(&self, address: GString) -> Option<Gd<DclAvatar>> {
        if let Some(address) = address.to_string().as_h160() {
            if let Some(alias) = self.avatar_address.get(&address) {
                if let Some(entity_id) = self.avatar_entity.get(alias) {
                    return self.avatar_godot_scene.get(entity_id).cloned();
                }
            }
        }
        None
    }

    #[func]
    pub fn on_scene_spawned(&mut self) {
        for (_, avatar) in self.avatar_godot_scene.iter_mut() {
            avatar.bind_mut().on_parcel_scenes_changed();
        }
    }

    #[func]
    fn on_avatar_changed_scene(&self, scene_id: i32, prev_scene_id: i32, avatar_entity_id: i32) {
        let scene_id = SceneId(scene_id);
        let prev_scene_id = SceneId(prev_scene_id);
        let avatar_entity_id = SceneEntityId::from_i32(avatar_entity_id);

        // TODO: as this function was deferred called, check if the current_parcel_entity_id is the same
        // maybe it's better to cache the last parcel here instead of using prev_scene_id
        // the state of to what parcel the avatar belongs is stored in the avatar_scene

        let mut scene_runner = DclGlobal::singleton().bind().scene_runner.clone();
        let mut scene_runner = scene_runner.bind_mut();
        if let Some(prev_scene) = scene_runner.get_scene_mut(&prev_scene_id) {
            prev_scene
                .avatar_scene_updates
                .transform
                .insert(avatar_entity_id, None);
            prev_scene
                .avatar_scene_updates
                .internal_player_data
                .insert(avatar_entity_id, InternalPlayerData { inside: false });
        }

        if let Some(scene) = scene_runner.get_scene_mut(&scene_id) {
            let dcl_transform = DclTransformAndParent::default(); // TODO: get real transform with scene_offset

            let mut avatar_scene_transform = dcl_transform.clone();
            avatar_scene_transform.translation.x -=
                (scene.scene_entity_definition.get_base_parcel().x as f32) * 16.0;

            // TODO: I think this is working fine but
            //   Should it be added instead of subtracted? (z is inverted in godot and dcl)
            avatar_scene_transform.translation.z -=
                (scene.scene_entity_definition.get_base_parcel().y as f32) * 16.0;

            scene
                .avatar_scene_updates
                .transform
                .insert(avatar_entity_id, Some(dcl_transform.clone()));

            scene
                .avatar_scene_updates
                .internal_player_data
                .insert(avatar_entity_id, InternalPlayerData { inside: true });
        }
    }

    #[func]
    pub fn update_dcl_avatar_by_alias(&mut self, alias: u32, profile: Gd<DclUserProfile>) {
        self.update_avatar_by_alias(alias, &profile.bind().inner);
    }
}

impl AvatarScene {
    const FROM_ENTITY_ID: u16 = 32;
    const MAX_ENTITY_ID: u16 = 256;

    // This function is not optimized, it will iterate over all the entities but this happens only when add an player
    fn get_next_entity_id(&self) -> Result<SceneEntityId, &'static str> {
        for entity_number in Self::FROM_ENTITY_ID..Self::MAX_ENTITY_ID {
            let (version, live) = self.crdt_state.entities.get_entity_stat(entity_number);

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
            self.crdt_state.kill_entity(&entity_id);
            let mut avatar = self.avatar_godot_scene.remove(&entity_id).unwrap();
            self.base.remove_child(avatar.clone().upcast());

            self.avatar_address.retain(|_, v| *v != alias);

            self.last_updated_profile.remove(&entity_id);

            avatar.queue_free();

            // Push dirty state in all the scenes
            let mut scene_runner = DclGlobal::singleton().bind().scene_runner.clone();
            let mut scene_runner = scene_runner.bind_mut();
            for (_, scene) in scene_runner.get_all_scenes_mut().iter_mut() {
                scene
                    .avatar_scene_updates
                    .deleted_entities
                    .insert(entity_id);
            }
        }
    }

    fn _update_avatar_transform(
        &mut self,
        avatar_entity_id: &SceneEntityId,
        dcl_transform: DclTransformAndParent,
    ) {
        let avatar_scene = self
            .avatar_godot_scene
            .get_mut(avatar_entity_id)
            .expect("avatar not found");
        avatar_scene
            .bind_mut()
            .set_target_position(dcl_transform.to_godot_transform_3d());

        let mut scene_runner = DclGlobal::singleton().bind().scene_runner.clone();
        let mut scene_runner = scene_runner.bind_mut();

        let avatar_current_parcel_scene_id = avatar_scene.bind().get_current_parcel_scene_id();
        let avatar_active_scene_ids = {
            let mut scene_ids = scene_runner.get_global_scene_ids();
            if avatar_current_parcel_scene_id != SceneId::INVALID.0 {
                scene_ids.push(SceneId(avatar_current_parcel_scene_id));
            }
            scene_ids
        };

        // Push dirty state only in active scenes
        for scene_id in avatar_active_scene_ids {
            if let Some(scene) = scene_runner.get_scene_mut(&scene_id) {
                let mut avatar_scene_transform = dcl_transform.clone();
                avatar_scene_transform.translation.x -=
                    (scene.scene_entity_definition.get_base_parcel().x as f32) * 16.0;

                // TODO: I think this is working fine but
                //   Should it be added instead of subtracted? (z is inverted in godot and dcl)
                avatar_scene_transform.translation.z -=
                    (scene.scene_entity_definition.get_base_parcel().y as f32) * 16.0;

                scene
                    .avatar_scene_updates
                    .transform
                    .insert(*avatar_entity_id, Some(dcl_transform.clone()));
            }
        }

        self.crdt_state
            .get_transform_mut()
            .put(*avatar_entity_id, Some(dcl_transform));
    }

    pub fn update_avatar_transform_with_rfc4_position(
        &mut self,
        alias: u32,
        transform: &rfc4::Position,
    ) {
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

        self._update_avatar_transform(&entity_id, dcl_transform);
    }

    pub fn update_avatar_by_alias(&mut self, alias: u32, profile: &UserProfile) {
        let entity_id = if let Some(entity_id) = self.avatar_entity.get(&alias) {
            *entity_id
        } else {
            // TODO: handle this condition
            return;
        };

        self.update_avatar(entity_id, profile);
    }

    pub fn update_avatar(&mut self, entity_id: SceneEntityId, profile: &UserProfile) {
        // Avoid updating avatar with the same data
        if let Some(val) = self.last_updated_profile.get(&entity_id) {
            if profile.eq(val) {
                return;
            }
        }
        self.last_updated_profile.insert(entity_id, profile.clone());

        if let Some(avatar_scene) = self.avatar_godot_scene.get_mut(&entity_id) {
            let dcl_user_profile = DclUserProfile::from_gd(profile.clone());
            avatar_scene.call(
                "async_update_avatar_from_profile".into(),
                &[dcl_user_profile.to_variant()],
            );
        }

        let new_avatar_base = Some(profile.content.to_pb_avatar_base());
        let avatar_base_component =
            SceneCrdtStateProtoComponents::get_avatar_base(&self.crdt_state);
        let avatar_base_component_value = avatar_base_component
            .get(&entity_id)
            .and_then(|v| v.value.clone());
        if avatar_base_component_value != new_avatar_base {
            // Push dirty state in all the scenes
            let mut scene_runner = DclGlobal::singleton().bind().scene_runner.clone();
            let mut scene_runner = scene_runner.bind_mut();
            for (_, scene) in scene_runner.get_all_scenes_mut().iter_mut() {
                scene.avatar_scene_updates.avatar_base.insert(
                    entity_id,
                    new_avatar_base.clone().expect("value was assigned above"),
                );
            }
            SceneCrdtStateProtoComponents::get_avatar_base_mut(&mut self.crdt_state)
                .put(entity_id, new_avatar_base);
        }

        let new_avatar_equipped_data = Some(profile.content.to_pb_avatar_equipped_data());
        let avatar_equipped_data_component =
            SceneCrdtStateProtoComponents::get_avatar_equipped_data(&self.crdt_state);
        let avatar_equipped_data_value = avatar_equipped_data_component
            .get(&entity_id)
            .and_then(|v| v.value.clone());
        if avatar_equipped_data_value != new_avatar_equipped_data {
            // Push dirty state in all the scenes
            let mut scene_runner = DclGlobal::singleton().bind().scene_runner.clone();
            let mut scene_runner = scene_runner.bind_mut();
            for (_, scene) in scene_runner.get_all_scenes_mut().iter_mut() {
                scene.avatar_scene_updates.avatar_equipped_data.insert(
                    entity_id,
                    new_avatar_equipped_data
                        .clone()
                        .expect("value was assigned above"),
                );
            }
            SceneCrdtStateProtoComponents::get_avatar_equipped_data_mut(&mut self.crdt_state)
                .put(entity_id, new_avatar_equipped_data);
        }

        let new_player_identity_data = Some(profile.content.to_pb_player_identity_data());
        let player_identity_data_component =
            SceneCrdtStateProtoComponents::get_player_identity_data(&self.crdt_state);
        let player_identity_data_value = player_identity_data_component
            .get(&entity_id)
            .and_then(|v| v.value.clone());
        if player_identity_data_value != new_player_identity_data {
            // Push dirty state in all the scenes
            let mut scene_runner = DclGlobal::singleton().bind().scene_runner.clone();
            let mut scene_runner = scene_runner.bind_mut();
            for (_, scene) in scene_runner.get_all_scenes_mut().iter_mut() {
                scene.avatar_scene_updates.player_identity_data.insert(
                    entity_id,
                    new_player_identity_data
                        .clone()
                        .expect("value was assigned above"),
                );
            }

            SceneCrdtStateProtoComponents::get_player_identity_data_mut(&mut self.crdt_state)
                .put(entity_id, new_player_identity_data);
        }
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

    // This function should be only called in the first tick
    pub fn first_sync_crdt_state(
        &self,
        target_crdt_state: &mut SceneCrdtState,
        filter_by_scene_id: Option<SceneId>,
        primary_player_inside: bool,
    ) {
        for entity_number in Self::FROM_ENTITY_ID..Self::MAX_ENTITY_ID {
            let (local_version, local_live) =
                self.crdt_state.entities.get_entity_stat(entity_number);
            let (target_version, target_live) =
                target_crdt_state.entities.get_entity_stat(entity_number);

            if local_version != target_version || local_live != target_live {
                if *local_live {
                    target_crdt_state
                        .entities
                        .try_init(SceneEntityId::new(entity_number, *local_version));
                } else {
                    target_crdt_state
                        .entities
                        .kill(SceneEntityId::new(entity_number, *local_version - 1));
                }
            }
        }

        let local_transform_component = self.crdt_state.get_transform();
        let local_player_identity_data =
            SceneCrdtStateProtoComponents::get_player_identity_data(&self.crdt_state);
        let local_avatar_equipped_data =
            SceneCrdtStateProtoComponents::get_avatar_equipped_data(&self.crdt_state);
        let local_avatar_base = SceneCrdtStateProtoComponents::get_avatar_base(&self.crdt_state);

        for (entity_id, avatar_scene) in self.avatar_godot_scene.iter() {
            let target_transform_component = target_crdt_state.get_transform_mut();

            let null_transform: bool = if let Some(scene_id_int) = filter_by_scene_id {
                scene_id_int.0 != avatar_scene.bind().get_current_parcel_scene_id()
            } else {
                false
            };

            if null_transform {
                if let Some(value) = target_transform_component.get(entity_id) {
                    if value.value.is_some() {
                        target_transform_component.put(*entity_id, None);
                    }
                }
            } else {
                // todo: transform to local coordinates
                sync_crdt_lww_component!(
                    entity_id,
                    target_transform_component,
                    local_transform_component
                );
            }

            let target_internal_player_data = target_crdt_state.get_internal_player_data_mut();
            target_internal_player_data.put(
                *entity_id,
                Some(InternalPlayerData {
                    inside: !null_transform,
                }),
            );

            let target_player_identity_data =
                SceneCrdtStateProtoComponents::get_player_identity_data_mut(target_crdt_state);
            sync_crdt_lww_component!(
                entity_id,
                target_player_identity_data,
                local_player_identity_data
            );

            let target_avatar_base =
                SceneCrdtStateProtoComponents::get_avatar_base_mut(target_crdt_state);
            sync_crdt_lww_component!(entity_id, target_avatar_base, local_avatar_base);

            let target_avatar_equipped_data =
                SceneCrdtStateProtoComponents::get_avatar_equipped_data_mut(target_crdt_state);
            sync_crdt_lww_component!(
                entity_id,
                target_avatar_equipped_data,
                local_avatar_equipped_data
            );
        }

        let entity_id = &SceneEntityId::PLAYER;
        let target_player_identity_data =
            SceneCrdtStateProtoComponents::get_player_identity_data_mut(target_crdt_state);
        sync_crdt_lww_component!(
            entity_id,
            target_player_identity_data,
            local_player_identity_data
        );

        let target_avatar_base =
            SceneCrdtStateProtoComponents::get_avatar_base_mut(target_crdt_state);
        sync_crdt_lww_component!(entity_id, target_avatar_base, local_avatar_base);

        let target_avatar_equipped_data =
            SceneCrdtStateProtoComponents::get_avatar_equipped_data_mut(target_crdt_state);
        sync_crdt_lww_component!(
            entity_id,
            target_avatar_equipped_data,
            local_avatar_equipped_data
        );

        let target_internal_player_data = target_crdt_state.get_internal_player_data_mut();
        target_internal_player_data.put(
            *entity_id,
            Some(InternalPlayerData {
                inside: primary_player_inside,
            }),
        );
    }
}
