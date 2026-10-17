//! Avatar nameplates: the `NicknameUi` control and the shared screen-space layer that projects,
//! fades, stacks and occludes every avatar's plate (#2215).

use std::cell::RefCell;
use std::collections::HashMap;

use godot::classes::control::{LayoutPreset, MouseFilter};
use godot::classes::text_server::AutowrapMode;
use godot::classes::{
    CanvasLayer, Control, IMarginContainer, Label, MarginContainer, PackedScene,
    PhysicsRayQueryParameters3D, Tween,
};
use godot::prelude::*;

use crate::godot_classes::dcl_avatar::DclAvatar;
use crate::godot_classes::dcl_global::DclGlobal;

const MAX_CHAR_COUNT: usize = 80;
const MAX_WIDTH: f32 = 1200.0;
const TWEEN_DURATION: f64 = 0.25;
const MESSAGE_DURATION: f64 = 6.0;

/// The plate: nickname, claimed checkmark or `#tag`, mic icon, debug labels and the chat bubble.
#[derive(GodotClass)]
#[class(base=MarginContainer)]
pub struct NicknameUi {
    #[export]
    message_container_scene: Option<Gd<PackedScene>>,
    #[export]
    #[var(set = set_name_claimed)]
    name_claimed: bool,
    #[export]
    #[var(pub, set = set_mic_enabled)]
    mic_enabled: bool,
    #[export]
    #[var(set = set_nickname)]
    nickname: GString,
    #[export]
    #[var(set = set_tag)]
    tag: GString,
    #[export]
    #[var(set = set_nickname_color)]
    nickname_color: Color,
    #[export]
    #[var(set = set_client_version)]
    client_version: GString,
    #[export]
    #[var(set = set_room_debug)]
    room_debug: GString,
    message_tween: Option<Gd<Tween>>,
    base: Base<MarginContainer>,
}

#[godot_api]
impl IMarginContainer for NicknameUi {
    fn init(base: Base<MarginContainer>) -> Self {
        Self {
            message_container_scene: None,
            name_claimed: false,
            mic_enabled: false,
            nickname: "nickname".into(),
            tag: "xxxx".into(),
            nickname_color: Color::WHITE,
            client_version: GString::new(),
            room_debug: GString::new(),
            message_tween: None,
            base,
        }
    }

    fn ready(&mut self) {
        // Values set before the node entered the tree were stored only.
        self.apply_all();
    }
}

impl NicknameUi {
    fn node<T: GodotClass + Inherits<Node>>(&self, unique_name: &str) -> Option<Gd<T>> {
        self.base()
            .get_node_or_null(format!("%{unique_name}").as_str())
            .and_then(|n| n.try_cast::<T>().ok())
    }

    fn apply_all(&mut self) {
        let (claimed, mic, nickname, tag, color, version, room) = (
            self.name_claimed,
            self.mic_enabled,
            self.nickname.clone(),
            self.tag.clone(),
            self.nickname_color,
            self.client_version.clone(),
            self.room_debug.clone(),
        );
        self.set_name_claimed(claimed);
        self.set_mic_enabled(mic);
        self.set_nickname(nickname);
        self.set_tag(tag);
        self.set_nickname_color(color);
        self.set_client_version(version);
        self.set_room_debug(room);
    }

    fn create_message_container(&self, message: &str) -> Option<Gd<Control>> {
        let scene = self.message_container_scene.as_ref()?;
        let container = scene.instantiate()?.try_cast::<Control>().ok()?;
        let mut label = container.get_child(0)?.try_cast::<Label>().ok()?;
        label.set_text(message);
        Some(container)
    }

    /// Shows a chat bubble over the plate for a few seconds.
    pub fn show_message(&mut self, message: &str) {
        let mut message = message.to_string();
        if message.chars().count() > MAX_CHAR_COUNT {
            message = format!(
                "{}...",
                message.chars().take(MAX_CHAR_COUNT).collect::<String>()
            );
        }
        let Some(mut clip) = self.node::<Control>("MessageClip") else {
            return;
        };
        for mut child in clip.get_children().iter_shared() {
            child.queue_free();
        }
        if message.is_empty() {
            return;
        }
        if let Some(mut tween) = self.message_tween.take() {
            if tween.is_valid() {
                tween.kill();
            }
        }

        let Some(mut container) = self.create_message_container(&message) else {
            return;
        };
        clip.set_custom_minimum_size(Vector2::ZERO);
        clip.add_child(&container);

        let mut width = container.get_size().x;
        if width >= MAX_WIDTH {
            width = MAX_WIDTH;
            container.queue_free();
            let Some(wrapped) = self.create_message_container(&message) else {
                return;
            };
            if let Some(mut label) = wrapped
                .get_child(0)
                .and_then(|n| n.try_cast::<Label>().ok())
            {
                label.set_autowrap_mode(AutowrapMode::WORD_SMART);
                label.set_custom_minimum_size(Vector2::new(width, 0.0));
            }
            clip.add_child(&wrapped);
            container = wrapped;
        }
        let final_size = Vector2::new(width, container.get_size().y);

        let mut tween = self.base_mut().create_tween();
        let clip_object = clip.clone().upcast::<Object>();
        tween.tween_property(
            &clip_object,
            "custom_minimum_size",
            &final_size.to_variant(),
            TWEEN_DURATION,
        );
        tween.tween_property(
            &clip_object,
            "custom_minimum_size",
            &final_size.to_variant(),
            MESSAGE_DURATION,
        );
        let mut sub = self.base_mut().create_tween();
        sub.set_parallel();
        sub.tween_property(
            &clip_object,
            "custom_minimum_size",
            &Vector2::ZERO.to_variant(),
            TWEEN_DURATION,
        );
        sub.tween_property(
            &clip_object,
            "size",
            &Vector2::ZERO.to_variant(),
            TWEEN_DURATION,
        );
        tween.tween_subtween(&sub);
        let doomed = container.clone();
        tween.signals().finished().connect(move || {
            if doomed.is_instance_valid() {
                doomed.clone().queue_free();
            }
        });
        self.message_tween = Some(tween);
    }
}

#[godot_api]
impl NicknameUi {
    #[func]
    pub fn set_name_claimed(&mut self, value: bool) {
        self.name_claimed = value;
        if !self.base().is_inside_tree() {
            return;
        }
        if let Some(mut hash) = self.node::<Control>("Hash") {
            hash.set_visible(!value);
        }
        if let Some(mut check) = self.node::<Control>("ClaimedCheckmark") {
            check.set_visible(value);
        }
    }

    #[func]
    pub fn set_mic_enabled(&mut self, value: bool) {
        self.mic_enabled = value;
        if !self.base().is_inside_tree() {
            return;
        }
        if let Some(mut icon) = self.node::<Control>("MicEnabled") {
            icon.set_visible(value);
        }
    }

    #[func]
    pub fn set_nickname(&mut self, value: GString) {
        self.nickname = value.clone();
        if !self.base().is_inside_tree() {
            return;
        }
        if let Some(mut label) = self.node::<Label>("Nickname") {
            label.set_text(&value);
        }
    }

    #[func]
    pub fn set_tag(&mut self, value: GString) {
        self.tag = value.clone();
        if !self.base().is_inside_tree() {
            return;
        }
        if let Some(mut label) = self.node::<Label>("Tag") {
            label.set_text(&value);
        }
    }

    #[func]
    pub fn set_nickname_color(&mut self, value: Color) {
        self.nickname_color = value;
        if !self.base().is_inside_tree() {
            return;
        }
        if let Some(mut label) = self.node::<Label>("Nickname") {
            label.add_theme_color_override("font_color", value);
        }
    }

    #[func]
    pub fn set_client_version(&mut self, value: GString) {
        self.client_version = value.clone();
        if !self.base().is_inside_tree() {
            return;
        }
        if let Some(mut label) = self.node::<Label>("VersionLabel") {
            label.set_visible(!value.is_empty());
            label.set_text(&value);
        }
    }

    #[func]
    pub fn set_room_debug(&mut self, value: GString) {
        self.room_debug = value.clone();
        if !self.base().is_inside_tree() {
            return;
        }
        if let Some(mut label) = self.node::<Label>("RoomDebugLabel") {
            label.set_visible(!value.is_empty());
            label.set_text(&value);
        }
    }

    #[func]
    pub fn async_show_message(&mut self, message: GString) {
        self.show_message(&message.to_string());
    }
}

// On-screen size and distance fade (full below FADE_START, none at FADE_END).
pub const SCALE: f32 = 0.2875;
/// Fraction of the plate height kept above the head anchor.
pub const ANCHOR_HEIGHT_FACTOR: f32 = 0.85;
pub const FADE_START: f32 = 10.0;
pub const FADE_END: f32 = 15.0;
/// Alpha per second for the occlusion fade.
pub const FADE_SPEED: f32 = 6.0;
const CL_PHYSICS: u32 = 2;
const CL_PLAYER: u32 = 4;
const BODY_MASK: u32 = CL_PHYSICS | CL_PLAYER;
/// Frames between occlusion raycasts per avatar (staggered).
pub const OCCLUSION_PERIOD: u64 = 6;
pub const NAMETAG_MARGIN: f32 = 0.1;
const STACK_PAD: f32 = 2.0;
/// Fraction of the plate height left out (top and bottom) of the collision rect: the visible
/// pill is much smaller than the control.
const STACK_COLLISION_INSET: f32 = 0.22;
/// Fraction of the overlap corrected per frame, so stacking eases instead of snapping.
const STACK_RELAX: f32 = 0.3;
/// Extra separation other plates keep from a fading one.
const STACK_FADE_PAD: f32 = 30.0;
const STACK_FADE_MIN_RELAX: f32 = 0.15;

/// De-overlap state per visible plate, keyed by the plate's instance id. Each carries an offset
/// from its projected anchor that springs home when nothing overlaps and eases apart when
/// plates do. O(n²) over the plates within FADE_END; n is small.
#[derive(Default)]
pub struct PlateSolver {
    rects: HashMap<i64, Rect2>,
    offsets: HashMap<i64, Vector2>,
    alphas: HashMap<i64, f32>,
    /// World point each plate is drawn at: occlusion rays go there, so a cluster behind one
    /// occluder does not lose every tag.
    ray_targets: HashMap<i64, Vector3>,
}

impl PlateSolver {
    /// Where to draw plate `id` given where it would be alone. A plate yields only to those
    /// with a lower id, so a pair never escalates.
    pub fn stack_position(
        &mut self,
        id: i64,
        alpha: f32,
        desired: Vector2,
        screen_size: Vector2,
        view_size: Vector2,
    ) -> Vector2 {
        let alpha = alpha.clamp(0.0, 1.0);
        let offset = self.offsets.get(&id).copied().unwrap_or(Vector2::ZERO);
        let inset = screen_size.y * STACK_COLLISION_INSET;
        let col_size = Vector2::new(screen_size.x, screen_size.y - inset * 2.0);

        // Clear any overlapping plate along the axis of least penetration: sideways when
        // that is cheaper, upward otherwise (no name towers).
        let mut target = Vector2::ZERO;
        for (&other_id, &other) in &self.rects {
            if other_id == id || id < other_id {
                continue;
            }
            let candidate = Rect2::new(desired + target + Vector2::new(0.0, inset), col_size);
            if !candidate.intersects(other) {
                continue;
            }
            let other_alpha = self.alphas.get(&other_id).copied().unwrap_or(1.0);
            let pad = STACK_PAD + (1.0 - other_alpha) * STACK_FADE_PAD;
            let other_end = other.position + other.size;
            let candidate_end = candidate.position + candidate.size;
            let overlap_x =
                candidate_end.x.min(other_end.x) - candidate.position.x.max(other.position.x);
            let overlap_y =
                candidate_end.y.min(other_end.y) - candidate.position.y.max(other.position.y);
            if overlap_x < overlap_y {
                let other_center_x = other.position.x + other.size.x * 0.5;
                if desired.x + col_size.x * 0.5 >= other_center_x {
                    target.x = other_end.x + pad - desired.x;
                } else {
                    target.x = other.position.x - pad - col_size.x - desired.x;
                }
            } else {
                target.y = other.position.y - pad - (desired.y + inset) - col_size.y;
            }
        }
        let relax = STACK_RELAX * (STACK_FADE_MIN_RELAX + (1.0 - STACK_FADE_MIN_RELAX) * alpha);
        let offset = offset.lerp(target, relax);
        let mut pos = desired + offset;
        pos.y = pos.y.max(0.0);
        pos.x = pos.x.clamp(0.0, (view_size.x - screen_size.x).max(0.0));
        self.offsets.insert(id, pos - desired);
        self.rects
            .insert(id, Rect2::new(pos + Vector2::new(0.0, inset), col_size));
        self.alphas.insert(id, alpha);
        pos
    }

    /// Hidden, occluded and off-screen plates reserve no space.
    pub fn untrack(&mut self, id: i64) {
        self.rects.remove(&id);
        self.offsets.remove(&id);
        self.alphas.remove(&id);
        self.ray_targets.remove(&id);
    }

    pub fn set_ray_target(&mut self, id: i64, target: Vector3) {
        self.ray_targets.insert(id, target);
    }

    pub fn ray_target(&self, id: i64) -> Option<Vector3> {
        self.ray_targets.get(&id).copied()
    }

    #[cfg(test)]
    fn rect(&self, id: i64) -> Option<Rect2> {
        self.rects.get(&id).copied()
    }
}

thread_local! {
    static SOLVER: RefCell<PlateSolver> = RefCell::new(PlateSolver::default());
    static ROOT: RefCell<Option<Gd<Control>>> = const { RefCell::new(None) };
}

/// Debug: bypass the occlusion raycast so tags fade by distance only.
pub static DEBUG_DISABLE_OCCLUSION: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

pub fn with_solver<R>(f: impl FnOnce(&mut PlateSolver) -> R) -> R {
    SOLVER.with(|solver| f(&mut solver.borrow_mut()))
}

/// The control nameplates are parented under (screen-space), created on first use under
/// `Global`: each avatar owns its plate, so the layer must outlive every avatar.
pub fn layer_root() -> Gd<Control> {
    ROOT.with(|slot| {
        if let Some(root) = slot.borrow().as_ref().filter(|r| r.is_instance_valid()) {
            return root.clone();
        }
        let mut layer = CanvasLayer::new_alloc();
        layer.set_name("NameplateLayer");
        // Above the 3D world, below the default-layer HUD.
        layer.set_layer(-1);
        let mut root = Control::new_alloc();
        root.set_name("Root");
        root.set_anchors_preset(LayoutPreset::FULL_RECT);
        root.set_mouse_filter(MouseFilter::IGNORE);
        layer.add_child(&root);
        DclGlobal::singleton().add_child(&layer);
        *slot.borrow_mut() = Some(root.clone());
        root
    })
}

/// Picks the occlusion query: the ray from the camera to where the tag is drawn.
pub fn occlusion_query(
    from: Vector3,
    to: Vector3,
    exclude: Array<Rid>,
) -> Gd<PhysicsRayQueryParameters3D> {
    // Bodies only: a query with areas would also hit the nodeless DCL scene-sensor Area3Ds on
    // CL_PLAYER that sat in front of the third-person camera and hid every tag (#2321).
    let mut query = PhysicsRayQueryParameters3D::create(from, to).expect("ray query");
    query.set_collision_mask(BODY_MASK);
    query.set_collide_with_areas(false);
    query.set_exclude(&exclude);
    query
}

pub fn avatar_obj_id(avatar: &DclAvatar) -> i64 {
    avatar.base().instance_id().to_i64()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZE: Vector2 = Vector2::new(200.0, 40.0);
    const VIEW: Vector2 = Vector2::new(1600.0, 720.0);

    #[test]
    fn unconstrained_plate_stays_on_its_anchor() {
        let mut solver = PlateSolver::default();
        let desired = Vector2::new(100.0, 100.0);
        assert_eq!(solver.stack_position(1, 1.0, desired, SIZE, VIEW), desired);
    }

    #[test]
    fn overlapping_plates_separate() {
        let mut solver = PlateSolver::default();
        solver.stack_position(1, 1.0, Vector2::new(100.0, 100.0), SIZE, VIEW);
        // The higher id yields; the spring relaxes over frames.
        for _ in 0..200 {
            solver.stack_position(2, 1.0, Vector2::new(120.0, 110.0), SIZE, VIEW);
        }
        let a = solver.rect(1).unwrap();
        let b = solver.rect(2).unwrap();
        assert!(!a.intersects(b));
    }

    #[test]
    fn untracked_plate_reserves_no_space() {
        let mut solver = PlateSolver::default();
        solver.stack_position(1, 1.0, Vector2::new(100.0, 100.0), SIZE, VIEW);
        solver.untrack(1);
        let pos = solver.stack_position(3, 1.0, Vector2::new(100.0, 100.0), SIZE, VIEW);
        assert_eq!(pos, Vector2::new(100.0, 100.0));
    }

    #[test]
    fn plates_stay_inside_the_view() {
        let mut solver = PlateSolver::default();
        let pos = solver.stack_position(1, 1.0, Vector2::new(-50.0, -20.0), SIZE, VIEW);
        assert!(pos.x >= 0.0 && pos.y >= 0.0);
        let pos = solver.stack_position(2, 1.0, Vector2::new(5000.0, 10.0), SIZE, VIEW);
        assert!(pos.x <= VIEW.x - SIZE.x);
    }
}
