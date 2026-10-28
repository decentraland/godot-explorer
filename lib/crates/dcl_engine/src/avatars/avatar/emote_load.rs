//! Emote loading: wearable and scene emotes land in the same table, keyed by urn, with their
//! clip (and prop) merged into the avatar's `emotes` animation library.

use std::collections::HashMap;

use godot::classes::{Animation, AudioStream};
use godot::prelude::*;

use super::emote::{EmoteItemData, EmoteRequest, EMOTE_COOLDOWN_SECONDS};
use crate::avatars::emotes::{self, SceneEmoteUrn};
use crate::avatars::loaders::fetch_emote_definition;
use crate::avatars::wearables;
use crate::content::content_mapping::DclContentMappingAndUrl;
use crate::content::gltf::emote::DclEmoteGltf;
use crate::engine::local_task::{await_all, next_frame, spawn_local};
use crate::fetcher::globals;
use crate::godot_classes::dcl_avatar::DclAvatar;
use crate::godot_classes::promise::Promise;

/// What the scene that triggered a scene emote told us about its content.
#[derive(Default)]
pub struct SceneEmoteContent {
    pub base_url: String,
    /// `glb_hash -> audio_hash`
    pub emotes: HashMap<String, String>,
}

pub struct ResolvedSceneEmote {
    pub glb_hash: String,
    pub base_url: String,
    pub audio_hash: String,
    pub looping: bool,
}

fn now_seconds() -> f64 {
    godot::classes::Time::singleton().get_ticks_msec() as f64 / 1000.0
}

fn alive(avatar: &Gd<DclAvatar>) -> bool {
    avatar.is_instance_valid() && avatar.is_inside_tree()
}

impl DclAvatar {
    pub fn register_scene_emote_content(
        &mut self,
        scene_id: &str,
        base_url: &str,
        glb_hash: &str,
        audio_hash: &str,
    ) {
        let entry = self
            .scene_emote_registry
            .entry(scene_id.to_string())
            .or_insert_with(|| SceneEmoteContent {
                base_url: base_url.to_string(),
                emotes: HashMap::new(),
            });
        entry
            .emotes
            .insert(glb_hash.to_string(), audio_hash.to_string());
    }

    /// Content registered for a scene emote, or the realm url for remote players (no audio).
    fn scene_emote_info(&self, scene_id: &str, glb_hash: &str) -> (String, String) {
        if let Some(scene) = self.scene_emote_registry.get(scene_id) {
            if let Some(audio) = scene.emotes.get(glb_hash) {
                return (scene.base_url.clone(), audio.clone());
            }
        }
        let base = globals::realm().bind().get_content_base_url().to_string();
        (base, String::new())
    }

    /// Resolves a scene-emote urn against the registry Rust filled right before playing. Matching
    /// the urn is unambiguous where dash-splitting is not (preview hashes are `b64-<base64>`).
    fn find_scene_emote_by_urn(&self, urn: &str) -> Option<ResolvedSceneEmote> {
        for (scene_id, scene) in &self.scene_emote_registry {
            for (glb_hash, audio_hash) in &scene.emotes {
                for looping in [false, true] {
                    let candidate = format!(
                        "urn:decentraland:off-chain:scene-emote:{scene_id}-{glb_hash}-{looping}"
                    );
                    if candidate == urn {
                        return Some(ResolvedSceneEmote {
                            glb_hash: glb_hash.clone(),
                            base_url: scene.base_url.clone(),
                            audio_hash: audio_hash.clone(),
                            looping,
                        });
                    }
                }
            }
        }
        None
    }

    /// Stores a wearable emote's clips. `from_scene`/`looping` are for scene emotes.
    pub fn load_emote_from_gltf(
        &mut self,
        urn: &str,
        gltf: &Gd<DclEmoteGltf>,
        file_hash: &str,
        from_scene: bool,
        looping: bool,
    ) {
        if self.emote.loaded.contains_key(urn) {
            return;
        }
        let Some(mut library) = self.emote.nodes.as_ref().map(|n| n.library.clone()) else {
            return;
        };
        // Adding a clip only extends the mixer's caches, so unlike a removal
        // (`clean_unused_emotes`) this neither stops the tree nor resets the emote nodes.
        let gltf = gltf.bind();
        let mut armature_prop: Option<Gd<Node3D>> = None;
        if let Some(prop) = gltf.get_armature_prop() {
            // Only a node named Armature_Prop*: some emotes name their root plain "Armature",
            // which would collide with (and later free) the avatar's own skeleton subtree.
            let prop_name = prop.get_name().to_string();
            if prop_name.starts_with("Armature_Prop") {
                if self.base().has_node(prop_name.as_str()) {
                    armature_prop = self
                        .base()
                        .get_node_or_null(prop_name.as_str())
                        .and_then(|n| n.try_cast().ok());
                } else {
                    let mut prop = prop;
                    if let Some(mut parent) = prop.get_parent() {
                        parent.remove_child(&prop);
                    }
                    prop.set_owner(Gd::null_arg());
                    hotfix_legacy_prop_origin(&mut prop);
                    if let Some(mut anim_player) =
                        prop.try_get_node_as::<godot::classes::AnimationPlayer>("AnimationPlayer")
                    {
                        anim_player.stop();
                        anim_player.queue_free();
                    }
                    self.base_mut().add_child(&prop);
                    crate::avatars::mesh_assembler::apply_toon_material_recursive(
                        &prop.clone().upcast::<Node>(),
                    );
                    prop.hide();
                    if !self.emote.prop_armature_names.contains(&prop_name) {
                        self.emote.prop_armature_names.push(prop_name);
                    }
                    armature_prop = Some(prop);
                }
            }
        }

        let mut item = EmoteItemData {
            urn: urn.to_string(),
            default_anim_name: String::new(),
            file_hash: file_hash.to_string(),
            armature_prop,
            from_scene,
            looping,
        };

        if let Some(default_animation) = gltf.get_default_animation() {
            // Independent GLBs ship the same internal clip name ("Armature", "Animation"):
            // namespace by the content hash or the second emote silently reuses the first clip.
            let mut raw = default_animation.get_name().to_string();
            if raw.is_empty() {
                raw = if from_scene { "scene_emote" } else { "emote" }.to_string();
            }
            let anim_name = format!("{file_hash}_{raw}").replace(['/', ':'], "_");
            let final_animation = match gltf.get_prop_animation() {
                Some(prop_animation) => merge_animations(&default_animation, &prop_animation),
                None => default_animation.duplicate_resource(),
            };
            if !library.has_animation(anim_name.as_str()) {
                library.add_animation(anim_name.as_str(), &final_animation);
            }
            item.default_anim_name = anim_name;
        } else {
            godot_error!("default_animation is null for emote: {urn}");
        }
        self.emote.loaded.insert(urn.to_string(), item);
    }

    /// Re-enables the tree after the library changed, then plays what was queued meanwhile.
    fn reactivate_animation_system_later(&mut self, _was_active: bool) {
        let avatar = self.to_gd();
        spawn_local(async move {
            next_frame().await;
            if !avatar.is_instance_valid() {
                return;
            }
            let mut avatar = avatar;
            let queued = {
                let mut this = avatar.bind_mut();
                if let Some(nodes) = this.nodes.as_mut() {
                    nodes.animation_tree.set_active(true);
                }
                this.emote.is_modifying = false;
                this.emote.queued_play.take()
            };
            if let Some(queued) = queued {
                next_frame().await;
                if avatar.is_instance_valid() {
                    avatar
                        .bind_mut()
                        .play_emote(&queued.urn, queued.mask, queued.scene_id);
                }
            }
        });
    }

    /// Drops the emotes the avatar no longer carries (and their props).
    pub fn clean_unused_emotes(&mut self) {
        let keep_slots: Vec<String> = self
            .avatar_data
            .bind()
            .get_emotes()
            .as_slice()
            .iter()
            .map(|s| s.to_string())
            .collect();
        let to_delete: Vec<String> = self
            .emote
            .loaded
            .keys()
            .filter(|urn| !keep_slots.contains(urn))
            .cloned()
            .collect();
        if to_delete.is_empty() {
            return;
        }
        let (Some(mut tree), Some(mut player), Some(mut library)) = (
            self.nodes.as_ref().map(|n| n.animation_tree.clone()),
            self.nodes.as_ref().map(|n| n.animation_player.clone()),
            self.emote.nodes.as_ref().map(|n| n.library.clone()),
        ) else {
            return;
        };
        self.emote.is_modifying = true;
        let was_active = tree.is_active();
        tree.set_active(false);
        player.stop();
        self.reset_emote_nodes_to_idle();

        for urn in to_delete {
            let Some(item) = self.emote.loaded.remove(&urn) else {
                continue;
            };
            if library.has_animation(item.default_anim_name.as_str()) {
                library.remove_animation(item.default_anim_name.as_str());
            }
            if let Some(mut prop) = item.armature_prop {
                if prop.is_instance_valid() {
                    let name = prop.get_name().to_string();
                    self.emote.prop_armature_names.retain(|n| *n != name);
                    if prop.get_parent().as_ref() == Some(&self.to_gd().upcast::<Node>()) {
                        self.base_mut().remove_child(&prop);
                    }
                    prop.queue_free();
                }
            }
        }
        self.reactivate_animation_system_later(was_active);
    }

    /// Plays the sound of a loaded emote, found by its glb hash.
    pub fn play_emote_audio(&mut self, file_hash: &str) {
        if let Some(audio) = self.nodes.as_mut().map(|n| &mut n.audio_emote) {
            audio.stop();
        }
        let Some(urn) = self
            .emote
            .loaded
            .values()
            .find(|e| e.file_hash == file_hash)
            .map(|e| e.urn.clone())
        else {
            return;
        };
        let Some(emote) = globals::content_provider()
            .bind_mut()
            .get_wearable(urn.as_str().into())
        else {
            return;
        };
        let body_shape = self.avatar_data.bind().get_body_shape().to_string();
        let audio_file = emote.bind().get_emote_audio(body_shape);
        if audio_file.is_empty() {
            return;
        }
        let hash = emote
            .bind()
            .get_content_mapping()
            .bind()
            .get_hash(audio_file);
        let stream: Option<Gd<AudioStream>> = globals::content_provider()
            .bind_mut()
            .get_audio_from_hash(hash);
        if let (Some(stream), Some(audio)) =
            (stream, self.nodes.as_mut().map(|n| &mut n.audio_emote))
        {
            audio.play(stream);
        }
    }
}

/// Props baked before the prop-origin fix only had their basis flipped, so one authored at an
/// offset from the avatar ends up mirrored through it: complete the flip once and stamp it.
fn hotfix_legacy_prop_origin(prop: &mut Gd<Node3D>) {
    if prop.has_meta("prop_origin_corrected") && prop.get_meta("prop_origin_corrected").booleanize()
    {
        return;
    }
    let p = prop.get_position();
    prop.set_position(Vector3::new(-p.x, p.y, -p.z));
    prop.set_meta("prop_origin_corrected", &true.to_variant());
}

/// A clip with both the avatar's and the prop's tracks, the longer of the two lengths.
fn merge_animations(avatar_anim: &Gd<Animation>, prop_anim: &Gd<Animation>) -> Gd<Animation> {
    let mut merged: Gd<Animation> = avatar_anim.duplicate_resource();
    if prop_anim.get_length() > merged.get_length() {
        merged.set_length(prop_anim.get_length());
    }
    for track in 0..prop_anim.get_track_count() {
        let new_track = merged.add_track(prop_anim.track_get_type(track));
        merged.track_set_path(new_track, &prop_anim.track_get_path(track));
        merged
            .track_set_interpolation_type(new_track, prop_anim.track_get_interpolation_type(track));
        for key in 0..prop_anim.track_get_key_count(track) {
            merged
                .track_insert_key_ex(
                    new_track,
                    prop_anim.track_get_key_time(track, key),
                    &prop_anim.track_get_key_value(track, key),
                )
                .transition(prop_anim.track_get_key_transition(track, key))
                .done();
        }
    }
    merged
}

/// Loads (if needed) and plays an emote id or urn; wearable and scene emotes alike. While one
/// load is in flight only the newest request waits for the next one.
pub async fn async_play_emote(avatar: Gd<DclAvatar>, id: String, mask: i64, owner: i32) {
    if id.is_empty() {
        return;
    }
    // Other clients spell built-in emotes differently (#2986): Bevy sends
    // `...:base-scene-emotes:throw` where Unity and this client send `throw`.
    let id = emotes::normalize_emote_id(&id);
    let first;
    {
        let mut this = avatar.clone();
        let mut this = this.bind_mut();
        if this.is_loop_reannounce(&id, mask) {
            return;
        }
        let now = now_seconds();
        if now - this.emote.last_emote_time < EMOTE_COOLDOWN_SECONDS {
            return;
        }
        this.emote.last_emote_time = now;
        let urn = if id.starts_with("urn") {
            id.clone()
        } else if emotes::is_emote_utility(&id) {
            this.emote.play_seq += 1;
            this.emote.next_load = None;
            this.play_emote(&id, mask, owner);
            return;
        } else if emotes::is_emote_default(&id) {
            emotes::base_emote_urn(&id)
        } else {
            godot_error!("Unknown emote: {id}");
            return;
        };
        this.emote.play_seq += 1;
        if this.emote.loaded.contains_key(&urn) {
            this.emote.next_load = None;
            this.play_emote(&urn, mask, owner);
            return;
        }
        let request = EmoteRequest {
            urn,
            mask,
            scene_id: owner,
        };
        if this.emote.is_loading {
            this.emote.next_load = Some((request, this.emote.play_seq));
            return;
        }
        this.emote.is_loading = true;
        first = (request, this.emote.play_seq);
    }

    let mut next = Some(first);
    while let Some((request, seq)) = next {
        avatar.clone().bind_mut().emote.next_load = None;
        if !load_and_play(&avatar, request, seq).await {
            break;
        }
        next = avatar.clone().bind_mut().emote.next_load.take();
    }
    if avatar.is_instance_valid() {
        let mut this = avatar.clone();
        let mut this = this.bind_mut();
        this.emote.is_loading = false;
        this.emote.next_load = None;
    }
}

/// Loads the emote, then plays it unless a newer play or a stop arrived meanwhile. False when the
/// avatar left the tree during the load.
async fn load_and_play(avatar: &Gd<DclAvatar>, request: EmoteRequest, seq: u64) -> bool {
    let (stop_generation, loaded) = {
        let this = avatar.bind();
        (
            this.emote.stop_generation,
            this.emote.loaded.contains_key(&request.urn),
        )
    };
    if !loaded {
        async_load_emote(avatar.clone(), &request.urn).await;
    }
    if !alive(avatar) {
        return false;
    }
    next_frame().await;
    if !alive(avatar) {
        return false;
    }
    let mut handle = avatar.clone();
    let mut this = handle.bind_mut();
    // Superseded by a newer play, or stopped while downloading: starting it now would resurrect
    // a cancelled emote (loops: forever).
    if seq != this.emote.play_seq || stop_generation != this.emote.stop_generation {
        return true;
    }
    this.play_emote(&request.urn, request.mask, request.scene_id);
    true
}

/// Remote network avatars load emotes on first play; the local player, previews and scene NPCs
/// load every equipped emote up front.
pub fn loads_emotes_lazily(avatar: &Gd<DclAvatar>) -> bool {
    super::assembly::is_background(avatar)
}

/// Adds the equipped emotes to the library (unless lazy), then drops the unequipped ones.
pub async fn load_equipped_emotes(avatar: &Gd<DclAvatar>) {
    if !loads_emotes_lazily(avatar) {
        let (urns, body_shape, loader) = {
            let this = avatar.bind();
            let data = this.avatar_data.bind();
            let urns: Vec<String> = data
                .get_emotes()
                .as_slice()
                .iter()
                .map(|s| s.to_string())
                .filter(|urn| urn.starts_with("urn"))
                .collect();
            (
                urns,
                data.get_body_shape().to_string(),
                this.emote.loader.clone(),
            )
        };
        for urn in urns {
            let emote = globals::content_provider()
                .bind_mut()
                .get_wearable(urn.as_str().into());
            let Some(emote) = emote else {
                continue;
            };
            let hash = wearables::get_item_main_file_hash(&emote, &body_shape);
            if hash.is_empty() {
                continue;
            }
            let main_file = emote
                .bind()
                .get_representation_main_file(body_shape.clone())
                .to_string();
            let mapping = emote.bind().get_content_mapping();
            let gltf = loader
                .get_emote_gltf(&hash, &main_file, Some(&mapping), avatar)
                .await;
            if !avatar.is_instance_valid() {
                return;
            }
            if let Some(gltf) = gltf {
                avatar
                    .clone()
                    .bind_mut()
                    .load_emote_from_gltf(&urn, &gltf, &hash, false, false);
            }
        }
    }
    if avatar.is_instance_valid() {
        avatar.clone().bind_mut().clean_unused_emotes();
    }
}

/// Fetches and extracts one emote into the avatar's table.
pub async fn async_load_emote(avatar: Gd<DclAvatar>, urn: &str) {
    if emotes::is_scene_emote_urn(urn) {
        async_load_scene_emote(avatar, urn).await;
        return;
    }
    fetch_emote_definition(urn).await;
    if !avatar.is_instance_valid() {
        return;
    }
    let Some(emote) = globals::content_provider()
        .bind_mut()
        .get_wearable(urn.into())
    else {
        godot_error!("emote is null for urn: {urn}");
        return;
    };
    let body_shape = avatar
        .bind()
        .avatar_data
        .bind()
        .get_body_shape()
        .to_string();
    let file_hash = wearables::get_item_main_file_hash(&emote, &body_shape);
    if file_hash.is_empty() {
        godot_error!("file_hash is empty for emote: {urn}");
        return;
    }
    let loader = avatar.bind().emote.loader.clone();
    if loader.load_emote(urn, &body_shape).await.is_empty() {
        godot_error!("failed to load emote scene for: {urn}");
        return;
    }
    let main_file = emote
        .bind()
        .get_representation_main_file(body_shape.clone())
        .to_string();
    let mapping = emote.bind().get_content_mapping();
    let Some(gltf) = loader
        .get_emote_gltf(&file_hash, &main_file, Some(&mapping), &avatar)
        .await
    else {
        godot_error!("failed to extract emote GLTF for: {urn} (hash: {file_hash})");
        return;
    };
    if avatar.is_instance_valid() {
        avatar
            .clone()
            .bind_mut()
            .load_emote_from_gltf(urn, &gltf, &file_hash, false, false);
    }
}

async fn async_load_scene_emote(avatar: Gd<DclAvatar>, urn: &str) {
    let resolved = avatar.bind().find_scene_emote_by_urn(urn).or_else(|| {
        // Remote players and AvatarShape triggers never filled this avatar's registry: resolve
        // against the loaded scenes.
        let found = globals::scene_runner()
            .bind()
            .resolve_scene_emote_urn(urn.into());
        if found.is_empty() {
            return None;
        }
        let text = |key: &str| found.get(key).map(|v| v.to_string()).unwrap_or_default();
        Some(ResolvedSceneEmote {
            glb_hash: text("glb_hash"),
            base_url: text("base_url"),
            audio_hash: text("audio_hash"),
            looping: found.get("looping").is_some_and(|v| v.booleanize()),
        })
    });
    let resolved = match resolved {
        Some(resolved) => resolved,
        None => {
            // Last resort: the dash-split parse, right for dashless CID hashes.
            let Some(parsed) = SceneEmoteUrn::parse(urn) else {
                godot_error!("Failed to parse scene emote URN: {urn}");
                return;
            };
            let (base_url, audio_hash) = avatar
                .bind()
                .scene_emote_info(&parsed.scene_id, &parsed.glb_hash);
            ResolvedSceneEmote {
                glb_hash: parsed.glb_hash,
                base_url,
                audio_hash,
                looping: parsed.looping,
            }
        }
    };

    let mut files = VarDictionary::new();
    files.set("emote.glb", resolved.glb_hash.as_str());
    if !resolved.audio_hash.is_empty() {
        files.set("emote.mp3", resolved.audio_hash.as_str());
    }
    let base_url = if resolved.base_url.ends_with("contents/") {
        resolved.base_url.clone()
    } else {
        format!("{}contents/", resolved.base_url)
    };
    let mapping = DclContentMappingAndUrl::from_values(base_url.as_str().into(), files);
    let cli = globals::cli();
    let runtime_only = {
        let cli = cli.bind();
        cli.only_no_optimized_scene_emotes || cli.only_no_optimized
    };
    let loader = avatar.bind().emote.loader.clone();
    let path = loader
        .load_from_mapping(&resolved.glb_hash, "emote.glb", &mapping, runtime_only)
        .await;
    if path.is_empty() {
        godot_error!("Failed to load scene emote: {urn}");
        return;
    }
    let Some(gltf) = loader
        .get_emote_gltf(&resolved.glb_hash, "", None, &avatar)
        .await
    else {
        godot_error!("Failed to extract emote GLTF for scene emote: {urn}");
        return;
    };
    if avatar.is_instance_valid() {
        avatar.clone().bind_mut().load_emote_from_gltf(
            urn,
            &gltf,
            &resolved.glb_hash,
            true,
            resolved.looping,
        );
    }
}

/// Starts an emote's downloads without waiting; the audio promises are returned for the caller
/// to wait on (the clip is awaited later through the loader).
pub fn fetch_emote(emote_urn: &str, body_shape_id: &str) -> Vec<Gd<Promise>> {
    let mut provider = globals::content_provider();
    let Some(emote) = provider.bind_mut().get_wearable(emote_urn.into()) else {
        return Vec::new();
    };
    let file_name = emote
        .bind()
        .get_representation_main_file(body_shape_id.to_string());
    if file_name.is_empty() {
        return Vec::new();
    }
    let mapping = emote.bind().get_content_mapping();
    provider
        .bind_mut()
        .load_emote_gltf(file_name, mapping.clone());
    let audio = mapping
        .bind()
        .get_files()
        .as_slice()
        .iter()
        .find(|f| {
            let name = f.to_string();
            name.ends_with(".mp3") || name.ends_with(".ogg")
        })
        .cloned();
    match audio {
        Some(file) => vec![provider.bind_mut().fetch_audio(file, mapping)],
        None => Vec::new(),
    }
}

/// Poses the avatar mid-emote for a still: the tree is disabled and the clip sought to
/// `normalized_t`. False when the emote cannot be loaded or has no clip.
pub async fn async_freeze_on_emote(avatar: Gd<DclAvatar>, urn: &str, normalized_t: f64) -> bool {
    if !avatar.bind().emote.loaded.contains_key(urn) {
        async_load_emote(avatar.clone(), urn).await;
        if !alive(&avatar) {
            return false;
        }
        next_frame().await;
        if !avatar.bind().emote.loaded.contains_key(urn) {
            godot_error!("Emote {urn} could not be loaded");
            return false;
        }
    }
    let mut avatar = avatar;
    let mut this = avatar.bind_mut();
    let Some(item) = this.emote.loaded.get(urn) else {
        return false;
    };
    if item.default_anim_name.is_empty() {
        godot_error!("Emote {urn} has no animation");
        return false;
    }
    let anim_path = format!("emotes/{}", item.default_anim_name);
    let prop = item.armature_prop.clone();
    let Some(mut player) = this.nodes.as_ref().map(|n| n.animation_player.clone()) else {
        return false;
    };
    let Some(anim) = player.get_animation(anim_path.as_str()) else {
        godot_error!("Animation not found in player: {anim_path}");
        return false;
    };
    this.emit_emote_finished(true);
    this.abort_masked_for_freeze();
    if let Some(nodes) = this.nodes.as_mut() {
        nodes
            .animation_tree
            .set_process_mode(godot::classes::node::ProcessMode::DISABLED);
    }
    this.hide_all_props();
    this.reset_rest_pose_for_freeze();
    player.stop();
    player
        .play_ex()
        .name(anim_path.as_str())
        .custom_blend(-1.0)
        .custom_speed(0.0)
        .done();
    player
        .seek_ex(normalized_t.clamp(0.0, 1.0) * anim.get_length() as f64)
        .update(true)
        .done();
    if let Some(mut prop) = prop {
        if prop.is_instance_valid() {
            prop.show();
        }
    }
    true
}

/// Waits on a set of promises (a helper for callers outside this module).
pub async fn wait_all(promises: Vec<Gd<Promise>>) {
    await_all(promises).await;
}

/// Plays what an AvatarShape's `expression_trigger_id` names: a urn, or a default emote id.
pub async fn async_play_expression_trigger(avatar: Gd<DclAvatar>, id: String) {
    if id.is_empty() {
        return;
    }
    if id.starts_with("urn:") {
        async_play_emote_or_stash(avatar, id, -1, -1).await;
    } else {
        async_play_emote(avatar, id, -1, -1).await;
    }
}

/// A network or scene emote: parked until the avatar is built, since it would resolve against
/// the default body shape and be wiped by the rebuild.
pub async fn async_play_emote_or_stash(
    avatar: Gd<DclAvatar>,
    urn: String,
    mask: i64,
    scene_id: i32,
) {
    {
        let mut handle = avatar.clone();
        let mut this = handle.bind_mut();
        if !this.avatar_ready {
            this.pending_emote = Some(super::PendingEmote {
                urn,
                mask,
                scene_id,
            });
            return;
        }
    }
    async_play_emote(avatar, urn, mask, scene_id).await;
}
