//! Building the avatar from a profile: fetching the wearables, assembling the meshes onto the
//! skeleton, tinting skin and hair, and the facial textures.

use godot::classes::{MeshInstance3D, ShaderMaterial, Skeleton3D, StandardMaterial3D, Texture2D};
use godot::prelude::*;

use super::assembly;
use super::emote_load::{
    async_play_expression_trigger, fetch_emote, load_equipped_emotes, loads_emotes_lazily,
};
use crate::avatars::avatar_type::DclAvatarWireFormat;
use crate::avatars::dcl_user_profile::DclUserProfile;
use crate::avatars::eye_masks::base_eye_mask;
use crate::avatars::mesh_assembler::apply_toon_material;
use crate::avatars::wearables::{self, category, get_curated_wearable_list};
use crate::engine::local_task::await_all;
use crate::fetcher::globals;
use crate::godot_classes::dcl_avatar::DclAvatar;

/// Prefix of non-bodyshape mesh names: tells wearable categories apart (top_head vs head).
pub const WEARABLE_NAME_PREFIX: &str = "__";

const MASK_MATERIAL: &str = "res://assets/avatar/mask_material.tres";

/// What a scene's `AvatarShape` adds to the avatar it spawns.
#[derive(Default, Clone)]
pub struct AvatarShapeConfig {
    pub id: String,
    pub expression_trigger_id: String,
    /// None when the scene sent no timestamp: the trigger then fires when the id changes.
    pub expression_trigger_timestamp: Option<i64>,
}

/// Builds a scene NPC from its AvatarShape, starting next frame.
pub fn start_avatar_shape_update(
    avatar: Gd<DclAvatar>,
    wire: Gd<DclAvatarWireFormat>,
    name: String,
    config: AvatarShapeConfig,
) {
    crate::engine::local_task::spawn_local(update_avatar(avatar, wire, name, Some(config)));
}

fn alive(avatar: &Gd<DclAvatar>) -> bool {
    avatar.is_instance_valid() && avatar.is_inside_tree()
}

/// Applies the profile and builds the avatar from it.
pub async fn update_from_profile(avatar: Gd<DclAvatar>, profile: Gd<DclUserProfile>) {
    let (wire, mut name, claimed, address, web3) = {
        let profile = profile.bind();
        (
            profile.get_avatar(),
            profile.get_name().to_string(),
            profile.has_claimed_name(),
            profile.get_ethereum_address().to_string(),
            profile.has_connected_web3(),
        )
    };
    if !claimed {
        name.push('#');
        name.push_str(&address[address.len().saturating_sub(4)..]);
    }
    {
        let mut avatar = avatar.clone();
        let mut this = avatar.bind_mut();
        this.nametag.profile_ready = true;
        this.nametag.profile_status.clear();
        // The reveal waits for the body and the ground too: the give-up clock restarts here.
        this.spawn_reveal.born_ms = godot::classes::Time::singleton().get_ticks_msec();
        if let Some(nodes) = this.nodes.as_mut() {
            nodes.nickname_ui.bind_mut().set_name_claimed(claimed);
        }
        let changed = this.avatar_id.to_string() != address;
        this.avatar_id = address.as_str().into();
        this.has_connected_web3 = web3;
        godot_print!("Async update avatar from profile {address}");
        this.set_click_area_meta("avatar_id", address.as_str().to_variant());
        // The area may have triggered before the profile arrived (#2166).
        if changed {
            this.try_show();
        }
    }
    update_avatar(avatar, wire, name, None).await;
}

fn split_nickname(name: &str) -> (String, String) {
    let mut parts = name.split('#').filter(|s| !s.is_empty());
    let nickname = parts.next().unwrap_or("").to_string();
    match parts.next() {
        Some(tag) => (nickname, tag.to_string()),
        None => (name.to_string(), String::new()),
    }
}

/// Builds the avatar from wire data. A redundant update (same data, already loaded) only
/// refreshes the name: it saves re-duplicating every mesh and material.
pub async fn update_avatar(
    avatar: Gd<DclAvatar>,
    new_avatar: Gd<DclAvatarWireFormat>,
    new_name: String,
    shape: Option<AvatarShapeConfig>,
) {
    let mut handle = avatar.clone();
    let wearable_ids;
    {
        let mut this = handle.bind_mut();
        this.is_avatar_shape = shape.is_some();
        if let Some(shape) = shape.as_ref() {
            // An eth-address AvatarShape.id routes the impostor capture through the catalyst
            // body texture instead of an off-screen render.
            if shape.id.starts_with("0x") && this.avatar_id.to_string() != shape.id {
                this.avatar_id = shape.id.as_str().into();
                this.set_click_area_meta("avatar_id", shape.id.as_str().to_variant());
                this.try_show();
            }
        }
        let is_shape = this.is_avatar_shape;
        this.set_click_area_meta("is_avatar_shape", is_shape.to_variant());

        if let Some(shape) = shape.as_ref() {
            if !shape.expression_trigger_id.is_empty() {
                // Lamport timestamp when present; otherwise trigger when the id changes.
                let timestamp = shape.expression_trigger_timestamp.unwrap_or(-1);
                let trigger = if timestamp >= 0 {
                    timestamp > this.last_expression_trigger_timestamp
                } else {
                    shape.expression_trigger_id != this.last_expression_trigger_id
                };
                if trigger {
                    this.last_expression_trigger_timestamp = timestamp;
                    this.last_expression_trigger_id = shape.expression_trigger_id.clone();
                    if this.avatar_ready {
                        let id = shape.expression_trigger_id.clone();
                        drop(this);
                        crate::engine::local_task::spawn_local(async_play_expression_trigger(
                            avatar.clone(),
                            id,
                        ));
                        this = handle.bind_mut();
                    } else {
                        this.pending_expression_trigger = Some(shape.expression_trigger_id.clone());
                    }
                }
            }
        }

        let same = this.finish_loading && this.avatar_data.bind().equal(new_avatar.clone());
        if same {
            if this.avatar_name.to_string() != new_name {
                this.avatar_name = new_name.as_str().into();
                this.apply_nickname_text(&new_name);
                this.apply_nickname_visibility();
            }
            return;
        }

        this.avatar_data = new_avatar;
        this.avatar_name = new_name.as_str().into();
        this.apply_nickname_text(&new_name);
        if let Some(nodes) = this.nodes.as_mut() {
            nodes.nickname_ui.bind_mut().set_mic_enabled(false);
        }
        this.apply_nickname_visibility();

        let data = this.avatar_data.bind();
        let mut ids: Vec<String> = data
            .get_wearables()
            .as_slice()
            .iter()
            .map(|s| s.to_string())
            .collect();
        ids.extend(
            data.get_emotes()
                .as_slice()
                .iter()
                .map(|s| s.to_string())
                .filter(|urn| urn.starts_with("urn")),
        );
        ids.push(data.get_body_shape().to_string());
        wearable_ids = ids;
        drop(data);
        this.finish_loading = false;
    }

    let base_url = globals::realm().bind().get_profile_content_url();
    let mut ids = VarArray::new();
    for id in &wearable_ids {
        ids.push(&id.to_variant());
    }
    let promises = globals::content_provider()
        .bind_mut()
        .fetch_wearables(ids, base_url);
    await_all(promises.iter_shared().collect()).await;
    fetch_wearables_dependencies(avatar).await;
}

async fn fetch_wearables_dependencies(avatar: Gd<DclAvatar>) {
    if !alive(&avatar) {
        return;
    }
    let (body_shape, wearable_ids, emote_urns, loader) = {
        let this = avatar.bind();
        let data = this.avatar_data.bind();
        let body_shape = data.get_body_shape().to_string();
        let mut ids = vec![body_shape.clone()];
        ids.extend(
            data.get_wearables()
                .as_slice()
                .iter()
                .map(|s| s.to_string()),
        );
        let emotes: Vec<String> = data
            .get_emotes()
            .as_slice()
            .iter()
            .map(|s| s.to_string())
            .filter(|urn| urn.starts_with("urn"))
            .collect();
        (body_shape, ids, emotes, this.wearable_loader.clone())
    };
    let mut promises = Vec::new();
    if !loads_emotes_lazily(&avatar) {
        for urn in &emote_urns {
            promises.extend(fetch_emote(urn, &body_shape));
        }
    }
    loader.load_wearables(&wearable_ids, &body_shape).await;
    if !alive(&avatar) {
        return;
    }
    for result in await_all(promises).await {
        if let Ok(error) = result.try_to::<Gd<crate::godot_classes::promise::PromiseError>>() {
            godot_error!("Error loading emote: {}", error.bind().get_error());
        }
    }
    load_wearables(avatar).await;
}

/// Swaps the skeleton's meshes for the new body shape's.
async fn set_body_shape(avatar: &Gd<DclAvatar>, hash: &str, build: u64) {
    if !alive(avatar) {
        return;
    }
    let loader = avatar.bind().wearable_loader.clone();
    let Some(mut body) = loader.get_wearable_node(hash, avatar).await else {
        godot_error!("Avatar: failed to load body shape {hash}");
        return;
    };
    if !assembly::is_current(avatar, build) {
        body.queue_free();
        return;
    }
    let Some(mut new_skeleton) = body
        .find_child("Skeleton3D")
        .and_then(|n| n.try_cast::<Skeleton3D>().ok())
    else {
        body.queue_free();
        return;
    };
    let mut handle = avatar.clone();
    let mut this = handle.bind_mut();
    let Some(mut skeleton) = this.nodes.as_ref().map(|n| n.skeleton.clone()) else {
        return;
    };
    for mut child in skeleton.get_children().iter_shared() {
        if child.is_class("MeshInstance3D") {
            skeleton.remove_child(&child);
            child.queue_free();
        }
    }
    // Recycle the extra bones of the previous assembly so the next merge starts clean.
    if let Some(assembler) = this.mesh_assembler.as_mut() {
        assembler.recycle_extra_bones();
    }
    // The loader returns a fresh instance we discard, so children are reparented directly.
    for mut child in new_skeleton.get_children().iter_shared() {
        new_skeleton.remove_child(&child);
        child.set_owner(Gd::null_arg());
        let name = format!("bodyshape_{}", child.get_name().to_string().to_lowercase());
        child.set_name(name.as_str());
        skeleton.add_child(&child);
    }
    body.queue_free();
    this.reresolve_active_anchors();
}

/// Assembles everything onto the skeleton and marks the avatar ready.
async fn load_wearables(avatar: Gd<DclAvatar>) {
    if !alive(&avatar) {
        return;
    }
    let build = assembly::begin_build(&avatar);
    let (body_shape, wearable_ids, force_render, show_only) = {
        let mut this = avatar.clone();
        let mut this = this.bind_mut();
        let data = this.avatar_data.clone();
        let data = data.bind();
        let show_only = data.get_show_only_wearables();
        // Hide the skeleton at once so the default body never flashes.
        if show_only {
            if let Some(nodes) = this.nodes.as_mut() {
                nodes.skeleton.set_visible(false);
            }
        }
        (
            data.get_body_shape().to_string(),
            data.get_wearables()
                .as_slice()
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>(),
            data.get_force_render()
                .iter_shared()
                .map(|s| s.to_string())
                .collect::<Vec<_>>(),
            show_only,
        )
    };

    let mut curated =
        get_curated_wearable_list(&body_shape, &wearable_ids, &force_render, show_only);
    if curated.by_category.is_empty() {
        godot_error!("couldn't get curated wearables");
        return;
    }
    let Some(body_item) = curated.by_category.get(category::BODY_SHAPE).cloned() else {
        godot_error!("body shape not found");
        return;
    };

    let loader = avatar.bind().wearable_loader.clone();
    // Fallbacks the first request did not include.
    if !curated.need_to_fetch.is_empty() {
        let base_url = globals::realm().bind().get_profile_content_url();
        let mut ids = VarArray::new();
        for id in &curated.need_to_fetch {
            ids.push(&id.to_variant());
        }
        let promises = globals::content_provider()
            .bind_mut()
            .fetch_wearables(ids, base_url);
        await_all(promises.iter_shared().collect()).await;
        if !alive(&avatar) {
            return;
        }
        if !assembly::is_current(&avatar, build) {
            return;
        }
        let body_id = body_item.bind().get_id().to_string();
        loader
            .load_wearables(&curated.need_to_fetch, &body_id)
            .await;
        for id in &curated.need_to_fetch {
            let item = globals::content_provider()
                .bind_mut()
                .get_wearable(id.as_str().into());
            if let Some(item) = item {
                let slot = item.bind().get_category().to_string();
                curated.by_category.insert(&slot, item);
            }
        }
    }

    // Hidden until fully assembled: the build spans several frames (frame budget).
    if let Some(mut skeleton) = avatar.bind().body_shape_skeleton() {
        skeleton.set_visible(false);
    }
    let body_hash = wearables::get_item_main_file_hash(&body_item, &body_shape);
    set_body_shape(&avatar, &body_hash, build).await;
    curated.by_category.remove(category::BODY_SHAPE);

    let mut has_skin = false;
    let mut own = std::collections::HashSet::new();
    for (slot, item) in curated.by_category.iter() {
        if !alive(&avatar) {
            if avatar.is_instance_valid() {
                if let Some(mut skeleton) = avatar.bind().body_shape_skeleton() {
                    skeleton.set_visible(true);
                }
            }
            return;
        }
        if wearables::is_texture(slot) {
            continue;
        }
        let hash = wearables::get_item_main_file_hash(item, &body_shape);
        let Some(object) = loader.get_wearable_node(&hash, &avatar).await else {
            godot_error!("Avatar: failed to load wearable {slot} hash: {hash}");
            continue;
        };
        let suffix = format!("{WEARABLE_NAME_PREFIX}{slot}");
        if !assembly::attach_wearable(&avatar, object, &suffix, build).await {
            return;
        }
        if slot == category::SKIN {
            has_skin = true;
        }
        own.insert(slot.clone());
    }

    if !assembly::step(&avatar, build).await || !alive(&avatar) {
        return;
    }
    let hidden = curated.hidden_categories.clone();
    {
        let mut handle = avatar.clone();
        let mut this = handle.bind_mut();
        this.wearables_by_category = curated.by_category.clone();
        this.finish_meshes(&hidden, has_skin, &own, show_only);
    }
    assembly::end_build(&avatar);

    load_equipped_emotes(&avatar).await;
    if !assembly::is_current(&avatar, build) {
        return;
    }

    let (pending_trigger, pending_emote) = {
        let mut handle = avatar.clone();
        let mut this = handle.bind_mut();
        // The meshes were recreated: refresh the LOD state and the impostor.
        this.lod.mesh_visibility_captured = false;
        this.refresh_impostor_texture();
        this.avatar_ready = true;
        (
            this.pending_expression_trigger.take(),
            this.pending_emote.take(),
        )
    };
    // No bind is held here: listeners read the avatar back.
    avatar.clone().signals().avatar_loaded().emit();

    if let Some(id) = pending_trigger {
        async_play_expression_trigger(avatar.clone(), id).await;
    }
    if let Some(pending) = pending_emote {
        // A network emote that arrived while loading (Pulse replays the stored announcement).
        super::emote_load::async_play_emote(
            avatar.clone(),
            pending.urn,
            pending.mask,
            pending.scene_id,
        )
        .await;
    }
}

impl DclAvatar {
    /// Hides the covered base meshes, converts materials to toon, tints and applies the facial
    /// textures, and marks the skeleton visible.
    fn finish_meshes(
        &mut self,
        hidden_categories: &[String],
        has_skin: bool,
        own: &std::collections::HashSet<String>,
        show_only: bool,
    ) {
        let has = |c: &str| own.contains(c);
        let hides = |c: &str| hidden_categories.iter().any(|h| h == c);
        let all_off = show_only || has_skin;
        let head_own = has(category::HEAD);
        let mut rules: Vec<(String, bool)> = vec![
            (
                "ubody_basemesh".into(),
                all_off || has(category::UPPER_BODY) || hides("upper_body"),
            ),
            (
                "lbody_basemesh".into(),
                all_off || has(category::LOWER_BODY) || hides("lower_body"),
            ),
            (
                "feet_basemesh".into(),
                all_off || has(category::FEET) || hides("feet"),
            ),
            (
                "hands_basemesh".into(),
                all_off || has(category::HANDS) || hides("hands"),
            ),
            ("head_basemesh".into(), all_off || head_own || hides("head")),
            (
                "mask_eyes".into(),
                all_off || head_own || hides("eyes") || hides("head"),
            ),
            (
                "mask_eyebrows".into(),
                all_off || head_own || hides("eyebrows") || hides("head"),
            ),
            (
                "mask_mouth".into(),
                all_off || head_own || hides("mouth") || hides("head"),
            ),
        ];
        for slot in hidden_categories {
            rules.push((format!("{WEARABLE_NAME_PREFIX}{slot}"), true));
        }
        for mut mesh in self.mesh_children() {
            let name = mesh.get_name().to_string();
            if rules
                .iter()
                .any(|(suffix, on)| *on && name.ends_with(suffix.as_str()))
            {
                mesh.hide();
            }
        }

        if let Some(nodes) = self.nodes.as_ref() {
            apply_toon_material(&nodes.skeleton.clone().upcast());
        }
        for mesh in self.mesh_children() {
            apply_toon_material(&mesh.upcast());
        }
        self.apply_color_and_facial();

        // A wearable-only avatar is posed in T so the wearable does not animate.
        if show_only {
            if let Some(nodes) = self.nodes.as_mut() {
                for bone in 0..nodes.skeleton.get_bone_count() {
                    nodes.skeleton.reset_bone_pose(bone);
                }
            }
        }
        // #1554: during a spawn reveal the reveal owns the skeleton switch.
        let visible = self.spawn_reveal.is_idle();
        if let Some(nodes) = self.nodes.as_mut() {
            nodes.skeleton.set_visible(visible);
        }
        self.recompute_nametag_clearance();
        self.finish_loading = true;
    }

    /// Tints skin and hair surfaces per instance (the shared toon materials stay untinted) and
    /// applies the facial textures.
    pub fn apply_color_and_facial(&mut self) {
        let (skin, hair, eyes) = {
            let data = self.avatar_data.bind();
            (
                data.get_skin_color(),
                data.get_hair_color(),
                data.get_eyes_color(),
            )
        };
        for mut mesh in self.mesh_children() {
            if !mesh.is_visible() {
                continue;
            }
            let Some(mesh_resource) = mesh.get_mesh() else {
                continue;
            };
            for i in 0..mesh.get_surface_override_material_count() {
                let name = mesh_resource
                    .get(format!("surface_{i}/name").as_str())
                    .to_string()
                    .to_lowercase();
                let is_skin = name.contains("skin");
                let is_hair = name.contains("hair");
                let material = mesh
                    .get_surface_override_material(i)
                    .or_else(|| mesh_resource.surface_get_material(i));
                let Some(material) = material else {
                    continue;
                };
                if let Ok(shader) = material.clone().try_cast::<ShaderMaterial>() {
                    if is_skin || is_hair {
                        let mut copy: Gd<ShaderMaterial> = shader.duplicate_resource();
                        copy.set_shader_parameter(
                            "albedo_color",
                            &if is_skin { skin } else { hair }.to_variant(),
                        );
                        mesh.set_surface_override_material(i, &copy);
                    }
                } else if let Ok(standard) = material.try_cast::<StandardMaterial3D>() {
                    let mut copy: Gd<StandardMaterial3D> = standard.duplicate_resource();
                    copy.set_metallic(0.0);
                    copy.set_specular(0.0);
                    if is_skin {
                        copy.set_albedo(skin);
                    } else if is_hair {
                        copy.set_roughness(1.0);
                        copy.set_albedo(hair);
                    }
                    mesh.set_surface_override_material(i, &copy);
                }
            }
        }

        let body_shape = self.avatar_data.bind().get_body_shape().to_string();
        let pick = |slot: &str| self.wearables_by_category.get(slot).cloned();
        let eye_hashes =
            wearables::get_wearable_facial_hashes(pick(category::EYES).as_ref(), &body_shape);
        let brow_hashes =
            wearables::get_wearable_facial_hashes(pick(category::EYEBROWS).as_ref(), &body_shape);
        let mouth_hashes =
            wearables::get_wearable_facial_hashes(pick(category::MOUTH).as_ref(), &body_shape);
        for mut mesh in self.mesh_children() {
            if !mesh.is_visible() {
                continue;
            }
            let name = mesh.get_name().to_string();
            let (hashes, color, mask) = if name.ends_with("mask_eyes") {
                (&eye_hashes, eyes, Color::WHITE)
            } else if name.ends_with("mask_eyebrows") {
                (&brow_hashes, hair, Color::BLACK)
            } else if name.ends_with("mask_mouth") {
                (&mouth_hashes, skin, Color::BLACK)
            } else {
                continue;
            };
            if hashes.is_empty() {
                mesh.hide();
            } else {
                apply_texture_and_mask(&mut mesh, hashes, color, mask);
            }
        }
    }

    pub fn update_colors(&mut self, eyes: Color, skin: Color, hair: Color) {
        {
            let mut data = self.avatar_data.bind_mut();
            data.set_eyes_color(eyes);
            data.set_skin_color(skin);
            data.set_hair_color(hair);
        }
        if self.finish_loading {
            self.apply_color_and_facial();
            self.refresh_impostor_texture();
        }
    }

    pub(crate) fn apply_nickname_text(&mut self, name: &str) {
        let (nickname, tag) = split_nickname(name);
        let color = DclAvatar::get_nickname_color(name.into());
        if let Some(nodes) = self.nodes.as_mut() {
            let mut ui = nodes.nickname_ui.bind_mut();
            ui.set_nickname(nickname.as_str().into());
            ui.set_tag(tag.as_str().into());
            ui.set_nickname_color(color);
        }
    }

    pub(crate) fn set_click_area_meta(&mut self, key: &str, value: Variant) {
        // The local player's click area is freed to avoid self-targeting.
        if let Some(nodes) = self
            .nodes
            .as_mut()
            .filter(|n| n.click_area.is_instance_valid())
        {
            nodes.click_area.set_meta(key, &value);
        }
    }
}

fn apply_texture_and_mask(
    mesh: &mut Gd<MeshInstance3D>,
    hashes: &[String],
    color: Color,
    mask: Color,
) {
    let template = load::<ShaderMaterial>(MASK_MATERIAL);
    let mut material: Gd<ShaderMaterial> = template.duplicate_resource();
    let mut provider = globals::content_provider();
    let mut texture = |hash: &String| {
        provider
            .bind_mut()
            .get_texture_from_hash(hash.as_str().into())
    };
    material.set_shader_parameter("base_texture", &texture(&hashes[0]).to_variant());
    material.set_shader_parameter("material_color", &color.to_variant());
    material.set_shader_parameter("mask_color", &mask.to_variant());
    let mask_texture = match hashes.get(1) {
        Some(hash) => texture(hash),
        None => base_eye_mask(&hashes[0]).map(|(path, replace)| {
            material.set_shader_parameter("mask_replace", &(replace as i32 as f32).to_variant());
            load::<Texture2D>(path)
        }),
    };
    material.set_shader_parameter("mask_texture", &mask_texture.to_variant());
    mesh.set_surface_override_material(0, &material);
}

#[cfg(test)]
mod tests {
    use super::split_nickname;

    #[test]
    fn nickname_splits_at_the_hash() {
        assert_eq!(split_nickname("Ana#1a2b"), ("Ana".into(), "1a2b".into()));
        assert_eq!(split_nickname("Ana"), ("Ana".into(), String::new()));
        assert_eq!(split_nickname("#abcd"), ("#abcd".into(), String::new()));
    }
}
