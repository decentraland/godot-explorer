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

const IMPOSTOR_MAX_LAYERS: u32 = 128;
// Multimesh instance count is decoupled from layer count. Beyond MAX_LAYERS,
// extra avatars get an "overflow" slot that borrows another slot's texture and
// renders fully tinted (looks like a distant silhouette).
const IMPOSTOR_MAX_INSTANCES: u32 = 256;
const IMPOSTOR_TEX_WIDTH: i32 = 256;
const IMPOSTOR_TEX_HEIGHT: i32 = 512;
// Quad world-space size matches the AvatarPreview ortho capture
// (256x512 px @ ortho_size=2.5 → 1.25m W × 2.5m H).
const IMPOSTOR_QUAD_WIDTH: f32 = 1.25;
const IMPOSTOR_QUAD_HEIGHT: f32 = 2.5;
const IMPOSTOR_VERTICAL_OFFSET: f32 = 1.0;
const IMPOSTOR_SHADER_PATH: &str = "res://assets/avatar/impostor.gdshader";

// Disk cache. Keyed by lowercase eth address; PNGs live in user://impostor_cache.
// Entries idle longer than IMPOSTOR_CACHE_TTL_MS get evicted on the next
// cleanup tick, which itself runs at most every IMPOSTOR_CACHE_CLEANUP_MS.
const IMPOSTOR_CACHE_DIR: &str = "user://impostor_cache";
const IMPOSTOR_CACHE_TTL_MS: i64 = 5 * 60 * 1000;
const IMPOSTOR_CACHE_CLEANUP_MS: i64 = 60 * 1000;

#[derive(Clone, Debug)]
struct ImpostorSlot {
    // Index into the multimesh (0..IMPOSTOR_MAX_INSTANCES). Independent of
    // layer_index: overflow slots get an instance but borrow another slot's
    // layer.
    instance_index: u32,
    layer_index: u32,
    // Layer to sample when this slot can't render its own (overflow, or real
    // slot whose first capture hasn't landed yet). Set once at allocation and
    // only refreshed when the lender's layer becomes invalid — keeps the
    // borrowed silhouette stable frame-to-frame instead of remapping every
    // time loaded_layers.len() changes.
    borrow_layer_hint: u32,
    // Whether this slot exclusively owns layer_index. False for pure overflow
    // slots that never received a real allocation. Demoted slots (real that
    // became overflow due to camera turning away) still own their layer so
    // their captured texture stays warm for instant flip-back.
    owns_layer: bool,
    // Render mode flag: when true, sample texture is rendered with tint=1.0
    // (black silhouette). Independent of owns_layer.
    is_overflow: bool,
    fade_alpha: f32,
    tint_strength: f32,
    texture_loaded: bool,
    avatar_instance_id: InstanceId,
    distance_sq: f32,
    // Frame index of the last time this slot rendered with fade_alpha>0. Used
    // by LRU eviction so that slots that are no longer being shown (e.g., the
    // user moved away or turned around) are evicted before recently-seen ones.
    last_seen_frame: u64,
    // Cache file key (lowercased) for this avatar. Used to keep the disk
    // PNG's last_used_ms timestamp fresh while the slot actively renders, so
    // entries in-use don't get TTL-evicted. Empty when the avatar didn't
    // provide an identity (no avatar_data, brand new avatar, etc.).
    cache_key: String,
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
    impostor_free_instances: Vec<u32>,
    // Disk-backed impostor texture cache. The actual PNGs live on disk under
    // user://impostor_cache/<avatar_id>.png; this map only tracks
    // last-access timestamps for TTL eviction. Keyed by lowercase eth address
    // so the cache survives cross-session.
    impostor_cache_last_used: HashMap<String, i64>,
    impostor_cache_last_cleanup_ms: i64,
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
            impostor_free_instances: (0..IMPOSTOR_MAX_INSTANCES).rev().collect(),
            impostor_cache_last_used: HashMap::new(),
            impostor_cache_last_cleanup_ms: 0,
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
        self.maybe_run_cache_cleanup();
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
        multimesh.set_instance_count(IMPOSTOR_MAX_INSTANCES as i32);

        let mut mmi = MultiMeshInstance3D::new_alloc();
        mmi.set_name("impostor_multimesh");
        mmi.set_multimesh(&multimesh);
        mmi.set_material_override(&shader_material.upcast::<godot::classes::Material>());
        mmi.set_cast_shadows_setting(
            godot::classes::geometry_instance_3d::ShadowCastingSetting::OFF,
        );

        for i in 0..IMPOSTOR_MAX_INSTANCES as i32 {
            multimesh.set_instance_transform(i, Transform3D::IDENTITY);
            multimesh.set_instance_custom_data(i, Color::from_rgba(0.0, 0.0, 0.0, 0.0));
        }

        self.base_mut().add_child(&mmi);
        self.impostor_multimesh = Some(mmi);
        self.impostor_texture_array = Some(texture_array);
        tracing::info!(
            "Impostor renderer initialized: {} instances, {} layers, {}x{} px",
            IMPOSTOR_MAX_INSTANCES,
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

        // Set of layers currently holding a captured texture, used both for
        // borrow-fallback rendering and for refreshing stale borrow hints.
        let loaded_layers: Vec<u32> = self
            .impostor_slots
            .values()
            .filter(|s| s.texture_loaded)
            .map(|s| s.layer_index)
            .collect();
        let loaded_set: std::collections::HashSet<u32> = loaded_layers.iter().copied().collect();

        // Pre-pass: refresh borrow_layer_hint for slots whose hint points to
        // a layer that's no longer loaded. Hints that are already valid stay
        // put — frame-to-frame stability prevents flicker when loaded_layers
        // grows. New assignments distribute via least-borrowed so we don't
        // clump everyone on a single silhouette early in capture warmup.
        if !loaded_layers.is_empty() {
            let stale_borrowers: Vec<i64> = self
                .impostor_slots
                .iter()
                .filter(|(_, s)| !s.texture_loaded && !loaded_set.contains(&s.borrow_layer_hint))
                .map(|(k, _)| *k)
                .collect();
            if !stale_borrowers.is_empty() {
                let mut counts: HashMap<u32, u32> = loaded_layers.iter().map(|l| (*l, 0)).collect();
                for s in self.impostor_slots.values() {
                    if !s.texture_loaded && loaded_set.contains(&s.borrow_layer_hint) {
                        if let Some(c) = counts.get_mut(&s.borrow_layer_hint) {
                            *c += 1;
                        }
                    }
                }
                for key in stale_borrowers {
                    let tie = key.unsigned_abs() as u32;
                    let new_hint = loaded_layers
                        .iter()
                        .min_by_key(|l| {
                            (
                                counts.get(*l).copied().unwrap_or(0),
                                l.wrapping_mul(0x9E37_79B9) ^ tie,
                            )
                        })
                        .copied()
                        .unwrap_or(loaded_layers[0]);
                    if let Some(slot) = self.impostor_slots.get_mut(&key) {
                        slot.borrow_layer_hint = new_hint;
                    }
                    *counts.entry(new_hint).or_insert(0) += 1;
                }
            }
        }

        // Bump the TTL on every cache_key whose slot is currently rendering a
        // valid texture. Without this, an avatar that's been on screen for
        // longer than IMPOSTOR_CACHE_TTL_MS would have its disk PNG evicted
        // even though it's actively in use, forcing a recapture the moment
        // it next loses and re-acquires its slot (e.g. quick frustum churn).
        let cache_now = Self::now_ms();
        for slot in self.impostor_slots.values() {
            if slot.texture_loaded && !slot.cache_key.is_empty() {
                self.impostor_cache_last_used
                    .insert(slot.cache_key.clone(), cache_now);
            }
        }

        let mut stale: Vec<i64> = Vec::new();
        for (key, slot) in self.impostor_slots.iter() {
            let Ok(avatar) = Gd::<DclAvatar>::try_from_instance_id(slot.avatar_instance_id) else {
                stale.push(*key);
                continue;
            };
            let avatar_pos = avatar.get_global_position();
            let mut transform = Transform3D::IDENTITY;
            transform.origin = avatar_pos + Vector3::new(0.0, IMPOSTOR_VERTICAL_OFFSET, 0.0);

            multimesh.set_instance_transform(slot.instance_index as i32, transform);

            let (render_layer, render_tint, render_alpha) = if slot.texture_loaded {
                // Slot has its own captured texture. Render with normal tint,
                // or full tint when the slot is in overflow render mode (was
                // demoted because the avatar moved off-screen / out-of-rank).
                let tint = if slot.is_overflow {
                    1.0
                } else {
                    slot.tint_strength
                };
                (slot.layer_index, tint, slot.fade_alpha)
            } else if loaded_set.contains(&slot.borrow_layer_hint) {
                // No own texture yet — render the borrowed lender as a
                // silhouette. The hint is stable across frames thanks to the
                // pre-pass above.
                (slot.borrow_layer_hint, 1.0, slot.fade_alpha)
            } else {
                // Nothing to borrow (very early startup, no captures yet).
                (0, 0.0, 0.0)
            };

            multimesh.set_instance_custom_data(
                slot.instance_index as i32,
                Color::from_rgba(render_tint, render_alpha, render_layer as f32, 0.0),
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
        allow_overflow: bool,
        avatar_id: GString,
    ) -> i32 {
        let now = godot::classes::Engine::singleton().get_frames_drawn() as u64;

        // Existing slot: toggle the render mode in place and, if we're
        // promoting a pure-overflow slot to real-tier, attach a real layer.
        // Demoting (real → overflow) keeps the existing layer so a camera
        // that swings back doesn't recapture; promoting from a borrow-only
        // slot needs an actual layer so the avatar can be rendered with
        // colour instead of as a black silhouette.
        if self.impostor_slots.contains_key(&impostor_id) {
            let needs_real_layer = {
                let slot = self.impostor_slots.get_mut(&impostor_id).expect("checked");
                slot.is_overflow = allow_overflow;
                slot.last_seen_frame = now;
                !allow_overflow && !slot.owns_layer
            };
            if needs_real_layer {
                let layer = self.alloc_layer();
                let cache_key = avatar_id.to_string().to_lowercase();
                let loaded = if !cache_key.is_empty() {
                    self.try_load_cached_texture(&cache_key, layer)
                } else {
                    false
                };
                let slot = self.impostor_slots.get_mut(&impostor_id).expect("checked");
                slot.layer_index = layer;
                slot.owns_layer = true;
                slot.texture_loaded = loaded;
                slot.cache_key = cache_key;
            }
            let inst = self.impostor_slots[&impostor_id].instance_index;
            return inst as i32;
        }

        let distance_sq = distance * distance;

        let instance_index = match self.alloc_instance() {
            Some(i) => i,
            None => return -1,
        };

        // Pure overflow alloc: no real layer, just an instance + a borrow
        // hint. Real alloc: dedicated layer, populated from disk cache when
        // possible so we skip a recapture; otherwise pick a borrow to render
        // as silhouette until the capture lands.
        let cache_key_str = avatar_id.to_string().to_lowercase();
        let (layer_index, owns_layer, texture_loaded) = if allow_overflow {
            (0u32, false, false)
        } else {
            let layer = self.alloc_layer();
            let loaded = if !cache_key_str.is_empty() {
                self.try_load_cached_texture(&cache_key_str, layer)
            } else {
                false
            };
            (layer, true, loaded)
        };
        let borrow_layer_hint = if !texture_loaded {
            self.pick_loaded_layer_for(impostor_id).unwrap_or(0)
        } else {
            0
        };

        self.impostor_slots.insert(
            impostor_id,
            ImpostorSlot {
                instance_index,
                layer_index,
                borrow_layer_hint,
                owns_layer,
                is_overflow: allow_overflow,
                fade_alpha: 0.0,
                tint_strength: 0.0,
                texture_loaded,
                avatar_instance_id: avatar.instance_id(),
                distance_sq,
                last_seen_frame: now,
                cache_key: cache_key_str,
            },
        );
        instance_index as i32
    }

    /// Pick the loaded layer with the fewest current borrowers (slots that
    /// don't have their own captured texture). Tie-broken by `impostor_id`
    /// hash so different slots hitting the same minimum spread across
    /// candidates. Distributes silhouettes across all available lenders
    /// instead of clumping them on whichever layer wins the hash modulo when
    /// `loaded.len()` is small (early startup).
    fn pick_loaded_layer_for(&self, impostor_id: i64) -> Option<u32> {
        let loaded: Vec<u32> = self
            .impostor_slots
            .values()
            .filter(|s| s.texture_loaded)
            .map(|s| s.layer_index)
            .collect();
        if loaded.is_empty() {
            return None;
        }
        let mut counts: HashMap<u32, u32> = loaded.iter().map(|l| (*l, 0)).collect();
        for s in self.impostor_slots.values() {
            if !s.texture_loaded {
                if let Some(c) = counts.get_mut(&s.borrow_layer_hint) {
                    *c += 1;
                }
            }
        }
        let tie = impostor_id.unsigned_abs() as u32;
        loaded.into_iter().min_by_key(|l| {
            let c = counts.get(l).copied().unwrap_or(0);
            // Use the high bits of the layer hash to break ties among layers
            // with equal borrower count.
            (c, l.wrapping_mul(0x9E37_79B9) ^ tie)
        })
    }

    #[func]
    fn impostor_needs_capture(&self, impostor_id: i64) -> bool {
        self.impostor_slots
            .get(&impostor_id)
            .map(|s| s.owns_layer && !s.texture_loaded)
            .unwrap_or(false)
    }

    fn alloc_instance(&mut self) -> Option<u32> {
        if let Some(i) = self.impostor_free_instances.pop() {
            return Some(i);
        }
        // No free multimesh slots — evict the LRU slot. Use slot fields locally
        // to avoid borrowing self mutably while iterating.
        let lru_id = self
            .impostor_slots
            .iter()
            .min_by_key(|(_, s)| s.last_seen_frame)
            .map(|(id, _)| *id)?;
        self.clear_impostor(lru_id);
        self.impostor_free_instances.pop()
    }

    fn alloc_layer(&mut self) -> u32 {
        if let Some(l) = self.impostor_free_layers.pop() {
            return l;
        }
        // No free layers — steal the layer from the LRU layer-owning slot.
        // We don't fully remove the victim; we just release its layer so it
        // falls back to borrow-rendering. That preserves its multimesh
        // instance and avatar binding, so when LOD picks it up again it
        // simply gets a fresh layer rather than a fresh instance.
        let lru_id = self
            .impostor_slots
            .iter()
            .filter(|(_, s)| s.owns_layer)
            .min_by_key(|(_, s)| s.last_seen_frame)
            .map(|(id, _)| *id);
        if let Some(id) = lru_id {
            if let Some(slot) = self.impostor_slots.get_mut(&id) {
                let freed = slot.layer_index;
                slot.owns_layer = false;
                slot.texture_loaded = false;
                slot.layer_index = 0;
                return freed;
            }
        }
        // Last resort: nothing to evict. Caller gets layer 0 (will render
        // garbage briefly until a real layer is reclaimed).
        0
    }

    #[func]
    fn set_impostor_texture(&mut self, impostor_id: i64, image: Gd<Image>, avatar_id: GString) {
        let Some(slot) = self.impostor_slots.get_mut(&impostor_id) else {
            tracing::warn!(
                "set_impostor_texture: no slot for impostor_id {}",
                impostor_id
            );
            return;
        };
        if !slot.owns_layer {
            // Slot doesn't have a real layer (pure overflow, or LRU-evicted
            // before the capture finished). Drop the image; another capture
            // will be requested if/when a layer is allocated again.
            return;
        }
        let layer_index = slot.layer_index;
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

        tex_array.update_layer(&img, layer_index as i32);
        let cache_key = avatar_id.to_string().to_lowercase();
        if let Some(slot) = self.impostor_slots.get_mut(&impostor_id) {
            slot.texture_loaded = true;
            slot.cache_key = cache_key.clone();
        }

        // Mirror to disk so the avatar doesn't need to re-capture next time
        // it gets a real slot (after frustum churn or session restart).
        if !cache_key.is_empty() {
            self.save_cached_texture(&cache_key, &img);
        }
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
            self.impostor_free_instances.push(slot.instance_index);
            // Pure overflow slots only borrowed a layer; the lender still
            // owns it. Real (or formerly-real) slots return their layer to
            // the pool so a future allocation can claim it.
            if slot.owns_layer {
                self.impostor_free_layers.push(slot.layer_index);
            }
            if let Some(mmi) = self.impostor_multimesh.as_ref().cloned() {
                if let Some(mut multimesh) = mmi.get_multimesh() {
                    multimesh.set_instance_custom_data(
                        slot.instance_index as i32,
                        Color::from_rgba(0.0, 0.0, 0.0, 0.0),
                    );
                }
            }
        }
    }

    #[func]
    fn invalidate_impostor_texture(&mut self, impostor_id: i64, avatar_id: GString) {
        // Don't flip texture_loaded — keep rendering the (slightly stale)
        // pixels in the layer until the new capture lands. With many avatars
        // invalidating in lockstep (e.g. all remote profiles fetched at once),
        // mass-clearing texture_loaded would briefly empty loaded_layers and
        // every impostor would render either invisible or as a borrowed
        // silhouette for the full duration of the capture queue. The layer's
        // pixel data is still valid; the caller is responsible for queuing a
        // fresh capture which will overwrite it.
        let _ = impostor_id;
        let cache_key = avatar_id.to_string().to_lowercase();
        if !cache_key.is_empty() {
            self.delete_cached_texture(&cache_key);
        }
    }

    #[func]
    fn has_impostor_capacity(&self) -> bool {
        !self.impostor_free_instances.is_empty()
    }

    #[func]
    fn impostor_texture_size(&self) -> Vector2i {
        Vector2i::new(IMPOSTOR_TEX_WIDTH, IMPOSTOR_TEX_HEIGHT)
    }

    #[func]
    fn impostor_max_layers(&self) -> i32 {
        IMPOSTOR_MAX_LAYERS as i32
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

    /// Debug: list every avatar currently tracked by `AvatarScene`, plus the
    /// local player avatar (if present). Each entry is a Dictionary with
    /// `entity_id`, `alias`, `address`, `name`, `position` (Vector3),
    /// `instance_id`, and `is_local` (bool). Used by the debug WS `avatars`
    /// command. The avatar tree lives outside per-scene CRDT state so it
    /// gets its own commands rather than overloading `scene`/`entity`.
    ///
    /// Note: `avatar_godot_scene` only tracks remote players (filled by
    /// `add_avatar` from comms). The local player is held separately on
    /// `SceneManager.player_avatar_node` with its profile under
    /// `last_updated_profile[PLAYER]`; we synthesize a row for it here so
    /// solo-debugging sessions still see "yourself".
    #[func]
    fn debug_list_avatars(&self) -> Array<VarDictionary> {
        let mut out = Array::new();

        // Local player first (so the row order is stable: yourself, then
        // remote players by entity_id).
        if let Some(d) = self.debug_local_player_row() {
            out.push(&d);
        }

        // Build entity_id → alias and alias → &H160 reverse maps so we can
        // resolve identity from the entity_id we iterate.
        let entity_to_alias: HashMap<SceneEntityId, AvatarAlias> =
            self.avatar_entity.iter().map(|(&a, &e)| (e, a)).collect();
        let alias_to_address: HashMap<AvatarAlias, &H160> = self
            .avatar_address
            .iter()
            .map(|(addr, &a)| (a, addr))
            .collect();

        for (entity_id, avatar) in self.avatar_godot_scene.iter() {
            let alias = entity_to_alias.get(entity_id).copied().unwrap_or(0);
            let address_str = alias_to_address
                .get(&alias)
                .map(|h| format!("{:?}", h))
                .unwrap_or_default();
            let name = self
                .last_updated_profile
                .get(entity_id)
                .map(|p| p.content.name.clone())
                .unwrap_or_default();
            let pos = avatar.get_global_position();

            let mut d = VarDictionary::new();
            d.set("entity_id", entity_id.as_i32());
            d.set("alias", alias as i64);
            d.set("address", address_str);
            d.set("name", name);
            d.set("position", pos);
            d.set("instance_id", avatar.instance_id().to_i64());
            d.set("is_local", false);
            out.push(&d);
        }
        out
    }

    /// Builds the local player row for `debug_list_avatars`. Identity comes
    /// from `last_updated_profile[PLAYER]`; transform comes from
    /// `SceneManager.player_avatar_node`. Returns None if either is missing
    /// (e.g. between session-close and the next sign-in).
    fn debug_local_player_row(&self) -> Option<VarDictionary> {
        let player = SceneEntityId::PLAYER;
        let profile = self.last_updated_profile.get(&player)?;
        let scene_runner = DclGlobal::singleton().bind().scene_runner.clone();
        let player_node = scene_runner.bind().get_player_avatar_node();
        if !player_node.is_instance_valid() {
            return None;
        }
        let pos = player_node.get_global_position();

        let mut d = VarDictionary::new();
        d.set("entity_id", player.as_i32());
        // The local player has no comms-side alias; expose 0 so the field is
        // always present and the WS protocol stays homogeneous.
        d.set("alias", 0_i64);
        d.set("address", profile.content.eth_address.clone());
        d.set("name", profile.content.name.clone());
        d.set("position", pos);
        d.set("instance_id", player_node.instance_id().to_i64());
        d.set("is_local", true);
        Some(d)
    }

    /// Debug: returns the Godot `InstanceId` of the *local* player's avatar
    /// node, or `-1` if it's been freed (e.g. just signed out). Pairs with
    /// the `avatar` command's `by: "local"` mode.
    #[func]
    fn debug_get_local_player_instance_id(&self) -> i64 {
        let scene_runner = DclGlobal::singleton().bind().scene_runner.clone();
        let player_node = scene_runner.bind().get_player_avatar_node();
        if !player_node.is_instance_valid() {
            return -1;
        }
        player_node.instance_id().to_i64()
    }

    /// Debug: returns the Godot `InstanceId` (as i64) of the avatar matching
    /// `address`, or `-1` if no avatar has that address.
    #[func]
    fn debug_get_avatar_instance_id_by_address(&self, address: GString) -> i64 {
        let Some(addr) = address.to_string().as_h160() else {
            return -1;
        };
        let Some(&alias) = self.avatar_address.get(&addr) else {
            return -1;
        };
        let Some(entity_id) = self.avatar_entity.get(&alias) else {
            return -1;
        };
        match self.avatar_godot_scene.get(entity_id) {
            Some(avatar) => avatar.instance_id().to_i64(),
            None => -1,
        }
    }

    /// Debug: returns the Godot `InstanceId` (as i64) of the avatar matching
    /// `alias`, or `-1` if not found.
    #[func]
    fn debug_get_avatar_instance_id_by_alias(&self, alias: i64) -> i64 {
        let alias = alias as u32;
        let Some(entity_id) = self.avatar_entity.get(&alias) else {
            return -1;
        };
        match self.avatar_godot_scene.get(entity_id) {
            Some(avatar) => avatar.instance_id().to_i64(),
            None => -1,
        }
    }

    /// Debug: returns the Godot `InstanceId` (as i64) of the avatar matching
    /// `entity_id` (in the avatar-internal SceneEntityId space), or `-1` if
    /// not found.
    #[func]
    fn debug_get_avatar_instance_id_by_entity(&self, entity_id: i32) -> i64 {
        let entity_id = SceneEntityId::from_i32(entity_id);
        match self.avatar_godot_scene.get(&entity_id) {
            Some(avatar) => avatar.instance_id().to_i64(),
            None => -1,
        }
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

        // Cached component values for this avatar, used to (re)populate the scene
        // being entered. `@dcl/sdk/players` derives onEnterScene from an entity having
        // both PlayerIdentityData and AvatarBase, and onLeaveScene from AvatarBase
        // being removed — so scene membership must add/remove these components.
        let avatar_base = SceneCrdtStateProtoComponents::get_avatar_base(&self.crdt_state)
            .get(&avatar_entity_id)
            .and_then(|v| v.value.clone());
        let player_identity_data =
            SceneCrdtStateProtoComponents::get_player_identity_data(&self.crdt_state)
                .get(&avatar_entity_id)
                .and_then(|v| v.value.clone());
        let avatar_equipped_data =
            SceneCrdtStateProtoComponents::get_avatar_equipped_data(&self.crdt_state)
                .get(&avatar_entity_id)
                .and_then(|v| v.value.clone());

        let mut scene_runner = DclGlobal::singleton().bind().scene_runner.clone();
        let mut scene_runner = scene_runner.bind_mut();

        // Leaving the previous scene: clear the avatar's components there so the SDK's
        // onLeaveScene fires and the player drops from getEntitiesWith(PlayerIdentityData).
        // Routed through `deleted_entities`, which sets these components to None — the
        // departure cannot be communicated as an entity death (the renderer→scene path
        // never carries entity deaths). See `update_avatar_scene_updates`.
        if let Some(prev_scene) = scene_runner.get_scene_mut(&prev_scene_id) {
            prev_scene
                .avatar_scene_updates
                .deleted_entities
                .insert(avatar_entity_id);
        }

        // Entering the new scene: (re)populate the avatar's components so onEnterScene
        // fires and the player appears in getEntitiesWith(PlayerIdentityData, AvatarBase).
        if let Some(scene) = scene_runner.get_scene_mut(&scene_id) {
            let dcl_transform = DclTransformAndParent::default(); // TODO: get real transform with scene_offset

            scene
                .avatar_scene_updates
                .transform
                .insert(avatar_entity_id, Some(dcl_transform));
            scene
                .avatar_scene_updates
                .internal_player_data
                .insert(avatar_entity_id, InternalPlayerData { inside: true });
            if let Some(avatar_base) = avatar_base {
                scene
                    .avatar_scene_updates
                    .avatar_base
                    .insert(avatar_entity_id, avatar_base);
            }
            if let Some(player_identity_data) = player_identity_data {
                scene
                    .avatar_scene_updates
                    .player_identity_data
                    .insert(avatar_entity_id, player_identity_data);
            }
            if let Some(avatar_equipped_data) = avatar_equipped_data {
                scene
                    .avatar_scene_updates
                    .avatar_equipped_data
                    .insert(avatar_entity_id, avatar_equipped_data);
            }
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
    fn cache_path_for(key: &str) -> String {
        format!("{}/{}.png", IMPOSTOR_CACHE_DIR, key)
    }

    fn ensure_cache_dir() {
        let mut da = match godot::classes::DirAccess::open("user://") {
            Some(d) => d,
            None => return,
        };
        if !da.dir_exists("impostor_cache") {
            let _ = da.make_dir("impostor_cache");
        }
    }

    fn now_ms() -> i64 {
        godot::classes::Time::singleton().get_ticks_msec() as i64
    }

    /// Try to upload a cached PNG into the given layer. Returns true on hit.
    /// The decoded Image is dropped after upload so it doesn't sit in RAM.
    fn try_load_cached_texture(&mut self, cache_key: &str, layer_index: u32) -> bool {
        let path = Self::cache_path_for(cache_key);
        let img_opt = Image::load_from_file(&path);
        let Some(mut img) = img_opt else {
            return false;
        };
        if img.is_empty() {
            return false;
        }
        if img.get_width() != IMPOSTOR_TEX_WIDTH || img.get_height() != IMPOSTOR_TEX_HEIGHT {
            img.resize(IMPOSTOR_TEX_WIDTH, IMPOSTOR_TEX_HEIGHT);
        }
        if img.get_format() != Format::RGBA8 {
            img.convert(Format::RGBA8);
        }
        let mip_err = img.generate_mipmaps();
        if mip_err != godot::global::Error::OK {
            return false;
        }
        let Some(tex_array) = self.impostor_texture_array.as_mut() else {
            return false;
        };
        tex_array.update_layer(&img, layer_index as i32);
        self.impostor_cache_last_used
            .insert(cache_key.to_string(), Self::now_ms());
        true
    }

    fn save_cached_texture(&mut self, cache_key: &str, image: &Gd<Image>) {
        Self::ensure_cache_dir();
        let path = Self::cache_path_for(cache_key);
        let err = image.clone().save_png(&path);
        if err == godot::global::Error::OK {
            self.impostor_cache_last_used
                .insert(cache_key.to_string(), Self::now_ms());
        } else {
            tracing::warn!("Failed to save impostor PNG to {}: {:?}", path, err);
        }
    }

    fn delete_cached_texture(&mut self, cache_key: &str) {
        let path = Self::cache_path_for(cache_key);
        if let Some(mut da) = godot::classes::DirAccess::open(IMPOSTOR_CACHE_DIR) {
            let _ = da.remove(&format!("{}.png", cache_key));
        } else {
            // fall back to absolute remove if open failed
            let _ = godot::classes::DirAccess::remove_absolute(&path);
        }
        self.impostor_cache_last_used.remove(cache_key);
    }

    fn maybe_run_cache_cleanup(&mut self) {
        let now = Self::now_ms();
        if now - self.impostor_cache_last_cleanup_ms < IMPOSTOR_CACHE_CLEANUP_MS {
            return;
        }
        self.impostor_cache_last_cleanup_ms = now;
        let stale: Vec<String> = self
            .impostor_cache_last_used
            .iter()
            .filter(|(_, &ts)| now - ts > IMPOSTOR_CACHE_TTL_MS)
            .map(|(k, _)| k.clone())
            .collect();
        for key in stale {
            self.delete_cached_texture(&key);
        }
    }

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
