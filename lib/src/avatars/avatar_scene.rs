use std::collections::HashMap;

use ethers_core::types::H160;
use godot::classes::image::Format;
use godot::classes::multi_mesh::TransformFormat;
use godot::classes::{
    Image, MultiMesh, MultiMeshInstance3D, QuadMesh, ResourceLoader, ShaderMaterial, Texture2DArray,
};
use godot::prelude::*;

use crate::{
    auth::wallet::AsH160,
    avatars::dcl_user_profile::DclUserProfile,
    comms::profile::UserProfile,
    dcl::{
        components::{
            internal_player_data::InternalPlayerData,
            proto_components::{kernel::comms::rfc4, sdk::components::PbAvatarEmoteCommand},
            transform_and_parent::DclTransformAndParent,
            SceneEntityId,
        },
        crdt::{
            last_write_wins::{LastWriteWins, LastWriteWinsComponentOperation},
            SceneCrdtState, SceneCrdtStateProtoComponents,
        },
        SceneId,
    },
    godot_classes::{
        dcl_avatar::{AvatarMovementType, DclAvatar},
        dcl_global::DclGlobal,
    },
};

type AvatarAlias = u32;

const IMPOSTOR_MAX_LAYERS: u32 = 256;
const IMPOSTOR_TEX_WIDTH: i32 = 256;
const IMPOSTOR_TEX_HEIGHT: i32 = 512;
// Quad world-space size matches the AvatarPreview ortho capture
// (256x512 px @ ortho_size=2.5 → 1.25m W × 2.5m H).
const IMPOSTOR_QUAD_WIDTH: f32 = 1.25;
const IMPOSTOR_QUAD_HEIGHT: f32 = 2.5;
const IMPOSTOR_VERTICAL_OFFSET: f32 = 1.0;
const IMPOSTOR_SHADER_PATH: &str = "res://assets/avatar/impostor.gdshader";

#[derive(Clone, Copy, Debug)]
struct ImpostorSlot {
    layer_index: u32,
    fade_alpha: f32,
    tint_strength: f32,
    texture_loaded: bool,
    avatar_instance_id: InstanceId,
    distance_sq: f32,
    // Frame index of the last time this slot rendered with fade_alpha>0. Used
    // by LRU eviction so that slots that are no longer being shown (e.g., the
    // user moved away or turned around) are evicted before recently-seen ones.
    last_seen_frame: u64,
}

#[derive(GodotClass)]
#[class(base=Node)]
pub struct AvatarScene {
    base: Base<Node>,

    // map alias to the entity_id
    avatar_entity: HashMap<AvatarAlias, SceneEntityId>,
    avatar_godot_scene: HashMap<SceneEntityId, Gd<DclAvatar>>,
    avatar_address: HashMap<H160, AvatarAlias>,

    crdt_state: SceneCrdtState,

    last_updated_profile: HashMap<SceneEntityId, UserProfile>,

    // Timestamp tracking for movement messages
    last_movement_timestamp: HashMap<AvatarAlias, f32>,
    last_position_index: HashMap<AvatarAlias, u32>,

    last_emote_incremental_id: HashMap<AvatarAlias, u32>,

    impostor_multimesh: Option<Gd<MultiMeshInstance3D>>,
    impostor_texture_array: Option<Gd<Texture2DArray>>,
    impostor_slots: HashMap<i64, ImpostorSlot>,
    impostor_free_layers: Vec<u32>,
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
            last_movement_timestamp: HashMap::new(),
            last_position_index: HashMap::new(),
            last_emote_incremental_id: HashMap::new(),
            impostor_multimesh: None,
            impostor_texture_array: None,
            impostor_slots: HashMap::new(),
            impostor_free_layers: (0..IMPOSTOR_MAX_LAYERS).rev().collect(),
        }
    }

    fn ready(&mut self) {
        DclGlobal::singleton()
            .bind_mut()
            .scene_runner
            .connect("scene_spawned", &self.base().callable("on_scene_spawned"));

        self.setup_impostor_renderer();
    }

    fn process(&mut self, _delta: f64) {
        self.update_impostor_transforms();
    }
}

fn sync_crdt_lww_component<T>(
    entity_id: &SceneEntityId,
    target_component: &mut LastWriteWins<T>,
    local_component: &LastWriteWins<T>,
) where
    T: Clone,
{
    let local_value_entry = local_component.get(entity_id);

    if let Some(value_entry) = local_value_entry {
        let diff_timestamp = local_value_entry.map(|v| v.timestamp)
            != target_component.get(entity_id).map(|v| v.timestamp);

        if diff_timestamp {
            target_component.set(*entity_id, value_entry.timestamp, value_entry.value.clone());
        }
    }
}

#[godot_api]
impl AvatarScene {
    #[signal]
    fn avatar_scene_changed(avatars: Array<Gd<DclAvatar>>);

    #[signal]
    fn avatar_added(avatar: Gd<DclAvatar>);

    #[signal]
    fn avatar_removed(address: GString);

    fn setup_impostor_renderer(&mut self) {
        // Load shader first; if it fails we bail out so we don't render
        // default-white quads that mask the failure.
        let Some(shader_resource) = ResourceLoader::singleton().load(IMPOSTOR_SHADER_PATH) else {
            tracing::warn!(
                "Impostor shader not found at {} — impostor renderer disabled",
                IMPOSTOR_SHADER_PATH
            );
            return;
        };
        let shader = match shader_resource.try_cast::<godot::classes::Shader>() {
            Ok(s) => s,
            Err(_) => {
                tracing::error!(
                    "Impostor resource at {} is not a Shader — disabled",
                    IMPOSTOR_SHADER_PATH
                );
                return;
            }
        };

        let mut blank_images: Array<Gd<Image>> = Array::new();
        for _ in 0..IMPOSTOR_MAX_LAYERS {
            // mipmaps=true so the GPU can pick a smaller mip for distant
            // impostors. The data is zero-initialized; mip chain layout still
            // has to match what update_layer will upload later.
            let img = Image::create(IMPOSTOR_TEX_WIDTH, IMPOSTOR_TEX_HEIGHT, true, Format::RGBA8)
                .expect("Failed to create blank impostor image");
            blank_images.push(&img);
        }

        let mut texture_array = Texture2DArray::new_gd();
        let create_err = texture_array.create_from_images(&blank_images);
        if create_err != godot::global::Error::OK {
            tracing::error!(
                "Failed to create impostor texture array: error code {:?}",
                create_err
            );
            return;
        }

        let mut shader_material = ShaderMaterial::new_gd();
        shader_material.set_shader(&shader);
        shader_material.set_shader_parameter("impostor_array", &texture_array.to_variant());

        // Set the material on the QuadMesh surface (more reliable for
        // MultiMesh than relying solely on MMI3D.material_override).
        let mut quad = QuadMesh::new_gd();
        quad.set_size(Vector2::new(IMPOSTOR_QUAD_WIDTH, IMPOSTOR_QUAD_HEIGHT));
        quad.set_material(&shader_material.clone().upcast::<godot::classes::Material>());

        let mut multimesh = MultiMesh::new_gd();
        multimesh.set_transform_format(TransformFormat::TRANSFORM_3D);
        multimesh.set_use_custom_data(true);
        multimesh.set_mesh(&quad.upcast::<godot::classes::Mesh>());
        multimesh.set_instance_count(IMPOSTOR_MAX_LAYERS as i32);

        let mut mmi = MultiMeshInstance3D::new_alloc();
        mmi.set_name("impostor_multimesh");
        mmi.set_multimesh(&multimesh);
        mmi.set_material_override(&shader_material.upcast::<godot::classes::Material>());
        mmi.set_cast_shadows_setting(
            godot::classes::geometry_instance_3d::ShadowCastingSetting::OFF,
        );

        for i in 0..IMPOSTOR_MAX_LAYERS as i32 {
            multimesh.set_instance_transform(i, Transform3D::IDENTITY);
            multimesh.set_instance_custom_data(i, Color::from_rgba(0.0, 0.0, 0.0, 0.0));
        }

        self.base_mut().add_child(&mmi);
        self.impostor_multimesh = Some(mmi);
        self.impostor_texture_array = Some(texture_array);
        tracing::info!(
            "Impostor renderer initialized: {} layers, {}x{} px",
            IMPOSTOR_MAX_LAYERS,
            IMPOSTOR_TEX_WIDTH,
            IMPOSTOR_TEX_HEIGHT
        );
    }

    fn update_impostor_transforms(&mut self) {
        let Some(mmi) = self.impostor_multimesh.as_ref().cloned() else {
            return;
        };
        let Some(mut multimesh) = mmi.get_multimesh() else {
            return;
        };

        let mut stale: Vec<i64> = Vec::new();
        for (key, slot) in self.impostor_slots.iter() {
            let Ok(avatar) = Gd::<DclAvatar>::try_from_instance_id(slot.avatar_instance_id) else {
                stale.push(*key);
                continue;
            };
            let avatar_pos = avatar.get_global_position();
            let mut transform = Transform3D::IDENTITY;
            transform.origin = avatar_pos + Vector3::new(0.0, IMPOSTOR_VERTICAL_OFFSET, 0.0);

            multimesh.set_instance_transform(slot.layer_index as i32, transform);

            let visible_alpha = if slot.texture_loaded {
                slot.fade_alpha
            } else {
                0.0
            };
            multimesh.set_instance_custom_data(
                slot.layer_index as i32,
                Color::from_rgba(
                    slot.tint_strength,
                    visible_alpha,
                    slot.layer_index as f32,
                    0.0,
                ),
            );
        }

        for key in stale {
            self.clear_impostor(key);
        }
    }

    #[func]
    fn request_impostor_layer(
        &mut self,
        impostor_id: i64,
        avatar: Gd<DclAvatar>,
        distance: f32,
    ) -> i32 {
        let now = godot::classes::Engine::singleton().get_frames_drawn() as u64;
        if let Some(slot) = self.impostor_slots.get_mut(&impostor_id) {
            slot.last_seen_frame = now;
            return slot.layer_index as i32;
        }
        let distance_sq = distance * distance;

        let layer = match self.impostor_free_layers.pop() {
            Some(l) => l,
            None => {
                // Pool full — evict the LRU slot (oldest last_seen_frame). This
                // keeps slots that are actively rendered and frees the ones that
                // haven't been touched in a while (e.g., player walked away).
                let Some((lru_id, lru_layer)) = self
                    .impostor_slots
                    .iter()
                    .map(|(id, slot)| (*id, slot.last_seen_frame, slot.layer_index))
                    .min_by_key(|(_, last_seen, _)| *last_seen)
                    .map(|(id, _, layer)| (id, layer))
                else {
                    return -1;
                };
                self.impostor_slots.remove(&lru_id);
                if let Some(mmi) = self.impostor_multimesh.as_ref().cloned() {
                    if let Some(mut multimesh) = mmi.get_multimesh() {
                        multimesh.set_instance_custom_data(
                            lru_layer as i32,
                            Color::from_rgba(0.0, 0.0, 0.0, 0.0),
                        );
                    }
                }
                lru_layer
            }
        };

        self.impostor_slots.insert(
            impostor_id,
            ImpostorSlot {
                layer_index: layer,
                fade_alpha: 0.0,
                tint_strength: 0.0,
                texture_loaded: false,
                avatar_instance_id: avatar.instance_id(),
                distance_sq,
                last_seen_frame: now,
            },
        );
        layer as i32
    }

    #[func]
    fn set_impostor_texture(&mut self, impostor_id: i64, image: Gd<Image>) {
        let Some(slot) = self.impostor_slots.get_mut(&impostor_id) else {
            tracing::warn!(
                "set_impostor_texture: no slot for impostor_id {}",
                impostor_id
            );
            return;
        };
        let Some(tex_array) = self.impostor_texture_array.as_mut() else {
            return;
        };

        let mut img = image;
        if img.get_width() != IMPOSTOR_TEX_WIDTH || img.get_height() != IMPOSTOR_TEX_HEIGHT {
            img.resize(IMPOSTOR_TEX_WIDTH, IMPOSTOR_TEX_HEIGHT);
        }
        if img.get_format() != Format::RGBA8 {
            img.convert(Format::RGBA8);
        }
        // The texture array was created with mipmaps; the layer upload must
        // include the same mip chain or update_layer rejects it.
        let mip_err = img.generate_mipmaps();
        if mip_err != godot::global::Error::OK {
            tracing::warn!(
                "Failed to generate impostor mipmaps for slot {}: {:?}",
                impostor_id,
                mip_err
            );
        }

        tex_array.update_layer(&img, slot.layer_index as i32);
        slot.texture_loaded = true;
    }

    #[func]
    fn set_impostor_state(
        &mut self,
        impostor_id: i64,
        fade_alpha: f32,
        tint_strength: f32,
        distance: f32,
    ) {
        if let Some(slot) = self.impostor_slots.get_mut(&impostor_id) {
            let clamped = fade_alpha.clamp(0.0, 1.0);
            slot.fade_alpha = clamped;
            slot.tint_strength = tint_strength.clamp(0.0, 1.0);
            slot.distance_sq = distance * distance;
            // Only mark the slot as recently-seen when it's actually rendering.
            // A slot at fade_alpha=0 is allocated-but-hidden; LRU eviction
            // should pick those before slots that are currently visible.
            if clamped > 0.0 {
                slot.last_seen_frame =
                    godot::classes::Engine::singleton().get_frames_drawn() as u64;
            }
        }
    }

    #[func]
    fn clear_impostor(&mut self, impostor_id: i64) {
        if let Some(slot) = self.impostor_slots.remove(&impostor_id) {
            self.impostor_free_layers.push(slot.layer_index);
            if let Some(mmi) = self.impostor_multimesh.as_ref().cloned() {
                if let Some(mut multimesh) = mmi.get_multimesh() {
                    multimesh.set_instance_custom_data(
                        slot.layer_index as i32,
                        Color::from_rgba(0.0, 0.0, 0.0, 0.0),
                    );
                }
            }
        }
    }

    #[func]
    fn invalidate_impostor_texture(&mut self, impostor_id: i64) {
        if let Some(slot) = self.impostor_slots.get_mut(&impostor_id) {
            slot.texture_loaded = false;
        }
    }

    #[func]
    fn has_impostor_capacity(&self) -> bool {
        !self.impostor_free_layers.is_empty()
    }

    #[func]
    fn impostor_diagnostics(&self) -> VarDictionary {
        let total = self.impostor_slots.len() as i64;
        let loaded = self
            .impostor_slots
            .values()
            .filter(|s| s.texture_loaded)
            .count() as i64;
        let visible = self
            .impostor_slots
            .values()
            .filter(|s| s.texture_loaded && s.fade_alpha > 0.0)
            .count() as i64;
        let mut dict = VarDictionary::new();
        dict.set("total_slots", total);
        dict.set("texture_loaded", loaded);
        dict.set("currently_visible", visible);
        dict.set("free_layers", self.impostor_free_layers.len() as i64);
        dict
    }

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
        // Check if avatar with this alias already exists
        if self.avatar_entity.contains_key(&alias) {
            tracing::debug!("Avatar with alias {} already exists, discarding", alias);
            return;
        }

        // TODO: the entity Self::MAX_ENTITY_ID + 1 would be a buggy avatar
        let entity_id = self
            .get_next_entity_id()
            .unwrap_or(SceneEntityId::new(Self::MAX_ENTITY_ID + 1, 0));
        self.crdt_state.entities.try_init(entity_id);

        self.avatar_entity.insert(alias, entity_id);

        let mut new_avatar: Gd<DclAvatar> = godot::tools::load::<PackedScene>(
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

        let instance_id = self.base().instance_id();
        let avatar_entity_id = entity_id;
        let avatar_changed_scene_callable =
            Callable::from_fn("on_avatar_changed_scene", move |args: &[&Variant]| {
                if args.len() != 2 {
                    return Variant::nil();
                }

                let Ok(scene_id) = args[0].try_to::<i32>() else {
                    return Variant::nil();
                };
                let Ok(prev_scene_id) = args[1].try_to::<i32>() else {
                    return Variant::nil();
                };

                if let Ok(mut avatar_scene) = Gd::<AvatarScene>::try_from_instance_id(instance_id) {
                    avatar_scene.call_deferred(
                        "on_avatar_changed_scene",
                        &[
                            scene_id.to_variant(),
                            prev_scene_id.to_variant(),
                            avatar_entity_id.as_i32().to_variant(),
                        ],
                    );
                }

                Variant::nil()
            });

        let emote_triggered_callable =
            Callable::from_fn("on_avatar_trigger_emote", move |args: &[&Variant]| {
                if args.len() != 2 {
                    return Variant::nil();
                }

                let Ok(emote_id) = args[0].try_to::<String>() else {
                    return Variant::nil();
                };
                let Ok(looping) = args[1].try_to::<bool>() else {
                    return Variant::nil();
                };

                if let Ok(mut avatar_scene) = Gd::<AvatarScene>::try_from_instance_id(instance_id) {
                    avatar_scene.call_deferred(
                        "on_avatar_trigger_emote",
                        &[
                            emote_id.to_variant(),
                            looping.to_variant(),
                            avatar_entity_id.as_i32().to_variant(),
                        ],
                    );
                }

                Variant::nil()
            });

        new_avatar.connect("change_scene_id", &avatar_changed_scene_callable);
        new_avatar.connect("emote_triggered", &emote_triggered_callable);

        self.base_mut().add_child(&new_avatar);

        // Setup trigger detection with the assigned entity_id
        // NOTE: This must be called AFTER add_child so that _ready() has been called
        // and the @onready trigger_detector variable is initialized
        new_avatar.call(
            "setup_trigger_detection",
            &[entity_id.as_i32().to_variant()],
        );

        self.avatar_godot_scene
            .insert(entity_id, new_avatar.clone());

        // Emit signal for the new avatar
        self.base_mut()
            .emit_signal("avatar_added", &[new_avatar.to_variant()]);

        // Emit signal with updated avatar list (for backwards compatibility)
        let avatars = self.get_avatars();
        self.base_mut()
            .emit_signal("avatar_scene_changed", &[avatars.to_variant()]);
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
    pub fn get_avatars(&self) -> Array<Gd<DclAvatar>> {
        Array::from_iter(self.avatar_godot_scene.values().cloned())
    }

    #[func]
    pub fn get_avatars_count(&self) -> i32 {
        self.avatar_godot_scene.len() as i32
    }

    #[func]
    pub fn on_scene_spawned(&mut self, _scene_id: i32, _entity_id: GString) {
        for (_, avatar) in self.avatar_godot_scene.iter_mut() {
            avatar.bind_mut().on_parcel_scenes_changed();
        }
    }

    #[func]
    fn on_avatar_trigger_emote(&self, emote_id: GString, looping: bool, avatar_entity_id: i32) {
        let avatar_entity_id = SceneEntityId::from_i32(avatar_entity_id);
        let avatar_scene = self
            .avatar_godot_scene
            .get(&avatar_entity_id)
            .expect("avatar not found");

        let mut scene_runner = DclGlobal::singleton().bind().scene_runner.clone();
        let mut scene_runner = scene_runner.bind_mut();

        let avatar_current_parcel_scene_id = avatar_scene.bind().get_current_parcel_scene_id();
        let avatar_active_scene_ids = {
            let mut scene_ids = scene_runner.get_global_scene_ids().clone();
            if avatar_current_parcel_scene_id != SceneId::INVALID.0 {
                scene_ids.push(SceneId(avatar_current_parcel_scene_id));
            }
            scene_ids
        };

        let emote_command = PbAvatarEmoteCommand {
            emote_urn: emote_id.to_string(),
            r#loop: looping,
            timestamp: 0,
        };

        // Push dirty state only in active scenes
        for scene_id in avatar_active_scene_ids {
            if let Some(scene) = scene_runner.get_scene_mut(&scene_id) {
                let emote_vector = scene
                    .avatar_scene_updates
                    .avatar_emote_command
                    .entry(avatar_entity_id)
                    .or_insert(Vec::new());

                emote_vector.push(emote_command.clone());
            }
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

    #[func]
    pub fn set_avatar_version_by_address(&mut self, address: GString, version: GString) {
        let address_str = address.to_string();
        if let Some(h160) = address_str.as_h160() {
            if let Some(alias) = self.avatar_address.get(&h160) {
                if let Some(entity_id) = self.avatar_entity.get(alias) {
                    if let Some(avatar) = self.avatar_godot_scene.get_mut(entity_id) {
                        avatar.call("set_client_version", &[version.to_variant()]);
                    }
                }
            }
        }
    }

    #[func]
    pub fn set_avatar_room_debug_by_address(&mut self, address: GString, room_info: GString) {
        let address_str = address.to_string();
        if let Some(h160) = address_str.as_h160() {
            if let Some(alias) = self.avatar_address.get(&h160) {
                if let Some(entity_id) = self.avatar_entity.get(alias) {
                    if let Some(avatar) = self.avatar_godot_scene.get_mut(entity_id) {
                        avatar.call("set_room_debug", &[room_info.to_variant()]);
                    }
                }
            }
        }
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

        let impostor_ids: Vec<i64> = self.impostor_slots.keys().copied().collect();
        for id in impostor_ids {
            self.clear_impostor(id);
        }

        let avatars = std::mem::take(&mut self.avatar_godot_scene);
        for (_, mut avatar) in avatars {
            self.base_mut()
                .remove_child(&avatar.clone().upcast::<Node>());
            avatar.queue_free()
        }
    }

    pub fn remove_avatar(&mut self, alias: u32) {
        if let Some(entity_id) = self.avatar_entity.remove(&alias) {
            if let Some(avatar) = self.avatar_godot_scene.get(&entity_id) {
                let impostor_id = avatar.instance_id().to_i64();
                self.clear_impostor(impostor_id);
            }
            self.crdt_state.kill_entity(&entity_id);
            let mut avatar = self.avatar_godot_scene.remove(&entity_id).unwrap();

            // Get the address before removing it from the map
            let removed_address: Option<H160> = self
                .avatar_address
                .iter()
                .find(|(_, &v)| v == alias)
                .map(|(k, _)| *k);

            self.avatar_address.retain(|_, v| *v != alias);

            self.last_updated_profile.remove(&entity_id);
            self.last_movement_timestamp.remove(&alias);
            self.last_position_index.remove(&alias);

            // Remove from tree first, then queue_free (correct order)
            self.base_mut()
                .remove_child(&avatar.clone().upcast::<Node>());
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

            // Emit signal for the removed avatar with its address
            if let Some(address) = removed_address {
                let address_str = format!("{:#x}", address);
                self.base_mut()
                    .emit_signal("avatar_removed", &[address_str.to_variant()]);
            }

            // Emit signal with updated avatar list (for backwards compatibility)
            let avatars = self.get_avatars();
            self.base_mut()
                .emit_signal("avatar_scene_changed", &[avatars.to_variant()]);
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
            let mut scene_ids = scene_runner.get_global_scene_ids().clone();
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
    ) -> bool {
        let entity_id = if let Some(entity_id) = self.avatar_entity.get(&alias) {
            *entity_id
        } else {
            // TODO: handle this condition
            return false;
        };

        // Skip position messages if we have movement messages (Movement has priority)
        if self.last_movement_timestamp.contains_key(&alias) {
            return false;
        }

        // Check position index to ensure we only process newer positions
        if let Some(last_index) = self.last_position_index.get(&alias) {
            if transform.index <= *last_index {
                return false; // Skip if the position index is not newer than the last one
            }
        }

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
        self.last_position_index.insert(alias, transform.index);
        true
    }

    pub fn update_avatar_transform_with_movement(
        &mut self,
        alias: u32,
        movement: &rfc4::Movement,
    ) -> bool {
        let entity_id = if let Some(entity_id) = self.avatar_entity.get(&alias) {
            *entity_id
        } else {
            // TODO: handle this condition
            tracing::warn!("Avatar with alias {} not found", alias);
            return false;
        };

        // Discard if movement.timestamp is older than the last one (with tolerance)
        const TIMESTAMP_TOLERANCE: f32 = 0.001;
        if let Some(last_timestamp) = self.last_movement_timestamp.get(&alias) {
            // Only discard if the new timestamp is significantly older
            if movement.timestamp < *last_timestamp - TIMESTAMP_TOLERANCE {
                return false;
            }
            // If timestamps are nearly identical (within tolerance), also skip to avoid duplicate processing
            if (movement.timestamp - *last_timestamp).abs() < TIMESTAMP_TOLERANCE {
                return false;
            }
        }

        // rotation_y on the wire is the yaw in DCL/Unity (left-handed) space,
        // in degrees — matching Unity Foundation Client's rfc4.Movement
        // encoding (transform.eulerAngles.y). Convert to radians, then build
        // the DCL-space quaternion directly. to_godot_transform_3d flips z/w
        // at render time to convert to Godot's right-handed space, mirroring
        // how Position packets handle translation.z.
        let dcl_quat = godot::prelude::Quaternion::from_euler(godot::prelude::Vector3 {
            x: 0.0,
            y: movement.rotation_y.to_radians(),
            z: 0.0,
        });

        let dcl_transform = DclTransformAndParent {
            translation: godot::prelude::Vector3 {
                x: movement.position_x,
                y: movement.position_y,
                z: movement.position_z,
            },
            rotation: dcl_quat,
            scale: godot::prelude::Vector3::ONE,
            parent: SceneEntityId::ROOT,
        };

        self._update_avatar_transform(&entity_id, dcl_transform);
        // Wire-authoritative animation state for remote double-jump / glide.
        if let Some(avatar) = self.avatar_godot_scene.get_mut(&entity_id) {
            avatar.bind_mut().apply_wire_movement_state(
                movement.jump_count,
                movement.glide_state,
                movement.is_grounded,
            );
        }
        self.last_movement_timestamp
            .insert(alias, movement.timestamp);
        true
    }

    pub fn update_avatar_transform_with_movement_compressed(
        &mut self,
        alias: u32,
        position: godot::prelude::Vector3,
        rotation_rad: f32,
        timestamp: f32,
    ) -> bool {
        let entity_id = if let Some(entity_id) = self.avatar_entity.get(&alias) {
            *entity_id
        } else {
            // TODO: handle this condition
            tracing::warn!("Avatar with alias {} not found", alias);
            return false;
        };

        // Discard if timestamp is older than the last one (with tolerance)
        const TIMESTAMP_TOLERANCE: f32 = 0.001;
        if let Some(last_timestamp) = self.last_movement_timestamp.get(&alias) {
            // Only discard if the new timestamp is significantly older
            if timestamp < *last_timestamp - TIMESTAMP_TOLERANCE {
                return false;
            }
            // If timestamps are nearly identical (within tolerance), also skip to avoid duplicate processing
            if (timestamp - *last_timestamp).abs() < TIMESTAMP_TOLERANCE {
                return false;
            }
        }

        // rotation_rad is the yaw in DCL/Unity (left-handed) space. See
        // update_avatar_transform_with_movement for the rationale.
        let dcl_quat = godot::prelude::Quaternion::from_euler(godot::prelude::Vector3 {
            x: 0.0,
            y: rotation_rad,
            z: 0.0,
        });

        let dcl_transform = DclTransformAndParent {
            translation: position,
            rotation: dcl_quat,
            scale: godot::prelude::Vector3::ONE,
            parent: SceneEntityId::ROOT,
        };

        self._update_avatar_transform(&entity_id, dcl_transform);
        self.last_movement_timestamp.insert(alias, timestamp);
        true
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

    pub fn set_avatar_blocked(&mut self, alias: u32, blocked: bool) {
        if let Some(entity_id) = self.avatar_entity.get(&alias) {
            if let Some(avatar) = self.avatar_godot_scene.get_mut(entity_id) {
                avatar.call("set_blocked_and_hidden", &[blocked.to_variant()]);
            }
        }
    }

    pub fn set_avatar_blocked_by_address(&mut self, address: &H160, blocked: bool) {
        if let Some(alias) = self.avatar_address.get(address) {
            self.set_avatar_blocked(*alias, blocked);
        }
    }

    pub fn play_emote(&mut self, alias: u32, incremental_id: u32, emote_urn: &String) {
        let entity_id = if let Some(entity_id) = self.avatar_entity.get(&alias) {
            *entity_id
        } else {
            return;
        };

        // Discard if the emote is less than or equal to the last played emote
        if let Some(last_incremental_id) = self.last_emote_incremental_id.get(&alias) {
            if incremental_id <= *last_incremental_id {
                tracing::debug!(
                    "Discarding emote {} for alias {}: incremental_id {} <= last_emote_incremental_id {}",
                    emote_urn,
                    alias,
                    incremental_id,
                    last_incremental_id
                );
                return;
            }
        }

        // Store the last emote incremental ID for this alias
        self.last_emote_incremental_id.insert(alias, incremental_id);

        if let Some(avatar_scene) = self.avatar_godot_scene.get_mut(&entity_id) {
            avatar_scene.call("async_play_emote", &[emote_urn.to_variant()]);
        }
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
                "async_update_avatar_from_profile",
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
            "spawn_voice_channel",
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
            .call("push_voice_frame", &[frame.to_variant()]);
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

        // Transforms
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
                sync_crdt_lww_component(
                    entity_id,
                    target_transform_component,
                    local_transform_component,
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
            sync_crdt_lww_component(
                entity_id,
                target_player_identity_data,
                local_player_identity_data,
            );

            let target_avatar_base =
                SceneCrdtStateProtoComponents::get_avatar_base_mut(target_crdt_state);
            sync_crdt_lww_component(entity_id, target_avatar_base, local_avatar_base);

            let target_avatar_equipped_data =
                SceneCrdtStateProtoComponents::get_avatar_equipped_data_mut(target_crdt_state);
            sync_crdt_lww_component(
                entity_id,
                target_avatar_equipped_data,
                local_avatar_equipped_data,
            );
        }

        let entity_id = &SceneEntityId::PLAYER;
        let target_player_identity_data =
            SceneCrdtStateProtoComponents::get_player_identity_data_mut(target_crdt_state);
        sync_crdt_lww_component(
            entity_id,
            target_player_identity_data,
            local_player_identity_data,
        );

        let target_avatar_base =
            SceneCrdtStateProtoComponents::get_avatar_base_mut(target_crdt_state);
        sync_crdt_lww_component(entity_id, target_avatar_base, local_avatar_base);

        let target_avatar_equipped_data =
            SceneCrdtStateProtoComponents::get_avatar_equipped_data_mut(target_crdt_state);
        sync_crdt_lww_component(
            entity_id,
            target_avatar_equipped_data,
            local_avatar_equipped_data,
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
