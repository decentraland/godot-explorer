//! The avatar's nameplate: when it shows, what it says, and where it floats.

use godot::classes::control::{LayoutPreset, MouseFilter};
use godot::classes::{CollisionObject3D, MeshInstance3D};
use godot::prelude::*;

use crate::avatars::impostor::LodState;
use crate::avatars::nameplate::{
    self, with_plates, ANCHOR_HEIGHT_FACTOR, CL_PHYSICS, DEBUG_DISABLE_OCCLUSION, FADE_END,
    FADE_SPEED, FAR_SKIP_DISTANCE, NAMETAG_MARGIN, SCALE,
};
use crate::avatars::nameplate_occlusion;
use crate::avatars::nameplate_tiers::{tier_alpha, Tier, TIER_FULL_END, TIER_NAME_SCALE};
use crate::avatars::proximity_fade::AvatarProximityFade;
use crate::godot_classes::dcl_avatar::DclAvatar;
use crate::godot_classes::dcl_global::DclGlobal;

/// Nametag height when no meshes are loaded yet, meters above the avatar origin.
const DEFAULT_NAMETAG_HEIGHT: f32 = 1.9;

pub struct NametagState {
    /// The hide flags and the FAR gate: whether the plate may show at all.
    pub gate_visible: bool,
    pub occluded: bool,
    /// False until a real profile was applied: until then the plate shows the scene default.
    pub profile_ready: bool,
    pub request_failures: i32,
    pub request_banned: bool,
    /// Max of the posed bones' height along the skeleton's Y row, cached per skeleton update.
    pub posed_top: f32,
    /// Meters between the topmost joint and the top of the meshes (skull, hats).
    pub clearance: f32,
}

impl Default for NametagState {
    fn default() -> Self {
        Self {
            gate_visible: true,
            occluded: false,
            profile_ready: false,
            request_failures: 0,
            request_banned: false,
            posed_top: 0.0,
            clearance: 0.3,
        }
    }
}

impl DclAvatar {
    /// World-space Y the nameplate floats at: the posed skeleton top plus a clearance covering
    /// the mesh volume above the topmost joint. Both parts are cached.
    pub fn get_bounds_top_y(&self) -> f32 {
        match self.nodes.as_ref() {
            Some(nodes) if nodes.skeleton.get_bone_count() > 0 => {
                nodes.skeleton.get_global_transform().origin.y
                    + self.nametag.posed_top
                    + self.nametag.clearance
            }
            _ => self.base().get_global_position().y + DEFAULT_NAMETAG_HEIGHT,
        }
    }

    /// Recomputed on `skeleton_updated`. The basis Y row is constant while the avatar only yaws
    /// and moves, so the per-frame cost is adding the skeleton's world origin.
    pub(crate) fn recompute_nametag_posed_top(&mut self) {
        if !self.nametag.gate_visible {
            return;
        }
        let Some(nodes) = self.nodes.as_ref() else {
            return;
        };
        let basis = nodes.skeleton.get_global_transform().basis;
        let y_row = Vector3::new(basis.rows[0].y, basis.rows[1].y, basis.rows[2].y);
        let mut top = f32::NEG_INFINITY;
        for bone in 0..nodes.skeleton.get_bone_count() {
            top = top.max(y_row.dot(nodes.skeleton.get_bone_global_pose(bone).origin));
        }
        if top.is_finite() {
            self.nametag.posed_top = top;
        }
    }

    /// Clearance above the topmost joint: bind mesh AABB top minus bind bone top. Recomputed on
    /// wearable load.
    pub(crate) fn recompute_nametag_clearance(&mut self) {
        let Some(nodes) = self.nodes.as_ref() else {
            return;
        };
        let skeleton = nodes.skeleton.clone();
        let xform = skeleton.get_global_transform();
        let mut rest_top = f32::NEG_INFINITY;
        for bone in 0..skeleton.get_bone_count() {
            rest_top = rest_top.max((xform * skeleton.get_bone_global_rest(bone).origin).y);
        }
        let mut mesh_top = f32::NEG_INFINITY;
        let mut head_radius = f32::NEG_INFINITY;
        for child in skeleton.get_children().iter_shared() {
            let Ok(mesh) = child.try_cast::<MeshInstance3D>() else {
                continue;
            };
            if !mesh.is_visible() {
                continue;
            }
            let aabb = mesh.get_transform() * mesh.get_aabb();
            mesh_top = mesh_top.max(aabb.end().y);
            // Occlusion radius from head-zone meshes only: T-pose arms inflate full-body AABBs.
            if aabb.center().y > 1.2 {
                head_radius = head_radius.max(aabb.size.x.max(aabb.size.z) * 0.5);
            }
        }
        self.occlusion_radius = if head_radius.is_finite() {
            head_radius.clamp(0.4, 1.5)
        } else {
            0.4
        };
        if !mesh_top.is_finite() || !rest_top.is_finite() {
            self.nametag.clearance = 0.3;
            return;
        }
        let mesh_top_world = self.base().to_global(Vector3::new(0.0, mesh_top, 0.0)).y;
        self.nametag.clearance = (mesh_top_world - rest_top).max(0.15);
    }

    pub fn set_force_hide_name(&mut self, value: bool) {
        if self.force_hide_name == value {
            return;
        }
        self.force_hide_name = value;
        if self.base().is_inside_tree() {
            self.apply_nickname_visibility();
        }
    }

    /// Opens or closes the gate the per-frame placement reads.
    pub(crate) fn apply_nickname_visibility(&mut self) {
        if self.nodes.is_none() {
            return;
        }
        // No profile yet: the plate still shows its scene default ("nickname#xxxx"). Hide it in
        // production; dev builds show the request state.
        let profile_pending = !self.nametag.profile_ready && !self.is_avatar_shape;
        if profile_pending && !DclGlobal::is_production() {
            self.apply_pending_profile_nameplate();
        }
        // A scene NPC keeps the proto default "NPC" name only when the scene set none; FAR
        // plates are unreadable and each one is a draw call.
        let name = self.avatar_name.to_string();
        let no_name = self.is_avatar_shape && (name.is_empty() || name == "NPC");
        let far = self.lod.state == LodState::Far;
        let own_nametag_off = self.is_local_player && !crate::engine::show_own_nametag();
        let should_hide = no_name
            || self.hide_name
            || self.force_hide_name
            || far
            || self.nametag_hidden
            || self.hidden
            || own_nametag_off
            || (profile_pending && DclGlobal::is_production());
        self.nametag.gate_visible = !should_hide;
        if should_hide {
            // Zero the alpha too: the placement derives visibility from its eased alpha, so a
            // plain hide would be undone and the stale plate would fade out for a few frames.
            if let Some(nodes) = self.nodes.as_mut() {
                nodes
                    .nickname_ui
                    .set_modulate(Color::from_rgba(1.0, 1.0, 1.0, 0.0));
                nodes.nickname_ui.hide();
            }
        }
    }

    /// Dev and staging: the request state instead of the meaningless "nickname#xxxx".
    fn apply_pending_profile_nameplate(&mut self) {
        let failed = self.nametag.request_banned;
        let id = self.avatar_id.to_string();
        let tag: GString = if id.is_empty() {
            "…".into()
        } else {
            id[id.len().saturating_sub(4)..].into()
        };
        if let Some(nodes) = self.nodes.as_mut() {
            let mut ui = nodes.nickname_ui.bind_mut();
            ui.set_name_claimed(false);
            ui.set_nickname(if failed { "Failed" } else { "Loading" }.into());
            ui.set_tag(tag);
            ui.set_nickname_color(if failed {
                Color::from_rgb(0.85, 0.4, 0.4)
            } else {
                Color::from_rgb(0.7, 0.7, 0.7)
            });
        }
    }

    /// Pushed from comms when a profile fetch fails or gets banned.
    pub fn set_profile_request_state(&mut self, failures: i32, banned: bool) {
        self.nametag.request_failures = failures;
        self.nametag.request_banned = banned;
        if !self.nametag.profile_ready {
            self.apply_nickname_visibility();
        }
    }

    pub fn set_client_version(&mut self, version: GString) {
        if let Some(nodes) = self.nodes.as_mut() {
            nodes.nickname_ui.bind_mut().set_client_version(version);
        }
    }

    pub fn set_room_debug(&mut self, info: GString) {
        if let Some(nodes) = self.nodes.as_mut() {
            nodes.nickname_ui.bind_mut().set_room_debug(info);
        }
    }

    pub(crate) fn on_chat_message(&mut self, address: &str, message: &str) {
        if self.avatar_id != address {
            return;
        }
        if let Some(nodes) = self.nodes.as_mut() {
            nodes.nickname_ui.bind_mut().show_message(message);
        }
    }

    /// Moves the plate out of its per-avatar SubViewport into the shared layer and drops the
    /// render target. The quad stays as an invisible head anchor: it still drives the SDK
    /// NAME_TAG attach point and the screen projection.
    pub(crate) fn nameplate_attach(&mut self) {
        let Some(nodes) = self.nodes.as_mut() else {
            return;
        };
        let mut ui = nodes.nickname_ui.clone();
        if let Some(mut viewport) = nodes.nickname_viewport.take() {
            if viewport.is_instance_valid() {
                viewport.remove_child(&ui);
                viewport.queue_free();
            }
        }
        nodes.nickname_quad.set_texture(Gd::null_arg());
        nodes.nickname_quad.set_visible(false);
        ui.set_anchors_preset(LayoutPreset::TOP_LEFT);
        ui.set_mouse_filter(MouseFilter::IGNORE);
        // Start transparent: placement eases the alpha up, so a plate can only fade in once the
        // gate opens with real data, never fade out from the scene default.
        ui.set_modulate(Color::from_rgba(1.0, 1.0, 1.0, 0.0));
        ui.hide();
        nameplate::layer_root().add_child(&ui);
        nameplate_occlusion::add(self.to_gd());
    }

    /// Frees the reparented plate: it lives in the shared layer, not under the avatar.
    pub(crate) fn nameplate_detach(&mut self) {
        nameplate_occlusion::remove(self.base().instance_id());
        if let Some(nodes) = self.nodes.as_ref() {
            if nodes.nickname_ui.is_instance_valid() {
                with_plates(|s| s.untrack(nodes.nickname_ui.instance_id().to_i64()));
                nodes.nickname_ui.clone().queue_free();
            }
        }
    }

    /// Drops the plate from the solver and hides it while out of the tree (a transient reparent
    /// must not free it: `ready` only runs once).
    pub(crate) fn nameplate_park(&mut self) {
        if let Some(nodes) = self.nodes.as_mut() {
            if nodes.nickname_ui.is_instance_valid() {
                with_plates(|s| s.untrack(nodes.nickname_ui.instance_id().to_i64()));
                nodes
                    .nickname_ui
                    .set_modulate(Color::from_rgba(1.0, 1.0, 1.0, 0.0));
                nodes.nickname_ui.hide();
            }
        }
    }

    /// Head anchor horizontally, bounds top plus margin vertically.
    fn nameplate_anchor(&self) -> Vector3 {
        let mut anchor = self
            .nodes
            .as_ref()
            .map(|n| n.nickname_quad.get_global_transform().origin)
            .unwrap_or_default();
        anchor.y = self.get_bounds_top_y() + NAMETAG_MARGIN;
        anchor
    }

    /// Per frame: project the anchor, place, scale and sort the plate, and ease its alpha toward
    /// a target that is zero unless the tag is gated visible, in front of the camera within
    /// FADE_END, inside the viewport and not occluded, so it fades instead of popping.
    pub(crate) fn nameplate_update(&mut self) {
        let Some(nodes) = self.nodes.as_ref() else {
            return;
        };
        let mut ui = nodes.nickname_ui.clone();
        if !ui.is_instance_valid() {
            return;
        }
        let id = ui.instance_id().to_i64();
        let me = self.to_gd();
        let viewport = self.base().get_viewport();
        let camera = viewport.as_ref().and_then(|v| v.get_camera_3d());
        let mut target_alpha = 0.0_f32;
        let mut on_screen = false;
        if let Some(camera) = camera.as_ref() {
            let quad = nodes.nickname_quad.get_global_position();
            if ui.get_modulate().a == 0.0
                && camera.get_global_position().distance_to(quad) > FAR_SKIP_DISTANCE
            {
                with_plates(|s| s.untrack(id));
                nameplate_occlusion::set_on_screen(&me, false);
                ui.set_visible(false);
                return;
            }
        }

        match (camera, viewport) {
            (Some(camera), Some(viewport)) if self.nametag.gate_visible => {
                let anchor = self.nameplate_anchor();
                let dist = camera.get_global_position().distance_to(anchor);
                if dist <= FADE_END && !camera.is_position_behind(anchor) {
                    let me_id = me.instance_id();
                    let focused = DclGlobal::singleton().bind().selected_avatar == Some(me_id);
                    let tier = with_plates(|p| p.tiers.content_tier(id, dist, focused));
                    let size = ui.get_combined_minimum_size();
                    ui.set_size(size);
                    let scale = SCALE
                        * if tier == Tier::Name {
                            TIER_NAME_SCALE
                        } else {
                            1.0
                        };
                    ui.set_scale(Vector2::new(scale, scale));
                    let screen_size = size * scale;
                    let mut pos = camera.unproject_position(anchor)
                        - Vector2::new(screen_size.x * 0.5, screen_size.y * ANCHOR_HEIGHT_FACTOR);
                    let view_size = viewport.get_visible_rect().size;
                    on_screen = Rect2::new(Vector2::ZERO, view_size)
                        .intersects(Rect2::new(pos, screen_size));
                    nameplate_occlusion::set_on_screen(&me, on_screen);
                    let untested = nameplate_occlusion::is_untested(me_id);
                    if on_screen && !self.nametag.occluded && !untested {
                        // Chat bubbles gate on distance only: the crowd cap must not hide the
                        // bubble of someone standing next to you.
                        let show_chat = focused || dist <= TIER_FULL_END;
                        let changed = with_plates(|p| {
                            p.tiers.record(id, dist, focused);
                            p.tiers.set_applied(id, tier, show_chat)
                        });
                        if changed {
                            ui.bind_mut().apply_tier(tier, show_chat);
                        }
                        target_alpha = tier_alpha(tier, dist, focused);
                    } else {
                        with_plates(|p| p.untrack(id));
                    }
                    // An anchor above the top edge would clip the tag: keep it in the viewport.
                    pos.y = pos.y.max(0.0);
                    pos.x = pos.x.clamp(0.0, (view_size.x - screen_size.x).max(0.0));
                    ui.set_position(pos);
                    ui.set_z_index((-(dist * 100.0) as i32).clamp(-4000, 4000));
                    // Where the tag is drawn, as a world point at anchor depth: occlusion rays
                    // go there, not to the head.
                    let center = pos + screen_size * 0.5;
                    let point = camera.project_ray_origin(center)
                        + camera.project_ray_normal(center) * dist;
                    with_plates(|s| s.set_ray_target(id, point));
                } else {
                    with_plates(|s| s.untrack(id));
                }
            }
            _ => with_plates(|s| s.untrack(id)),
        }
        if !on_screen {
            nameplate_occlusion::set_on_screen(&me, false);
        }
        // The plate follows the GPU-ready hold: it never floats over an avatar not drawn yet.
        let fade = self
            .base()
            .get_node_or_null("AvatarProximityFade")
            .and_then(|n| n.try_cast::<AvatarProximityFade>().ok());
        if let Some(fade) = fade {
            target_alpha *= 1.0 - fade.bind().get_hold();
        }

        let step = self.base().get_process_delta_time() as f32 * FADE_SPEED;
        let mut modulate = ui.get_modulate();
        modulate.a = move_toward(modulate.a, target_alpha, step);
        ui.set_modulate(modulate);
        ui.set_visible(modulate.a > 0.01);
    }

    /// Occlusion raycast for this plate; the occlusion driver throttles it. Must run in a physics
    /// step: the direct space state crashes when queried from an idle frame.
    pub(crate) fn nameplate_update_occlusion(&mut self) {
        if DEBUG_DISABLE_OCCLUSION.load(std::sync::atomic::Ordering::Relaxed) {
            self.nametag.occluded = false;
            return;
        }
        if !self.nametag.gate_visible {
            return;
        }
        let Some(camera) = self.base().get_viewport().and_then(|v| v.get_camera_3d()) else {
            return;
        };
        let anchor = self.nameplate_anchor();
        let from = camera.get_global_position();
        if from.distance_to(anchor) > FADE_END {
            return;
        }
        // Against the tag's drawn position, the head anchor before the first one.
        let ui_id = self
            .nodes
            .as_ref()
            .map(|n| n.nickname_ui.instance_id().to_i64());
        let target = ui_id
            .and_then(|id| with_plates(|s| s.ray_target(id)))
            .unwrap_or(anchor);
        self.nametag.occluded = self.is_occluded(from, target);
    }

    /// Something sits between camera and tag: a body raycast, then the analytic avatar check.
    fn is_occluded(&self, from: Vector3, to: Vector3) -> bool {
        let Some(world) = self.base().get_world_3d() else {
            return false;
        };
        let Some(mut space) = world.get_direct_space_state() else {
            return false;
        };
        // Never occlude with this avatar's own colliders. The local player's body is kept for
        // other avatars' rays: yours legitimately blocks tags behind it in third person.
        let mut exclude: Array<Rid> = Array::new();
        if let Some(nodes) = self.nodes.as_ref() {
            if nodes.click_area.is_instance_valid() {
                exclude.push(nodes.click_area.get_rid());
            }
            if let Some(trigger) = nodes.trigger_detector.as_ref() {
                if trigger.is_instance_valid() {
                    exclude.push(trigger.get_rid());
                }
            }
        }
        if self.is_local_player {
            if let Some(parent) = self.base().get_parent() {
                if let Ok(body) = parent.try_cast::<CollisionObject3D>() {
                    exclude.push(body.get_rid());
                }
            }
        }

        // A hit on a hidden avatar's collider (blocked / modifier-area hidden) does not count:
        // skip it and ray again so real geometry behind it is still tested.
        let mut query = nameplate::occlusion_query(from, to, exclude);
        for _ in 0..3 {
            let hit = space.intersect_ray(&query);
            if hit.is_empty() {
                break;
            }
            let collider = hit
                .get("collider")
                .and_then(|c| c.try_to::<Gd<Node3D>>().ok());
            // Scene colliders generated for invisible meshes (`*_colgen`) are invisible nodes
            // too: only a CL_PLAYER body may be skipped as a hidden avatar, or walls stop
            // occluding.
            let scene_collider = collider
                .as_ref()
                .and_then(|c| c.clone().try_cast::<CollisionObject3D>().ok())
                .is_some_and(|c| c.get_collision_layer() & CL_PHYSICS != 0);
            match collider {
                _ if scene_collider => return true,
                Some(collider) if !collider.is_visible_in_tree() => {
                    let mut next = query.get_exclude();
                    if let Some(rid) = hit.get("rid").and_then(|r| r.try_to::<Rid>().ok()) {
                        next.push(rid);
                    }
                    query.set_exclude(&next);
                }
                _ => return true,
            }
        }
        self.blocked_by_avatar(from, to)
    }

    /// Another avatar's body (an analytic vertical cylinder: the physics ClickArea capsule is
    /// twice the visual width) crosses the camera to anchor segment. Covers comms avatars, scene
    /// NPCs (whose ClickArea is LOD-disabled) and the local player.
    fn blocked_by_avatar(&self, from: Vector3, to: Vector3) -> bool {
        let me = self.base().instance_id();
        let my_world = self.base().get_world_3d().map(|w| w.instance_id());
        let delta = to - from;
        let delta_xz = Vector2::new(delta.x, delta.z);
        let len_sq = delta_xz.length_squared();
        nameplate_occlusion::with_occluders(|occluders| {
            occluders.iter().any(|other| {
                // Avatars of another world (a backpack preview) cannot block this one.
                if other.id == me || other.world != my_world {
                    return false;
                }
                let origin = other.origin;
                // Closest point (in XZ) of the segment to the other's vertical axis.
                let t = if len_sq > 0.0001 {
                    (Vector2::new(origin.x - from.x, origin.z - from.z).dot(delta_xz) / len_sq)
                        .clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let closest = from + delta * t;
                // The camera inside an avatar (spring arm collapsed on a wall) must not blank every
                // tag on screen: the #2321 symptom with a smaller trigger.
                if from.distance_squared_to(closest) < 0.25 {
                    return false;
                }
                if closest.y < origin.y - 0.1 || closest.y > other.top {
                    return false;
                }
                let (dx, dz) = (closest.x - origin.x, closest.z - origin.z);
                dx * dx + dz * dz < other.radius * other.radius
            })
        })
    }
}

fn move_toward(from: f32, to: f32, step: f32) -> f32 {
    if (to - from).abs() <= step {
        to
    } else {
        from + (to - from).signum() * step
    }
}
