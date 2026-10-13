use std::rc::Rc;

use godot::classes::{Control, DisplayServer};
use godot::prelude::*;

use super::scene_requests::{EmulatorSink, GodotUiSink, PendingResponses, SharedSink};

use crate::godot_classes::dcl_global::DclGlobal;

/// Nodes of the running world (`explorer.tscn`), registered by the explorer when it is ready and
/// dropped when it leaves the tree. Held by instance id: any of them can be freed between frames.
#[derive(Clone, Copy)]
struct WorldHandles {
    explorer: InstanceId,
    player: InstanceId,
    avatar: InstanceId,
    touch_camera_input: InstanceId,
    dialog_stack: InstanceId,
}

/// Owner of the engine-side handles. `Global.engine` exposes it; `explorer.gd` attaches the world.
#[derive(GodotClass)]
#[class(base=Node)]
pub struct DclEngine {
    world: Option<WorldHandles>,
    pub(super) pending: PendingResponses,
    pub(super) sink: SharedSink,
    base: Base<Node>,
}

#[godot_api]
impl INode for DclEngine {
    fn init(base: Base<Node>) -> Self {
        Self {
            world: None,
            pending: PendingResponses::default(),
            sink: Rc::new(GodotUiSink),
            base,
        }
    }
}

fn resolve(id: InstanceId) -> Option<Gd<Node>> {
    Gd::<Node>::try_from_instance_id(id).ok()
}

#[godot_api]
impl DclEngine {
    /// The engine owned by the `Global` autoload, `None` before it exists.
    pub fn get() -> Option<Gd<DclEngine>> {
        Some(DclGlobal::try_singleton()?.bind().get_engine())
    }

    /// Registers the world once the explorer has built its player; the handles are dropped when
    /// the explorer leaves the tree.
    #[func]
    pub fn attach_world(&mut self, mut explorer: Gd<Node>) {
        let node_property =
            |owner: &Gd<Node>, name: &str| owner.get(name).try_to::<Gd<Node>>().ok();
        let (Some(player), Some(touch_camera_input), Some(dialog_stack)) = (
            node_property(&explorer, "player"),
            node_property(&explorer, "mobile_camera_input"),
            explorer.get_node_or_null("UI/DialogStack/Stack"),
        ) else {
            tracing::error!(
                "engine: the explorer is missing the player, camera input or dialog stack"
            );
            return;
        };
        let Some(avatar) = node_property(&player, "avatar") else {
            tracing::error!("engine: the player has no avatar");
            return;
        };
        self.world = Some(WorldHandles {
            explorer: explorer.instance_id(),
            player: player.instance_id(),
            avatar: avatar.instance_id(),
            touch_camera_input: touch_camera_input.instance_id(),
            dialog_stack: dialog_stack.instance_id(),
        });
        let detach = self.base().callable("detach_world");
        explorer.connect_flags(
            "tree_exiting",
            &detach,
            godot::classes::object::ConnectFlags::ONE_SHOT,
        );
    }

    #[func]
    pub fn detach_world(&mut self) {
        self.world = None;
    }

    /// Answers a scene request a sink left pending.
    #[func]
    pub fn answer_scene_request(&mut self, id: u32, accepted: bool) {
        if let Some(responder) = self.pending.take(id) {
            responder.send(if accepted {
                Ok(())
            } else {
                Err("the user declined".to_string())
            });
        }
    }

    /// The emulator accepts what a scene asks instead of showing a modal.
    #[func]
    pub fn set_emulator_mode(&mut self, enabled: bool) {
        self.sink = if enabled {
            Rc::new(EmulatorSink)
        } else {
            Rc::new(GodotUiSink)
        };
    }

    #[func]
    pub fn has_world(&self) -> bool {
        self.world
            .is_some_and(|world| resolve(world.explorer).is_some())
    }

    fn explorer(&self) -> Option<Gd<Node>> {
        resolve(self.world?.explorer)
    }

    pub fn player(&self) -> Option<Gd<Node>> {
        resolve(self.world?.player)
    }

    fn avatar(&self) -> Option<Gd<Node>> {
        resolve(self.world?.avatar)
    }

    pub fn touch_camera_input(&self) -> Option<Gd<Node>> {
        resolve(self.world?.touch_camera_input)
    }

    /// Where dialogs opened by scenes are parented.
    pub fn dialog_stack(&self) -> Option<Gd<Node>> {
        resolve(self.world?.dialog_stack)
    }

    /// Whether the explorer UI has focus; `true` when there is no world to ask.
    pub fn ui_has_focus(&self) -> bool {
        match self.explorer() {
            Some(mut explorer) if explorer.has_method("ui_has_focus") => {
                explorer.call("ui_has_focus", &[]).to::<bool>()
            }
            _ => true,
        }
    }

    // ---- facade for the host (the JSON bridge today, the shell later)

    /// Moves the local player to the centre of `parcel` in the current realm.
    #[func]
    pub fn teleport(&self, parcel: Vector2i) {
        if let Some(mut explorer) = self.explorer() {
            explorer.call("teleport_to", &[parcel.to_variant()]);
        }
    }

    /// Plays an emote on the local avatar.
    #[func]
    pub fn play_local_emote(&self, urn: GString) {
        if let Some(mut avatar) = self.avatar() {
            avatar.call("async_play_emote", &[urn.to_variant()]);
        }
    }

    /// Where the host's native controls sit, in device pixels: the HUD and scene UI keep clear
    /// of these margins.
    #[func]
    pub fn set_hud_insets(&self, top: f32, right: f32, bottom: f32, left: f32) {
        let Some(mut explorer) = self.explorer() else {
            return;
        };
        let Ok(mut hud) = explorer.get("safe_area_hud").try_to::<Gd<Control>>() else {
            return;
        };
        for (side, pixels) in [
            ("top", top),
            ("right", right),
            ("bottom", bottom),
            ("left", left),
        ] {
            let units = Self::device_pixels_to_viewport_units(&explorer, pixels);
            hud.add_theme_constant_override(&format!("margin_{side}"), units as i32);
        }
        explorer.call("_push_scene_interactable_area", &[]);
    }

    /// The host reports device pixels; Godot lays out in its scaled viewport units.
    fn device_pixels_to_viewport_units(explorer: &Gd<Node>, pixels: f32) -> f32 {
        let window_width = DisplayServer::singleton().window_get_size().x;
        let viewport_width = explorer
            .get_viewport()
            .map(|viewport| viewport.get_visible_rect().size.x);
        match viewport_width {
            Some(width) if window_width > 0 => pixels * width / window_width as f32,
            _ => pixels,
        }
    }

    // ---- local player

    pub fn move_player_to(&self, position: Vector3, skip_loading: bool) {
        if let Some(mut explorer) = self.explorer() {
            explorer.call(
                "move_to",
                &[position.to_variant(), skip_loading.to_variant()],
            );
        }
    }

    pub fn player_look_at(&self, target: Vector3) {
        if let Some(mut explorer) = self.explorer() {
            explorer.call("player_look_at", &[target.to_variant()]);
        }
    }

    pub fn camera_look_at(&self, target: Vector3) {
        if let Some(mut explorer) = self.explorer() {
            explorer.call("camera_look_at", &[target.to_variant()]);
        }
    }

    pub fn avatar_look_at_independent(&self, target: Vector3) {
        if let Some(mut explorer) = self.explorer() {
            explorer.call("avatar_look_at_independent", &[target.to_variant()]);
        }
    }

    // ---- local avatar emotes

    pub fn play_emote(&self, emote_id: &str, mask: i64, scene_id: i32) {
        if let Some(mut avatar) = self.avatar() {
            avatar.call(
                "async_play_emote",
                &[
                    emote_id.to_variant(),
                    mask.to_variant(),
                    scene_id.to_variant(),
                ],
            );
        }
    }

    /// Lets the avatar find the content of a scene emote by the scene's id.
    pub fn register_scene_emote_content(
        &self,
        scene_entity_id: &str,
        base_url: &str,
        glb_hash: &str,
        audio_hash: &str,
    ) {
        if let Some(mut avatar) = self.avatar() {
            avatar.call(
                "register_scene_emote_content",
                &[
                    scene_entity_id.to_variant(),
                    base_url.to_variant(),
                    glb_hash.to_variant(),
                    audio_hash.to_variant(),
                ],
            );
        }
    }

    pub fn stop_emote_from_scene(&self, scene_id: i32) {
        if let Some(mut avatar) = self.avatar() {
            avatar.call("stop_emote_from_scene", &[scene_id.to_variant()]);
        }
    }
}
