use godot::classes::{Camera3D, ICamera3D, INode, Node, Node3D, Viewport};
use godot::prelude::*;

use super::body::{CameraMode, Player};
use crate::{
    fetcher::globals,
    godot_classes::{dcl_camera_3d::DclCamera3D, dcl_global::DclGlobal},
};

const DEFAULT_TRANSITION_TIME: f32 = 0.35;
const DEFAULT_VIRTUAL_CAMERA_FOV: f32 = 60.0;
/// The virtual camera sees everything but the outline layer (bit 19).
const VIRTUAL_CAMERA_CULL_MASK: u32 = 0x7fff;
/// After the camera clamp (-20) and before the scene manager's kick
/// (`SCENE_MANAGER_KICK_PROCESS_PRIORITY`, -5), so a virtual-camera blend is final when the scenes
/// receive the camera pose.
const PROCESS_PRIORITY: i32 = -10;

/// A camera that survives its parent being freed: when it leaves the tree it re-attaches itself to
/// the viewport it was in, so a scene entity it was reparented under can disappear without
/// orphaning it.
#[derive(GodotClass)]
#[class(init, base=Camera3D)]
pub struct PersistantCamera {
    base: Base<Camera3D>,
    fallback_viewport: Option<Gd<Viewport>>,
}

#[godot_api]
impl ICamera3D for PersistantCamera {
    fn ready(&mut self) {
        let this = self.to_gd();
        self.base_mut()
            .signals()
            .tree_exiting()
            .connect_other(&this, |this: &mut Self| this.on_tree_exiting());
    }
}

impl PersistantCamera {
    fn on_tree_exiting(&mut self) {
        self.fallback_viewport = self.base().get_viewport();
        self.run_deferred(|this| this.safety_reparent());
    }
}

#[godot_api]
impl PersistantCamera {
    #[func]
    fn safety_reparent(&mut self) {
        // Already back in the tree after a valid reparent: nothing to do.
        if self.base().is_inside_tree() {
            return;
        }
        // Otherwise it is orphaned: reparent to the viewport as a safety net.
        if let Some(mut viewport) = self.fallback_viewport.clone() {
            if viewport.is_instance_valid() && viewport.is_inside_tree() {
                let node = self.to_gd();
                viewport.add_child(&node);
            }
        }
    }
}

/// Transitions the view between the player camera and the scene's virtual camera.
///
/// Virtual cameras can change between scenes or inside one, computed as the desired target; the
/// target can be removed from the scene tree, so the global virtual camera can be orphaned at any
/// point. With no target it flies back to the player camera (reparented under this node, keeping
/// its global transform, then handing "current" back); with one it flies to the node and is
/// reparented under it once there.
#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct DclGlobalCameraController {
    base: Base<Node>,
    global_virtual_camera: Option<Gd<PersistantCamera>>,
    last_virtual_camera_entity_node: Option<Gd<Node3D>>,
    last_camera_reached: bool,
    transition_start_transform: Transform3D,
    transition_time_counter: f32,
}

#[godot_api]
impl INode for DclGlobalCameraController {
    fn ready(&mut self) {
        self.base_mut().set_process_priority(PROCESS_PRIORITY);
        self.last_camera_reached = true;
        let camera = self.create_virtual_camera();
        self.global_virtual_camera = Some(camera);
        // A scene always starts non-cinematic. The cinematic/virtual-camera state is reset
        // whenever the current parcel scene changes so the latch can't survive a teleport.
        let this = self.to_gd();
        globals::scene_runner()
            .signals()
            .on_change_scene_id()
            .connect_other(&this, |this: &mut Self, _scene_id| {
                this.on_parcel_scene_changed()
            });
    }

    fn process(&mut self, delta: f64) {
        self.tick(delta as f32);
    }
}

fn player_camera() -> Option<Gd<DclCamera3D>> {
    DclGlobal::singleton().bind().player_camera()
}

fn set_global_camera_mode(mode: CameraMode) {
    DclGlobal::set_camera_mode(mode.to_i32());
}

impl DclGlobalCameraController {
    fn create_virtual_camera(&mut self) -> Gd<PersistantCamera> {
        let mut camera = PersistantCamera::new_alloc();
        self.base_mut().add_child(&camera);
        camera.clear_current();
        camera.set_cull_mask(VIRTUAL_CAMERA_CULL_MASK);
        camera
    }

    fn tick(&mut self, delta: f32) {
        // The virtual camera was freed (the scene exited while it was reparented to an entity
        // node): reinitialize it.
        if !self
            .global_virtual_camera
            .as_ref()
            .is_some_and(|camera| camera.is_instance_valid())
        {
            let camera = self.create_virtual_camera();
            self.global_virtual_camera = Some(camera);
            self.last_virtual_camera_entity_node = None;
            self.last_camera_reached = true;
        }
        // The last tracked entity was freed: reset so the comparison below doesn't misfire.
        if !self
            .last_virtual_camera_entity_node
            .as_ref()
            .is_some_and(|node| node.is_instance_valid())
        {
            self.last_virtual_camera_entity_node = None;
        }
        let Some(mut camera) = self.global_virtual_camera.clone() else {
            return;
        };
        let Some(player_camera) = player_camera() else {
            return;
        };

        let mut runner = globals::scene_runner();
        let current_scene_id = runner.bind().get_current_parcel_scene_id();
        let scene_virtual_camera = runner.bind().get_scene_virtual_camera(current_scene_id);

        let mut desired_target: Option<Gd<Node3D>> = None;
        let mut look_at_entity_node: Option<Gd<Node3D>> = None;
        let mut transition_speed = 0.0;
        let mut transition_time;

        match scene_virtual_camera
            .filter(|virtual_camera| virtual_camera.bind().get_entity_id() > 0)
        {
            Some(virtual_camera) => {
                let virtual_camera = virtual_camera.bind();
                desired_target = runner.bind().get_scene_entity_node_or_null_3d(
                    current_scene_id,
                    virtual_camera.get_entity_id() as u32,
                );
                let look_at = virtual_camera.get_look_at_entity_id();
                if look_at > 0 {
                    look_at_entity_node = if look_at == 1 {
                        // The player's body (todo: it should be the feet).
                        player_camera
                            .get_parent_node_3d()
                            .and_then(|arm| arm.get_parent())
                            .and_then(|mount| mount.try_cast::<Node3D>().ok())
                    } else {
                        runner
                            .bind()
                            .get_scene_entity_node_or_null_3d(current_scene_id, look_at as u32)
                    };
                }
                transition_speed = virtual_camera.get_transition_speed();
                transition_time = virtual_camera.get_transition_time();
                runner.bind_mut().set_raycast_use_cursor_position(true);
            }
            None => {
                transition_time = DEFAULT_TRANSITION_TIME;
                // Latch guard: no active scene virtual camera, yet the cursor mode is still on
                // while the transition is already marked complete. That happens when a
                // virtual-camera sub-scene is torn down within the same scene: the target entity
                // is removed and the `last_camera_reached` short-circuit below returns before the
                // normal transition-back can reset state, leaving camera rotation frozen and taps
                // moving the crosshair. raycast_use_cursor_position is only ever set by an active
                // scene virtual camera, so seeing it true here is unambiguously the latch:
                // force a clean return to the player camera. The normal smooth transition-back
                // runs with last_camera_reached == false, so this never cuts it short.
                if self.last_camera_reached && runner.bind().get_raycast_use_cursor_position() {
                    self.force_return_to_player_camera();
                    return;
                }
            }
        }

        if desired_target != self.last_virtual_camera_entity_node {
            self.last_camera_reached = false;
            match &desired_target {
                None => {
                    // Going back to the player camera: reparent to self conserving the global
                    // transform.
                    if camera.get_parent().as_ref() != Some(&self.to_gd().upcast::<Node>()) {
                        camera.reparent(&self.to_gd());
                    }
                    let mode = player_camera.bind().get_camera_mode();
                    set_global_camera_mode(CameraMode::from_i32(mode));
                }
                Some(_) => {
                    // Switching to the virtual camera: start from the current viewport camera.
                    let current_camera = self
                        .base()
                        .get_viewport()
                        .and_then(|viewport| viewport.get_camera_3d());
                    if let Some(current) = current_camera {
                        if current.is_instance_valid()
                            && current.instance_id() != camera.instance_id()
                        {
                            camera.set_global_transform(current.get_global_transform());
                        }
                    }
                    camera.make_current();
                    camera.set_fov(DEFAULT_VIRTUAL_CAMERA_FOV);
                    set_global_camera_mode(CameraMode::Cinematic);

                    // With a virtual camera active the primary avatar is always shown and the
                    // outline hidden. set_hidden(false) rather than show(), so an
                    // AvatarModifierArea hide still applies.
                    if let Some(player) = explorer_player() {
                        if let Some(mut avatar) = player.bind().get_avatar() {
                            avatar.bind_mut().apply_hidden(false);
                        }
                        if let Some(mut outline) = player.bind().get_outline_system() {
                            outline.hide();
                        }
                    }
                    // Entering tap-cursor mode: re-center both the visual crosshair and the
                    // pointer raycast cursor so aim starts on the framed target (not a stale tap
                    // from a previous virtual-camera session).
                    self.center_pointer_cursor();
                }
            }
            self.transition_time_counter = 0.0;
            self.transition_start_transform = camera.get_global_transform();
            self.last_virtual_camera_entity_node = desired_target.clone();
        } else if self.last_camera_reached {
            // Already attached.
            if let Some(look_at) = &look_at_entity_node {
                if look_at.is_instance_valid() {
                    camera.look_at(look_at.get_global_position());
                }
            }
            return;
        }

        // The effective transition time (the default when both are 0).
        let effective_transition_time;
        if transition_speed == 0.0 && transition_time == 0.0 {
            effective_transition_time = DEFAULT_TRANSITION_TIME;
            self.transition_time_counter = effective_transition_time;
        } else if transition_time > 0.0 {
            effective_transition_time = transition_time;
        } else {
            // From speed and distance.
            let target_transform = match &desired_target {
                Some(target) => target.get_global_transform(),
                None => player_camera.get_global_transform(),
            };
            let distance = self
                .transition_start_transform
                .origin
                .distance_to(target_transform.origin);
            effective_transition_time = if transition_speed > 0.0 {
                distance / transition_speed
            } else {
                DEFAULT_TRANSITION_TIME
            };
        }
        transition_time = effective_transition_time;

        self.transition_time_counter += delta;
        // A constant-speed interpolation factor.
        let t = (self.transition_time_counter / transition_time).clamp(0.0, 1.0);

        match &desired_target {
            None => {
                // Transitioning to the player camera.
                camera.set_global_transform(
                    self.transition_start_transform
                        .interpolate_with(&player_camera.get_global_transform(), t),
                );
                // Once next to the player camera, hand "current" back.
                if t >= 1.0 {
                    let mut player_camera = player_camera.clone();
                    player_camera.make_current();
                    camera.clear_current();
                    self.last_camera_reached = true;
                    runner.bind_mut().set_raycast_use_cursor_position(false);
                    // Coming back from the virtual camera the outline system is shown again and
                    // the first/third person behaviour restored.
                    self.restore_player_camera_visibility();
                }
            }
            Some(target) => {
                // Transitioning to the virtual camera target.
                camera.set_global_transform(
                    self.transition_start_transform
                        .interpolate_with(&target.get_global_transform(), t),
                );
                // Once next to the node, reparent the global virtual camera under it.
                if t >= 1.0
                    && camera.get_parent().as_ref() != Some(&target.clone().upcast::<Node>())
                {
                    camera.reparent(target);
                    camera.set_transform(Transform3D::IDENTITY);
                    self.last_camera_reached = true;
                }
                if let Some(look_at) = &look_at_entity_node {
                    if look_at.is_instance_valid() {
                        camera.look_at(look_at.get_global_position());
                    }
                }
            }
        }
    }

    fn on_parcel_scene_changed(&mut self) {
        // A teleport can free the virtual camera's target entity (and the camera reparented
        // under it), latching this controller in its re-init branch and leaving
        // raycast_use_cursor_position / CINEMATIC stuck, which freezes camera rotation. Only
        // reset when actually latched, so normal scene crossings don't re-tween or replay the
        // camera-mode fade. If the new scene is itself cinematic, `tick` re-enters it on the next
        // frame (its virtual-camera component is only applied then).
        let cinematic =
            DclGlobal::singleton().bind().current_camera_mode == CameraMode::Cinematic.to_i32();
        if cinematic
            || globals::scene_runner()
                .bind()
                .get_raycast_use_cursor_position()
        {
            self.force_return_to_player_camera();
        }
    }

    fn force_return_to_player_camera(&mut self) {
        // Keep the persistent camera alive (it may be parented to a now-freed scene entity) and
        // detach it from any virtual-camera target.
        let this: Gd<Node> = self.to_gd().upcast();
        if let Some(camera) = &mut self.global_virtual_camera {
            if camera.is_instance_valid() {
                if camera.get_parent().as_ref() != Some(&this) {
                    camera.reparent(&this);
                }
                camera.clear_current();
            }
        }
        self.last_virtual_camera_entity_node = None;
        self.last_camera_reached = true;
        self.transition_time_counter = 0.0;
        globals::scene_runner()
            .bind_mut()
            .set_raycast_use_cursor_position(false);
        if let Some(mut player_camera) = player_camera() {
            player_camera.make_current();
            let mode = player_camera.bind().get_camera_mode();
            set_global_camera_mode(CameraMode::from_i32(mode));
        }
        self.restore_player_camera_visibility();
    }

    fn restore_player_camera_visibility(&self) {
        if let Some(player) = explorer_player() {
            if let Some(mut outline) = player.bind().get_outline_system() {
                outline.show();
            }
            let first_person = player.bind().get_camera().is_some_and(|camera| {
                camera.bind().get_camera_mode() == CameraMode::FirstPerson.to_i32()
            });
            if first_person {
                if let Some(mut avatar) = player.bind().get_avatar() {
                    avatar.bind_mut().apply_hidden(true);
                }
            }
        }
        // Reset BOTH the visual crosshair and the raycast cursor so they cannot desync
        // (reset_cursor_position alone only moves the visual, leaving cursor_position stale).
        self.center_pointer_cursor();
    }

    /// Centers both the visual crosshair and the pointer raycast cursor. While a virtual camera
    /// is active the explorer uses tap-cursor raycasting, so the crosshair and the raycast
    /// origin are separate state that must be kept in sync.
    fn center_pointer_cursor(&self) {
        crate::world::reset_cursor_position();
        if let Some(viewport) = self.base().get_viewport() {
            let center = viewport.get_visible_rect().size * 0.5;
            globals::scene_runner()
                .bind_mut()
                .set_cursor_position(center);
        }
    }
}

fn explorer_player() -> Option<Gd<Player>> {
    crate::world::state::player()
}
