//! Avatar nameplates: the `NicknameUi` control and the shared screen-space layer that projects,
//! fades and occludes every avatar's plate (#2215).

use std::cell::RefCell;
use std::collections::HashMap;

use godot::classes::control::{LayoutPreset, MouseFilter};
use godot::classes::text_server::AutowrapMode;
use godot::classes::{
    CanvasLayer, Control, IMarginContainer, Label, MarginContainer, PackedScene,
    PhysicsRayQueryParameters3D, Tween,
};
use godot::prelude::*;

use super::nameplate_tiers::{Tier, TierState};
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
    profile_status: GString,
    status_label: Option<Gd<Label>>,
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
            profile_status: GString::new(),
            status_label: None,
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
        let (claimed, mic, nickname, tag, color, version, room, status) = (
            self.name_claimed,
            self.mic_enabled,
            self.nickname.clone(),
            self.tag.clone(),
            self.nickname_color,
            self.client_version.clone(),
            self.room_debug.clone(),
            self.profile_status.clone(),
        );
        self.set_name_claimed(claimed);
        self.set_mic_enabled(mic);
        self.set_nickname(nickname);
        self.set_tag(tag);
        self.set_nickname_color(color);
        self.set_client_version(version);
        self.set_room_debug(room);
        self.set_profile_status(status);
    }

    /// Shows the rows the tier allows: `#hash` only in FULL, the badge (checkmark and mic)
    /// dropped in NAME. The chat bubble follows `show_chat` (distance), not the capped tier.
    pub fn apply_tier(&mut self, tier: Tier, show_chat: bool) {
        let (claimed, mic) = (self.name_claimed, self.mic_enabled);
        if let Some(mut hash) = self.node::<Control>("Hash") {
            hash.set_visible(tier == Tier::Full && !claimed);
        }
        if let Some(mut clip) = self.node::<Control>("MessageClip") {
            clip.set_visible(show_chat);
        }
        if let Some(mut check) = self.node::<Control>("ClaimedCheckmark") {
            check.set_visible(tier != Tier::Name && claimed);
        }
        if let Some(mut icon) = self.node::<Control>("MicEnabled") {
            icon.set_visible(tier != Tier::Name && mic);
        }
    }

    /// The setters write row visibility directly: put the applied tier back on top.
    fn reassert_tier(&mut self) {
        let id = self.base().instance_id().to_i64();
        if let Some((tier, show_chat)) = with_plates(|plates| plates.tiers.applied(id)) {
            self.apply_tier(tier, show_chat);
        }
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
        self.reassert_tier();
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
        self.reassert_tier();
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

    /// Translation key shown under the name while the profile is pending or from another
    /// environment; empty hides the row.
    #[func]
    pub fn set_profile_status(&mut self, value: GString) {
        self.profile_status = value.clone();
        if !self.base().is_inside_tree() {
            return;
        }
        if self.status_label.is_none() {
            let Some(version) = self.node::<Label>("VersionLabel") else {
                return;
            };
            let Some(mut parent) = version.get_parent() else {
                return;
            };
            let mut label = Label::new_alloc();
            label.set_horizontal_alignment(godot::global::HorizontalAlignment::CENTER);
            label.set_auto_translate_mode(godot::classes::node::AutoTranslateMode::ALWAYS);
            label.add_theme_font_size_override("font_size", 30);
            parent.add_child(&label);
            self.status_label = Some(label);
        }
        if let Some(label) = self.status_label.as_mut() {
            label.set_visible(!value.is_empty());
            label.set_text(&value);
        }
    }

    #[func]
    pub fn async_show_message(&mut self, message: GString) {
        self.show_message(&message.to_string());
    }
}

// On-screen size and distance fade; the content tiers are in `nameplate_tiers`.
pub const SCALE: f32 = 0.2875;
/// Fraction of the plate height kept above the head anchor.
pub const ANCHOR_HEIGHT_FACTOR: f32 = 0.85;
/// Fully faded out from here.
pub const FADE_END: f32 = 25.0;
/// Alpha per second for the occlusion fade.
pub const FADE_SPEED: f32 = 6.0;
pub(crate) const CL_PHYSICS: u32 = 2;
const CL_PLAYER: u32 = 4;
const BODY_MASK: u32 = CL_PHYSICS | CL_PLAYER;
/// A plate with alpha 0 whose head is farther than this skips the anchor and projection work.
pub const FAR_SKIP_DISTANCE: f32 = FADE_END + 5.0;
pub const NAMETAG_MARGIN: f32 = 0.1;

/// Per-plate state keyed by the plate's instance id.
#[derive(Default)]
pub struct Plates {
    /// World point each plate is drawn at: occlusion rays go there, so a cluster behind one
    /// occluder does not lose every tag.
    ray_targets: HashMap<i64, Vector3>,
    pub tiers: TierState,
}

impl Plates {
    /// Hidden, occluded and off-screen plates are not tracked.
    pub fn untrack(&mut self, id: i64) {
        self.ray_targets.remove(&id);
        self.tiers.untrack(id);
    }

    pub fn set_ray_target(&mut self, id: i64, target: Vector3) {
        self.ray_targets.insert(id, target);
    }

    pub fn ray_target(&self, id: i64) -> Option<Vector3> {
        self.ray_targets.get(&id).copied()
    }
}

thread_local! {
    static PLATES: RefCell<Plates> = RefCell::new(Plates::default());
    static ROOT: RefCell<Option<Gd<Control>>> = const { RefCell::new(None) };
}

/// Debug: bypass the occlusion raycast so tags fade by distance only.
pub static DEBUG_DISABLE_OCCLUSION: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

pub fn with_plates<R>(f: impl FnOnce(&mut Plates) -> R) -> R {
    PLATES.with(|plates| f(&mut plates.borrow_mut()))
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
        let mut driver = super::nameplate_occlusion::NameplateOcclusionDriver::new_alloc();
        driver.set_name("OcclusionDriver");
        layer.add_child(&driver);
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
