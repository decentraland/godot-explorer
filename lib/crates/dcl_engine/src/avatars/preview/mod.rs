//! The avatar preview of the backpack, profile and lobby: an avatar in its own world, framed by
//! an orthographic camera, rotated and zoomed by touch or mouse, and captured to images.

pub mod fit;

use std::collections::HashMap;

use godot::builtin::Side;
use godot::classes::image::Interpolation;
use godot::classes::{
    Camera3D, Control, DirectionalLight3D, Environment, ISubViewportContainer, Image,
    ImmediateMesh, InputEvent, InputEventMouseButton, InputEventMouseMotion, InputEventScreenDrag,
    InputEventScreenTouch, MeshInstance3D, Skeleton3D, StandardMaterial3D, SubViewport,
    SubViewportContainer, WorldEnvironment,
};
use godot::global::MouseButton;
use godot::prelude::*;

use self::fit::{
    aabb_endpoint, fit_camera, fit_limits, fit_to_overall, focus_extra_margin, focus_for_category,
    mesh_category, pan_limits, stable_aabb, symmetrize_aabb_x, Aabbs, CameraTarget, FitView,
    OverallFit, MAX_CAMERA_SIZE, MIN_CAMERA_SIZE_PART,
};
use crate::engine::local_task::{next_frame, spawn_local};
use crate::fetcher::globals;
use crate::godot_classes::dcl_avatar::DclAvatar;
use crate::godot_classes::promise::Promise;
use crate::player::outline_system::OutlineSystem;

const CAMERA_PAN_SMOOTH: f32 = 2.0;
const CAMERA_ZOOM_SMOOTH: f32 = 2.0;
const AVATAR_ROTATION_SMOOTH: f32 = 15.0;

// Camera of the profile snapshots.
const PROFILE_BODY_CAMERA_POSITION: Vector3 = Vector3::new(0.0, 1.25, -3.5);
const PROFILE_HEAD_CAMERA_POSITION: Vector3 = Vector3::new(0.0, 1.70, -1.25);
const PROFILE_BODY_CAMERA_ROTATION: Vector3 = Vector3::new(-5.0, 180.0, 0.0);
const PROFILE_HEAD_CAMERA_ROTATION: Vector3 = Vector3::new(0.0, 180.0, 0.0);

#[derive(GodotClass)]
#[class(base=SubViewportContainer)]
pub struct AvatarPreview {
    #[export]
    #[var(pub)]
    hide_name: bool,
    #[export]
    #[var(pub)]
    show_platform: bool,
    /// Enables rotation and pan/zoom interactions.
    #[export]
    #[var(pub)]
    can_move: bool,
    /// With `can_move`, gates wheel and pinch zoom, the pinch vertical pan and the per-category
    /// auto focus. Rotation is always enabled.
    #[export]
    #[var(pub)]
    can_drag: bool,
    #[export]
    custom_environment: Option<Gd<Environment>>,
    #[export]
    #[var(pub)]
    with_light: bool,
    #[export]
    #[var(pub)]
    preview_margin_top: i32,
    #[export]
    #[var(pub)]
    preview_margin_bottom: i32,
    #[export]
    #[var(pub)]
    preview_margin_left: i32,
    #[export]
    #[var(pub)]
    preview_margin_right: i32,
    /// A Control whose bottom edge defines the top margin boundary.
    #[export]
    top_node_margin: Option<Gd<Control>>,
    /// A Control whose top edge defines the bottom margin boundary.
    #[export]
    bottom_node_margin: Option<Gd<Control>>,
    #[export]
    #[var(pub)]
    show_aabb_debug: bool,
    #[export]
    #[var(pub)]
    fit_avatar: bool,
    /// Grows this container upward so its top sits at the screen's top whatever the parent layout
    /// did: for hosts with UI above the preview, the avatar then shows behind it.
    #[export]
    #[var(pub, set = set_snap_top_to_viewport)]
    snap_top_to_viewport: bool,

    dragging: bool,
    start_angle: f32,
    start_dragging_position: Vector2,
    camera_focus: String,
    cached_aabbs: Aabbs,
    user_has_panned: bool,
    pending_camera_fit: bool,
    /// A deferred initial fit: the size was zero or the margins ate the viewport on first call.
    /// `on_resized` re-runs it while true.
    pending_fit_overall: bool,
    fitted_camera_size: f32,
    fitted_aabb_center_y: f32,
    last_fit_stable: Option<Aabb>,
    last_fit_extra_margin: i32,
    target_camera_center_y: f32,
    target_camera_size: f32,
    target_avatar_rotation_y: f32,
    lerp_paused: bool,
    touch_points: HashMap<i32, Vector2>,
    pinch_start_distance: f32,
    pinch_start_camera_size: f32,
    pinch_start_midpoint: Vector2,
    pinch_start_center_y: f32,
    snap_top_connected: bool,
    last_snapped_parent_y: Option<f32>,
    baseline_offset_top: f32,
    aabb_debug_nodes: Vec<Gd<MeshInstance3D>>,

    #[var(pub)]
    pub avatar: Option<Gd<DclAvatar>>,
    camera_center: Option<Gd<Node3D>>,
    #[var(pub)]
    camera_3d: Option<Gd<Camera3D>>,
    platform: Option<Gd<Node3D>>,
    #[var(pub)]
    pub subviewport: Option<Gd<SubViewport>>,
    #[var(pub)]
    world_environment: Option<Gd<WorldEnvironment>>,
    outline_system: Option<Gd<OutlineSystem>>,
    base: Base<SubViewportContainer>,
}

#[godot_api]
impl ISubViewportContainer for AvatarPreview {
    fn init(base: Base<SubViewportContainer>) -> Self {
        Self {
            hide_name: false,
            show_platform: false,
            can_move: true,
            can_drag: true,
            custom_environment: None,
            with_light: true,
            preview_margin_top: 0,
            preview_margin_bottom: 0,
            preview_margin_left: 0,
            preview_margin_right: 0,
            top_node_margin: None,
            bottom_node_margin: None,
            show_aabb_debug: false,
            fit_avatar: true,
            snap_top_to_viewport: false,
            dragging: false,
            start_angle: 0.0,
            start_dragging_position: Vector2::ZERO,
            camera_focus: "overall".into(),
            cached_aabbs: Aabbs::new(),
            user_has_panned: false,
            pending_camera_fit: false,
            pending_fit_overall: false,
            fitted_camera_size: MAX_CAMERA_SIZE,
            fitted_aabb_center_y: 0.0,
            last_fit_stable: None,
            last_fit_extra_margin: -1,
            target_camera_center_y: 0.0,
            target_camera_size: MAX_CAMERA_SIZE,
            target_avatar_rotation_y: 0.0,
            lerp_paused: false,
            touch_points: HashMap::new(),
            pinch_start_distance: 0.0,
            pinch_start_camera_size: 0.0,
            pinch_start_midpoint: Vector2::ZERO,
            pinch_start_center_y: 0.0,
            snap_top_connected: false,
            last_snapped_parent_y: None,
            baseline_offset_top: 0.0,
            aabb_debug_nodes: Vec::new(),
            avatar: None,
            camera_center: None,
            camera_3d: None,
            platform: None,
            subviewport: None,
            world_environment: None,
            outline_system: None,
            base,
        }
    }

    fn ready(&mut self) {
        let base = self.base().clone();
        self.subviewport = base
            .get_node_or_null("%SubViewport")
            .and_then(|n| n.try_cast().ok());
        self.avatar = base
            .get_node_or_null("%Avatar")
            .and_then(|n| n.try_cast().ok());
        self.camera_center = base
            .get_node_or_null("%CameraCenter")
            .and_then(|n| n.try_cast().ok());
        self.camera_3d = base
            .get_node_or_null("%Camera3D")
            .and_then(|n| n.try_cast().ok());
        self.platform = base
            .get_node_or_null("%Platform")
            .and_then(|n| n.try_cast().ok());
        self.outline_system = base
            .get_node_or_null("%OutlineSystem")
            .and_then(|n| n.try_cast().ok());
        self.world_environment = base
            .get_node_or_null("SubViewport/WorldEnvironment")
            .and_then(|n| n.try_cast().ok());
        let light: Option<Gd<DirectionalLight3D>> = base
            .get_node_or_null("SubViewport/DirectionalLight3D")
            .and_then(|n| n.try_cast().ok());

        if let (Some(environment), Some(world)) = (
            self.custom_environment.as_ref(),
            self.world_environment.as_mut(),
        ) {
            world.set_environment(environment);
        }
        if let Some(mut light) = light {
            light.set_visible(self.with_light);
        }
        let hide_name = self.hide_name;
        if let Some(avatar) = self.avatar.as_mut() {
            avatar.bind_mut().set_hide_name(hide_name);
        }
        if let Some(platform) = self.platform.as_mut() {
            platform.set_visible(self.show_platform);
        }
        if let (Some(outline), Some(camera)) =
            (self.outline_system.as_mut(), self.camera_3d.clone())
        {
            outline.bind_mut().setup(camera);
        }
        self.base_mut().set_process_input(true);

        self.target_camera_center_y = self
            .camera_center
            .as_ref()
            .map(|c| c.get_position().y)
            .unwrap_or_default();
        self.target_camera_size = self
            .camera_3d
            .as_ref()
            .map(|c| c.get_size())
            .unwrap_or(MAX_CAMERA_SIZE);
        self.target_avatar_rotation_y = self
            .avatar
            .as_ref()
            .map(|a| a.get_rotation().y)
            .unwrap_or_default();

        let on_loaded = self.base().callable("on_avatar_loaded");
        let on_resized = self.base().callable("on_resized");
        if let Some(avatar) = self.avatar.as_mut() {
            avatar.connect("avatar_loaded", &on_loaded);
        }
        self.base_mut().connect("resized", &on_resized);
        for node in [
            self.top_node_margin.clone(),
            self.bottom_node_margin.clone(),
        ]
        .into_iter()
        .flatten()
        {
            node.clone().connect("resized", &on_resized);
        }

        // Remembered so it can be restored if the flag flips back at runtime.
        self.baseline_offset_top = self.base().get_offset(Side::TOP);
        if self.snap_top_to_viewport {
            self.refresh_snap_top_state();
        }

        if DclGlobalStandalone::is_standalone() {
            let identity = globals::player_identity();
            identity.clone().bind_mut().set_default_profile();
            let profile = identity.bind().get_profile_or_null();
            if let (Some(profile), Some(avatar)) = (profile, self.avatar.clone()) {
                spawn_local(crate::avatars::avatar::build::update_from_profile(
                    avatar, profile,
                ));
            }
        }
    }

    fn process(&mut self, delta: f64) {
        if self.lerp_paused {
            return;
        }
        let delta = delta as f32;
        if self.can_drag
            && self.can_move
            && self.pinch_start_distance > 0.0
            && self.touch_points.len() >= 2
        {
            let distance = self.touch_distance();
            if distance > 0.0 {
                self.target_camera_size =
                    (self.pinch_start_camera_size * self.pinch_start_distance / distance).clamp(
                        MIN_CAMERA_SIZE_PART,
                        self.fitted_camera_size.max(MIN_CAMERA_SIZE_PART),
                    );
            }
            let mid_delta_y = self.touch_midpoint().y - self.pinch_start_midpoint.y;
            let pan = mid_delta_y * self.target_camera_size / self.base().get_size().y;
            let (lo, hi) = self.pan_limits();
            self.target_camera_center_y = (self.pinch_start_center_y + pan).clamp(lo, hi);
            self.user_has_panned = true;
        }
        let (center_y, camera_size, rotation_y) = (
            self.target_camera_center_y,
            self.target_camera_size,
            self.target_avatar_rotation_y,
        );
        if let Some(center) = self.camera_center.as_mut() {
            let mut position = center.get_position();
            position.y += (center_y - position.y) * (1.0 - (-CAMERA_PAN_SMOOTH * delta).exp());
            center.set_position(position);
        }
        if let Some(camera) = self.camera_3d.as_mut() {
            let size = camera.get_size();
            camera.set_size(
                size + (camera_size - size) * (1.0 - (-CAMERA_ZOOM_SMOOTH * delta).exp()),
            );
        }
        if let Some(avatar) = self.avatar.as_mut() {
            let mut rotation = avatar.get_rotation();
            rotation.y +=
                (rotation_y - rotation.y) * (1.0 - (-AVATAR_ROTATION_SMOOTH * delta).exp());
            avatar.set_rotation(rotation);
        }
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        if !self.can_move {
            return;
        }
        let inner = self.inner_rect();

        if let Ok(button) = event.clone().try_cast::<InputEventMouseButton>() {
            let index = button.get_button_index();
            let position = button.get_position();
            if index == MouseButton::LEFT {
                if button.is_pressed() {
                    if !inner.contains_point(position) {
                        return;
                    }
                    self.dragging = true;
                    self.start_dragging_position = self.base().get_global_mouse_position();
                    self.start_angle = self.target_avatar_rotation_y;
                } else {
                    self.dragging = false;
                }
            }
            if !button.is_pressed() && self.can_drag && inner.contains_point(position) {
                let direction = match index {
                    MouseButton::WHEEL_UP => -0.2,
                    MouseButton::WHEEL_DOWN => 0.2,
                    _ => 0.0,
                };
                if direction != 0.0 {
                    self.target_camera_size = (self.target_camera_size + direction).clamp(
                        MIN_CAMERA_SIZE_PART,
                        self.fitted_camera_size.max(MIN_CAMERA_SIZE_PART),
                    );
                    self.clamp_camera_center();
                }
            }
        }

        // Pinch zoom runs on touch and drag events: magnify gestures only exist on macOS
        // trackpads and are not emitted on iOS or Android.
        if let Ok(touch) = event.clone().try_cast::<InputEventScreenTouch>() {
            let index = touch.get_index();
            if touch.is_pressed() {
                if !inner.contains_point(touch.get_position()) {
                    return;
                }
                self.touch_points.insert(index, touch.get_position());
                if self.touch_points.len() == 2 && self.can_drag {
                    self.dragging = false;
                    self.pinch_start_distance = self.touch_distance();
                    self.pinch_start_camera_size = self.target_camera_size;
                    self.pinch_start_midpoint = self.touch_midpoint();
                    self.pinch_start_center_y = self.target_camera_center_y;
                }
            } else {
                self.touch_points.remove(&index);
                self.pinch_start_distance = 0.0;
                if self.touch_points.len() == 1 {
                    self.dragging = true;
                    self.start_dragging_position = self
                        .touch_points
                        .values()
                        .next()
                        .copied()
                        .unwrap_or_default();
                    self.start_angle = self.target_avatar_rotation_y;
                }
            }
        }

        if let Ok(drag) = event.clone().try_cast::<InputEventScreenDrag>() {
            self.touch_points
                .insert(drag.get_index(), drag.get_position());
        }

        if event.try_cast::<InputEventMouseMotion>().is_ok()
            && self.dragging
            && self.touch_points.len() < 2
        {
            let current = self.base().get_global_mouse_position();
            self.apply_drag(current);
        }
    }
}

/// `Global.standalone`, which is still a script property of the autoload.
struct DclGlobalStandalone;

impl DclGlobalStandalone {
    fn is_standalone() -> bool {
        crate::godot_classes::dcl_global::DclGlobal::singleton()
            .get("standalone")
            .booleanize()
    }
}

impl AvatarPreview {
    fn effective_margin_top(&self) -> f32 {
        let mut margin = self.preview_margin_top as f32;
        if let Some(node) = self.top_node_margin.as_ref() {
            if self.base().is_inside_tree() {
                margin += (node.get_global_position().y + node.get_size().y
                    - self.base().get_global_position().y)
                    .max(0.0);
            }
        }
        margin
    }

    fn effective_margin_bottom(&self) -> f32 {
        let mut margin = self.preview_margin_bottom as f32;
        if let Some(node) = self.bottom_node_margin.as_ref() {
            if self.base().is_inside_tree() {
                margin += (self.base().get_global_position().y + self.base().get_size().y
                    - node.get_global_position().y)
                    .max(0.0);
            }
        }
        margin
    }

    fn view(&self) -> FitView {
        FitView {
            size: self.base().get_size(),
            margin_left: self.preview_margin_left as f32,
            margin_right: self.preview_margin_right as f32,
            top: self.effective_margin_top(),
            bottom: self.effective_margin_bottom(),
        }
    }

    fn avatar_transform(&self) -> Transform3D {
        self.avatar
            .as_ref()
            .map(|a| a.get_global_transform())
            .unwrap_or(Transform3D::IDENTITY)
    }

    fn inner_rect(&self) -> Rect2 {
        let rect = self.base().get_global_rect();
        let (top, bottom) = (self.effective_margin_top(), self.effective_margin_bottom());
        let (left, right) = (
            self.preview_margin_left as f32,
            self.preview_margin_right as f32,
        );
        let size = Vector2::new(
            (rect.size.x - (left + right)).max(0.0),
            (rect.size.y - (top + bottom)).max(0.0),
        );
        Rect2::new(rect.position + Vector2::new(left, top), size)
    }

    fn touch_distance(&self) -> f32 {
        let mut points = self.touch_points.values();
        match (points.next(), points.next()) {
            (Some(a), Some(b)) => a.distance_to(*b),
            _ => 0.0,
        }
    }

    fn touch_midpoint(&self) -> Vector2 {
        let mut points = self.touch_points.values();
        match (points.next(), points.next()) {
            (Some(a), Some(b)) => (*a + *b) * 0.5,
            _ => Vector2::ZERO,
        }
    }

    fn pan_limits(&self) -> (f32, f32) {
        pan_limits(
            &self.cached_aabbs,
            self.view(),
            self.avatar_transform(),
            self.target_camera_size,
        )
    }

    fn clamp_camera_center(&mut self) {
        if !self.user_has_panned {
            let view = self.view();
            self.target_camera_center_y = self.fitted_aabb_center_y
                + (view.top - view.bottom) * self.target_camera_size / (2.0 * view.size.y);
        }
        let (lo, hi) = self.pan_limits();
        self.target_camera_center_y = self.target_camera_center_y.clamp(lo, hi);
    }

    fn apply_drag(&mut self, current: Vector2) {
        let drag = current - self.start_dragging_position;
        self.target_avatar_rotation_y = self.start_angle + drag.x * 0.005;
    }

    fn focus_key(&self) -> &str {
        if self.cached_aabbs.contains_key(&self.camera_focus) {
            &self.camera_focus
        } else {
            "overall"
        }
    }

    fn focus_aabb(&self) -> Option<Aabb> {
        self.cached_aabbs.get(self.focus_key()).copied()
    }

    fn apply_target(&mut self, target: CameraTarget) {
        self.target_camera_center_y = target.center_y;
        self.target_camera_size = target.size;
    }

    /// The initial fully-visible fit (`fit_avatar`), deferred until the layout settles.
    fn fit_to_overall(&mut self) {
        if !self.base().is_inside_tree() {
            return;
        }
        match fit_to_overall(&self.cached_aabbs, self.view(), self.avatar_transform()) {
            OverallFit::NoBody => {}
            OverallFit::Deferred => self.pending_fit_overall = true,
            OverallFit::Fit(target) => {
                self.pending_fit_overall = false;
                self.apply_target(target);
            }
        }
    }

    fn update_fit_limits(&mut self, aabb: Aabb, extra_margin: i32) {
        if !self.base().is_inside_tree() {
            return;
        }
        if let Some((camera_size, center_y)) = fit_limits(
            aabb,
            extra_margin,
            self.view(),
            &self.camera_focus,
            &self.cached_aabbs,
        ) {
            self.fitted_camera_size = camera_size;
            self.fitted_aabb_center_y = center_y;
        }
    }

    /// Frames a region. `instant` writes the camera directly (no lerp) so a first-load fit does
    /// not animate in from a wrong frame, but never while a capture owns the camera: a deferred
    /// first-load fit landing mid-capture clobbers the fixed ortho size and renders the avatar
    /// oversized (it broke the first avatar's snapshot).
    fn fit_camera_to_aabb(&mut self, aabb: Aabb, extra_margin: i32, instant: bool) {
        if !self.base().is_inside_tree() {
            return;
        }
        let view = self.view();
        if view.size.x <= 0.0 || view.size.y <= 0.0 {
            self.pending_camera_fit = true;
            return;
        }
        let available_h = view.top.mul_add(-1.0, view.size.y - view.bottom);
        let raw_inner_w =
            view.size.x - view.margin_left - view.margin_right - extra_margin as f32 * 2.0;
        // Judged without the extra margin, so tight but valid layouts (a panel near the top) are
        // not deferred forever because the category padding exceeds the remaining gap.
        if available_h < 50.0 || raw_inner_w < 50.0 {
            self.pending_camera_fit = true;
            return;
        }
        let capped_extra = extra_margin.min((available_h * 0.5) as i32 - 1);
        self.pending_camera_fit = false;
        self.update_fit_limits(aabb, capped_extra);
        let stable = stable_aabb(&self.camera_focus, &self.cached_aabbs);
        if self.last_fit_stable == Some(stable) && capped_extra == self.last_fit_extra_margin {
            return;
        }
        self.last_fit_stable = Some(stable);
        self.last_fit_extra_margin = capped_extra;
        let target = fit_camera(
            stable,
            aabb,
            capped_extra,
            view,
            self.avatar_transform(),
            self.fitted_camera_size,
            MIN_CAMERA_SIZE_PART,
        );
        self.apply_target(target);
        if instant && !self.lerp_paused {
            if let Some(camera) = self.camera_3d.as_mut() {
                camera.set_size(target.size);
            }
            if let Some(center) = self.camera_center.as_mut() {
                let mut position = center.get_position();
                position.y = target.center_y;
                center.set_position(position);
            }
        }
    }

    fn refit_focus(&mut self) {
        if let Some(aabb) = self.focus_aabb() {
            self.fit_camera_to_aabb(aabb, focus_extra_margin(&self.camera_focus), false);
        }
    }

    fn refresh_snap_top_state(&mut self) {
        let on_rect = self.base().callable("apply_snap_top_to_viewport");
        if self.snap_top_to_viewport {
            if !self.snap_top_connected {
                self.base_mut().connect("item_rect_changed", &on_rect);
                self.snap_top_connected = true;
            }
            self.apply_snap_top_to_viewport();
        } else {
            if self.snap_top_connected {
                self.base_mut().disconnect("item_rect_changed", &on_rect);
                self.snap_top_connected = false;
            }
            self.last_snapped_parent_y = None;
            let baseline = self.baseline_offset_top;
            if (self.base().get_offset(Side::TOP) - baseline).abs() > 1e-4 {
                self.base_mut().set_offset(Side::TOP, baseline);
            }
        }
    }

    /// The skinned bounds of a mesh in avatar space, measured against the current bone poses.
    fn skinned_mesh_aabb(
        mesh_instance: &Gd<MeshInstance3D>,
        skeleton: &Gd<Skeleton3D>,
        avatar_inverse: Transform3D,
    ) -> Aabb {
        let Some(mesh) = mesh_instance.get_mesh() else {
            return Aabb::default();
        };
        let Some(skin) = mesh_instance.get_skin() else {
            let local = mesh.get_aabb();
            if local.size == Vector3::ZERO {
                return Aabb::default();
            }
            let global = mesh_instance.get_global_transform();
            let mut result: Option<Aabb> = None;
            for i in 0..8u8 {
                let point = avatar_inverse * (global * aabb_endpoint(local, i));
                result = Some(match result {
                    Some(r) => r.expand(point),
                    None => Aabb::new(point, Vector3::ZERO),
                });
            }
            return result.unwrap_or_default();
        };
        let skin_count = skin.get_bind_count();
        if skin_count == 0 || !skeleton.is_inside_tree() {
            return Aabb::default();
        }
        let skeleton_global = skeleton.get_global_transform();
        let matrices: Vec<Transform3D> = (0..skin_count)
            .map(|i| {
                let mut bone = skin.get_bind_bone(i);
                if bone < 0 {
                    bone = skeleton.find_bone(&skin.get_bind_name(i).to_string());
                }
                if bone < 0 || bone >= skeleton.get_bone_count() {
                    Transform3D::IDENTITY
                } else {
                    skeleton_global * skeleton.get_bone_global_pose(bone) * skin.get_bind_pose(i)
                }
            })
            .collect();

        let mut result: Option<Aabb> = None;
        for surface in 0..mesh.get_surface_count() {
            let arrays = mesh.surface_get_arrays(surface);
            if arrays.is_empty() {
                continue;
            }
            let Ok(vertices) = arrays
                .at(godot::classes::mesh::ArrayType::VERTEX.ord() as usize)
                .try_to::<PackedVector3Array>()
            else {
                continue;
            };
            let bones_raw = arrays.at(godot::classes::mesh::ArrayType::BONES.ord() as usize);
            let Ok(weights) = arrays
                .at(godot::classes::mesh::ArrayType::WEIGHTS.ord() as usize)
                .try_to::<PackedFloat32Array>()
            else {
                continue;
            };
            let bones: Vec<i32> = if let Ok(ints) = bones_raw.try_to::<PackedInt32Array>() {
                ints.to_vec()
            } else if let Ok(floats) = bones_raw.try_to::<PackedFloat32Array>() {
                floats.to_vec().into_iter().map(|f| f as i32).collect()
            } else {
                continue;
            };
            let vertices = vertices.to_vec();
            let weights = weights.to_vec();
            if bones.is_empty()
                || vertices.is_empty()
                || !bones.len().is_multiple_of(vertices.len())
            {
                continue;
            }
            let influences = bones.len() / vertices.len();
            for (v, vertex) in vertices.iter().enumerate() {
                let mut skinned = Vector3::ZERO;
                for b in 0..influences {
                    let bind = bones[v * influences + b];
                    let weight = weights.get(v * influences + b).copied().unwrap_or(0.0);
                    if weight > 0.0 && bind >= 0 && bind < skin_count {
                        skinned += weight * (matrices[bind as usize] * *vertex);
                    }
                }
                if skinned == Vector3::ZERO {
                    continue;
                }
                let point = avatar_inverse * skinned;
                result = Some(match result {
                    Some(r) => r.expand(point),
                    None => Aabb::new(point, Vector3::ZERO),
                });
            }
        }
        result.unwrap_or_default()
    }

    /// The regions of the avatar in its local space: "overall" of what is visible, the body base
    /// parts that do not depend on wearables, and each category.
    fn compute_avatar_aabbs(&self) -> Aabbs {
        let Some(avatar) = self.avatar.as_ref() else {
            return Aabbs::new();
        };
        let Some(skeleton) = avatar.bind().body_shape_skeleton() else {
            return Aabbs::new();
        };
        let inverse = avatar.get_global_transform().affine_inverse();
        let mut results = Aabbs::new();
        let merge = |results: &mut Aabbs, key: &str, aabb: Aabb| {
            let merged = match results.get(key) {
                Some(existing) => existing.merge(aabb),
                None => aabb,
            };
            results.insert(key.to_string(), merged);
        };
        for child in skeleton.get_children().iter_shared() {
            let Ok(mesh_instance) = child.try_cast::<MeshInstance3D>() else {
                continue;
            };
            if mesh_instance.get_mesh().is_none() {
                continue;
            }
            let name = mesh_instance.get_name().to_string();
            let is_basemesh = name.contains("_basemesh");
            let visible = mesh_instance.is_visible();
            if !visible && !is_basemesh {
                continue;
            }
            let aabb = Self::skinned_mesh_aabb(&mesh_instance, &skeleton, inverse);
            if aabb.size == Vector3::ZERO {
                continue;
            }
            if is_basemesh {
                merge(&mut results, "body_base", aabb);
            }
            if visible {
                merge(&mut results, "overall", aabb);
            }
            let category = mesh_category(&name);
            if !category.is_empty() {
                merge(&mut results, category, aabb);
            }
            if name.contains("_hands_basemesh") {
                merge(&mut results, "hands", aabb);
                merge(&mut results, "hands_base", aabb);
            }
            if name.contains("_ubody_basemesh") {
                merge(&mut results, "torso_base", aabb);
            }
            if name.contains("_lbody_basemesh") {
                let knee = Aabb::new(
                    aabb.position,
                    Vector3::new(aabb.size.x, aabb.size.y * 0.5, aabb.size.z),
                );
                merge(&mut results, "feet", knee);
                merge(&mut results, "feet_base", knee);
                merge(&mut results, "legs_base", aabb);
            }
            if name.contains("_feet_basemesh") {
                merge(&mut results, "legs", aabb);
                merge(&mut results, "legs_base", aabb);
            }
            if name.contains("_head_basemesh") {
                merge(&mut results, "head_base", aabb);
                merge(&mut results, "head_base_facial", aabb);
            }
            if name.contains("__facial_hair") && visible {
                merge(&mut results, "head_base_facial", aabb);
            }
        }
        for key in ["feet", "feet_base"] {
            if let Some(a) = results.get_mut(key) {
                a.position += Vector3::new(0.0, -0.05, 0.0);
            }
        }
        results
            .into_iter()
            .map(|(key, aabb)| (key, symmetrize_aabb_x(aabb)))
            .collect()
    }

    fn update_aabb_debug_boxes(&mut self) {
        for mut node in self.aabb_debug_nodes.drain(..) {
            if node.is_instance_valid() {
                node.queue_free();
            }
        }
        if !self.show_aabb_debug || self.cached_aabbs.is_empty() {
            return;
        }
        let colors = [
            ("overall", Color::from_rgba(0.0, 1.0, 0.0, 0.15)),
            ("head", Color::from_rgba(0.0, 1.0, 1.0, 0.2)),
            ("torso", Color::from_rgba(1.0, 1.0, 0.0, 0.2)),
            ("legs", Color::from_rgba(1.0, 0.5, 0.0, 0.2)),
            ("feet", Color::from_rgba(0.0, 0.5, 1.0, 0.2)),
            ("hands", Color::from_rgba(1.0, 0.0, 1.0, 0.2)),
            ("head_base", Color::from_rgba(1.0, 0.0, 0.0, 0.2)),
            ("head_base_facial", Color::from_rgba(1.0, 0.5, 0.5, 0.2)),
        ];
        let Some(mut avatar) = self.avatar.clone() else {
            return;
        };
        for (key, color) in colors {
            let Some(aabb) = self.cached_aabbs.get(key).copied() else {
                continue;
            };
            if aabb.size == Vector3::ZERO {
                continue;
            }
            let box_node = make_aabb_box(aabb, color);
            avatar.add_child(&box_node);
            self.aabb_debug_nodes.push(box_node);
        }
    }
}

fn make_aabb_box(aabb: Aabb, color: Color) -> Gd<MeshInstance3D> {
    let center = aabb.center();
    let corners: Vec<Vector3> = (0..8u8).map(|i| aabb_endpoint(aabb, i) - center).collect();
    let mut mesh = ImmediateMesh::new_gd();
    mesh.surface_begin(godot::classes::mesh::PrimitiveType::LINES);
    const EDGES: [usize; 24] = [
        0, 1, 2, 3, 4, 5, 6, 7, 0, 2, 1, 3, 4, 6, 5, 7, 0, 4, 1, 5, 2, 6, 3, 7,
    ];
    for edge in EDGES.chunks(2) {
        mesh.surface_add_vertex(corners[edge[0]]);
        mesh.surface_add_vertex(corners[edge[1]]);
    }
    mesh.surface_end();
    let mut material = StandardMaterial3D::new_gd();
    material.set_shading_mode(godot::classes::base_material_3d::ShadingMode::UNSHADED);
    material.set_albedo(Color::from_rgba(color.r, color.g, color.b, 1.0));
    let mut node = MeshInstance3D::new_alloc();
    node.set_mesh(&mesh);
    node.set_material_override(&material);
    node.set_position(center);
    node
}

#[godot_api]
impl AvatarPreview {
    #[func]
    pub fn set_snap_top_to_viewport(&mut self, value: bool) {
        if value == self.snap_top_to_viewport {
            return;
        }
        self.snap_top_to_viewport = value;
        if self.base().is_inside_tree() {
            self.refresh_snap_top_state();
        }
    }

    #[func]
    fn apply_snap_top_to_viewport(&mut self) {
        if !self.snap_top_to_viewport || !self.base().is_inside_tree() {
            return;
        }
        let Some(parent) = self.base().get_parent_control() else {
            return;
        };
        let parent_y = parent.get_global_position().y;
        // Setting the offset fires `item_rect_changed` again: dedupe against the parent
        // position we last snapped to, or the handler recurses.
        if self
            .last_snapped_parent_y
            .is_some_and(|last| (last - parent_y).abs() < 1e-4)
        {
            return;
        }
        self.last_snapped_parent_y = Some(parent_y);
        // Grows the rect upward by the parent's distance from the screen top; the bottom edge
        // stays where the parent layout put it.
        let desired = self.baseline_offset_top - parent_y;
        if (self.base().get_offset(Side::TOP) - desired).abs() > 1e-4 {
            self.base_mut().set_offset(Side::TOP, desired);
        }
    }

    /// Frames the part of the avatar a wearable category edits.
    #[func]
    pub fn focus_camera_on(&mut self, category: GString, #[opt(default = false)] instant: bool) {
        self.camera_focus = if self.can_drag {
            focus_for_category(&category.to_string()).to_string()
        } else {
            "overall".to_string()
        };
        let Some(aabb) = self.focus_aabb() else {
            return;
        };
        let margin = focus_extra_margin(&self.camera_focus);
        if !instant
            && self.last_fit_stable == Some(stable_aabb(&self.camera_focus, &self.cached_aabbs))
            && margin == self.last_fit_extra_margin
        {
            self.update_fit_limits(aabb, margin);
            return;
        }
        self.user_has_panned = false;
        self.fit_camera_to_aabb(aabb, margin, instant);
    }

    #[func]
    pub fn reset_avatar_rotation(&mut self) {
        self.target_avatar_rotation_y = 0.0;
    }

    #[func]
    fn on_resized(&mut self) {
        let size = self.base().get_size();
        if size.x <= 0.0 || size.y <= 0.0 {
            return;
        }
        if self.cached_aabbs.contains_key("overall")
            && (self.pending_fit_overall
                || (self.fit_avatar && self.camera_focus == "overall" && !self.user_has_panned))
        {
            // The initial fit commits at whatever size the layout is passing through (safe-area
            // insets, sheet animation): keep it live until the user pans or zooms, or the
            // avatar reads tiny at the final size (#2685).
            self.fit_to_overall();
            return;
        }
        if self.pending_camera_fit {
            self.refit_focus();
        }
    }

    #[func]
    pub fn set_top_margin_node(&mut self, node: Option<Gd<Control>>) {
        let on_resized = self.base().callable("on_resized");
        let changed = self.top_node_margin.as_ref().map(|n| n.instance_id())
            != node.as_ref().map(|n| n.instance_id());
        if !changed {
            return;
        }
        if let Some(old) = self.top_node_margin.as_mut() {
            if old.is_connected("resized", &on_resized) {
                old.disconnect("resized", &on_resized);
            }
        }
        self.top_node_margin = node;
        if let Some(new) = self.top_node_margin.as_mut() {
            new.connect("resized", &on_resized);
        }
        self.refit_after_margin_change();
    }

    #[func]
    pub fn set_bottom_margin_node(&mut self, node: Option<Gd<Control>>) {
        let on_resized = self.base().callable("on_resized");
        let changed = self.bottom_node_margin.as_ref().map(|n| n.instance_id())
            != node.as_ref().map(|n| n.instance_id());
        if !changed {
            return;
        }
        if let Some(old) = self.bottom_node_margin.as_mut() {
            if old.is_connected("resized", &on_resized) {
                old.disconnect("resized", &on_resized);
            }
        }
        self.bottom_node_margin = node;
        if let Some(new) = self.bottom_node_margin.as_mut() {
            new.connect("resized", &on_resized);
        }
        self.refit_after_margin_change();
    }

    #[func]
    pub fn enable_outline(&mut self) {
        let avatar = self.avatar.clone();
        if let (Some(outline), Some(avatar)) = (self.outline_system.as_mut(), avatar) {
            outline
                .bind_mut()
                .set_outlined_avatar(Some(avatar.upcast()));
        }
    }

    #[func]
    pub fn disable_outline(&mut self) {
        if let Some(outline) = self.outline_system.as_mut() {
            outline.bind_mut().set_outlined_avatar(None);
        }
    }

    #[func]
    fn on_avatar_loaded(&mut self) {
        let preview = self.to_gd();
        spawn_local(async move {
            if !preview.is_instance_valid() || !preview.is_inside_tree() {
                return;
            }
            next_frame().await;
            if !preview.is_instance_valid() || !preview.is_inside_tree() {
                return;
            }
            let mut preview = preview;
            preview.bind_mut().after_avatar_loaded();
        });
    }

    /// The overall box of the visible meshes in avatar space, skinned against the current bone
    /// poses (a frozen emote pose or a lone wearable is measured as displayed); 2 m when empty.
    #[func]
    pub fn compute_current_fit_aabb(&self) -> Aabb {
        self.compute_avatar_aabbs()
            .get("overall")
            .copied()
            .unwrap_or(Aabb::new(
                Vector3::new(-1.0, 0.0, -1.0),
                Vector3::new(2.0, 2.0, 2.0),
            ))
    }

    /// An off-screen capture with a camera the caller positions (the asset renderer tool). It
    /// does not force the idle pose and leaves the camera entirely to the caller.
    #[func]
    pub fn async_capture_current_view(
        &mut self,
        dest_size: Vector2i,
        #[opt(default = 1)] ssaa: i32,
    ) -> Gd<Promise> {
        let preview = self.to_gd();
        capture(preview, dest_size, ssaa.max(1), None)
    }

    /// A capture of the profile body or face, in the idle pose, from a fixed camera.
    #[func]
    pub fn async_get_viewport_image(
        &mut self,
        face: bool,
        dest_size: Vector2i,
        #[opt(default = 2.5)] ortho_size: f32,
        #[opt(default = 1)] ssaa: i32,
    ) -> Gd<Promise> {
        if let Some(avatar) = self.avatar.as_mut() {
            avatar.bind_mut().freeze_on_idle();
            let mut rotation = avatar.get_rotation();
            rotation.y = 0.0;
            avatar.set_rotation(rotation);
        }
        let preview = self.to_gd();
        capture(preview, dest_size, ssaa.max(1), Some((face, ortho_size)))
    }
}

impl AvatarPreview {
    fn refit_after_margin_change(&mut self) {
        // A re-fit with the new reference: reset the dedupe so it applies.
        self.pending_camera_fit = true;
        let size = self.base().get_size();
        if size.x > 0.0 && size.y > 0.0 && !self.cached_aabbs.is_empty() {
            if let Some(aabb) = self.focus_aabb() {
                self.last_fit_stable = None;
                self.last_fit_extra_margin = -1;
                self.fit_camera_to_aabb(aabb, focus_extra_margin(&self.camera_focus), false);
            }
        }
    }

    fn after_avatar_loaded(&mut self) {
        let first_load = self.cached_aabbs.is_empty();
        self.cached_aabbs = self.compute_avatar_aabbs();
        self.update_aabb_debug_boxes();
        let Some(aabb) = self.focus_aabb() else {
            return;
        };
        let margin = focus_extra_margin(&self.camera_focus);
        if first_load {
            if self.fit_avatar && self.camera_focus == "overall" {
                self.fit_to_overall();
            } else {
                let preview = self.to_gd();
                spawn_local(async move {
                    next_frame().await;
                    if preview.is_instance_valid() {
                        preview
                            .clone()
                            .bind_mut()
                            .fit_camera_to_aabb(aabb, margin, true);
                    }
                });
            }
        } else {
            self.update_fit_limits(aabb, margin);
        }
    }
}

/// Renders the viewport at `dest_size * ssaa` and downsamples (Lanczos) to `dest_size`: the
/// scaling_3d_scale is clamped to 2.0 by the engine, so this is the only way past 2x SSAA. With
/// `profile_camera` (face, ortho size) the camera is the fixed profile one and restored after.
fn capture(
    preview: Gd<AvatarPreview>,
    dest_size: Vector2i,
    ssaa: i32,
    profile_camera: Option<(bool, f32)>,
) -> Gd<Promise> {
    let promise = Promise::new_alloc();
    let handle = promise.clone();
    spawn_local(async move {
        let mut node = preview.clone();
        let (original_stretch, original_size, mut camera, mut subviewport, saved) = {
            let mut this = node.bind_mut();
            let original_stretch = this.base().is_stretch_enabled();
            let original_size = this.base().get_size();
            let mut camera = this.camera_3d.clone();
            let mut subviewport = this.subviewport.clone();
            let saved = camera.as_ref().map(|c| {
                (
                    c.get_position(),
                    c.get_rotation_degrees(),
                    c.get_size(),
                    this.target_camera_center_y,
                    this.target_camera_size,
                )
            });
            this.lerp_paused = true;
            if let (Some((face, ortho)), Some(camera)) = (profile_camera, camera.as_mut()) {
                camera.set_as_top_level(true);
                camera.set_position(if face {
                    PROFILE_HEAD_CAMERA_POSITION
                } else {
                    PROFILE_BODY_CAMERA_POSITION
                });
                camera.set_rotation_degrees(if face {
                    PROFILE_HEAD_CAMERA_ROTATION
                } else {
                    PROFILE_BODY_CAMERA_ROTATION
                });
                camera.set_size(ortho);
            }
            let render_size = dest_size * ssaa;
            this.base_mut().set_stretch(false);
            this.base_mut()
                .set_size(Vector2::new(render_size.x as f32, render_size.y as f32));
            if let Some(viewport) = subviewport.as_mut() {
                viewport.set_size(render_size);
            }
            (original_stretch, original_size, camera, subviewport, saved)
        };
        let _ = &mut subviewport;
        for _ in 0..5 {
            next_frame().await;
            if !preview.is_instance_valid() {
                handle.clone().bind_mut().reject("preview freed".into());
                return;
            }
        }
        let mut image: Option<Gd<Image>> = subviewport
            .as_ref()
            .and_then(|v| v.get_texture())
            .and_then(|t| t.get_image());
        if let (Some(img), true) = (image.as_mut(), ssaa > 1) {
            img.resize_ex(dest_size.x, dest_size.y)
                .interpolation(Interpolation::LANCZOS)
                .done();
        }
        {
            let mut this = node.bind_mut();
            if let (
                Some(_),
                Some(camera),
                Some((position, rotation, size, center_y, target_size)),
            ) = (profile_camera, camera.as_mut(), saved)
            {
                camera.set_as_top_level(false);
                camera.set_position(position);
                camera.set_rotation_degrees(rotation);
                camera.set_size(size);
                this.target_camera_center_y = center_y;
                this.target_camera_size = target_size;
            }
            this.base_mut().set_stretch(original_stretch);
            this.base_mut().set_size(original_size);
            this.lerp_paused = false;
        }
        match image {
            Some(image) => handle
                .clone()
                .bind_mut()
                .resolve_with_data(image.to_variant()),
            None => handle.clone().bind_mut().reject("no viewport image".into()),
        }
    });
    promise
}
