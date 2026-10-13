use godot::prelude::*;

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
#[class(init, base=Node)]
pub struct DclEngine {
    world: Option<WorldHandles>,
    base: Base<Node>,
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
