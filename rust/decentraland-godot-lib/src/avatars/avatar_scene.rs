use std::collections::HashMap;

use godot::prelude::*;

use crate::dcl::{
    components::{
        proto_components::kernel::comms::rfc4, transform_and_parent::DclTransformAndParent,
        SceneEntityId,
    },
    crdt::{last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState},
};

#[derive(GodotClass)]
#[class(base=Node)]
pub struct AvatarScene {
    #[base]
    base: Base<Node>,

    // map alias to the entity_id
    avatar_entity: HashMap<u32, SceneEntityId>,
    avatar_godot_scene: HashMap<SceneEntityId, Gd<Node3D>>,

    // scenes_dirty: HashMap<SceneId, HashMap<SceneEntityId, SceneComponentId>>,
    //
    crdt: SceneCrdtState,
}

#[godot_api]
impl NodeVirtual for AvatarScene {
    fn init(base: Base<Node>) -> Self {
        AvatarScene {
            base,
            avatar_entity: HashMap::new(),
            crdt: SceneCrdtState::from_proto(),
            // scenes_dirty: HashMap::new(),
            avatar_godot_scene: HashMap::new(),
        }
    }
}

impl AvatarScene {
    const FROM_ENTITY_ID: u16 = 10;
    const MAX_ENTITY_ID: u16 = 200;
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
            self.remove_child(avatar.upcast());
        }
    }

    pub fn add_avatar(&mut self, alias: u32) {
        // TODO: the entity Self::MAX_ENTITY_ID + 1 would be a buggy avatar
        let entity_id = self
            .get_next_entity_id()
            .unwrap_or(SceneEntityId::new(Self::MAX_ENTITY_ID + 1, 0));

        self.avatar_entity.insert(alias, entity_id);

        let new_avatar =
            godot::engine::load::<PackedScene>("res://src/decentraland_components/avatar.tscn")
                .instantiate()
                .unwrap()
                .cast::<Node3D>();

        self.add_child(new_avatar.share().upcast());
        self.avatar_godot_scene.insert(entity_id, new_avatar);
    }

    pub fn remove_avatar(&mut self, alias: u32) {
        if let Some(entity_id) = self.avatar_entity.remove(&alias) {
            self.crdt.kill_entity(&entity_id);
            let mut avatar = self.avatar_godot_scene.remove(&entity_id).unwrap();
            self.remove_child(avatar.share().upcast());
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

        let avatar_scene = self.avatar_godot_scene.get_mut(&entity_id).unwrap();

        // TODO: the scale seted in the transform is local
        avatar_scene.set_transform(dcl_transform.to_godot_transform_3d());

        self.crdt
            .get_transform_mut()
            .put(entity_id, Some(dcl_transform));
    }
}
