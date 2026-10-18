//! Executes the core's typed commands on the engine and reports what the engine observes
//! (`engine.core_command` in, `engine.report_*` out). Active when the host drives the app.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use godot::classes::{INode, Node};
use godot::prelude::*;

use super::backpack::Backpack;
use super::chat_commands;
use super::enter;
use super::hud::Hud;
use super::session_watch::SessionWatch;
use super::state;
use crate::engine::local_task::{next_frame, spawn_local};
use crate::fetcher::api::DclNavigator;
use crate::fetcher::globals;
use crate::godot_classes::dcl_global::DclGlobal;
use crate::player::body::CameraMode;
use dcl_core::app::{CameraMode as HudCameraMode, HudSnapshot, Parcel, Vec3};

/// Seconds between the HUD snapshots reported while in a world.
const STATE_INTERVAL: f32 = 0.25;
/// Commands that act on the avatar preview, which exists with or without a world.
const AVATAR_CONTROLS: [&str; 3] = ["apply_avatar", "play_preview_emote", "set_name"];

#[derive(Clone)]
struct EnterArgs {
    parcel: Vector2i,
    realm: String,
}

struct Host {
    backpack: Backpack,
    hud: Rc<Hud>,
    watch: Rc<SessionWatch>,
    /// The world the host believes is up (set when the explorer shows, cleared as it leaves).
    world_up: Cell<bool>,
    avatar_host_up: Cell<bool>,
    exiting: Cell<bool>,
    entering: Cell<bool>,
    pending_enter: RefCell<Option<EnterArgs>>,
    preview_url: RefCell<String>,
    state_elapsed: Cell<f32>,
}

thread_local! {
    static HOST: RefCell<Option<Rc<Host>>> = const { RefCell::new(None) };
}

fn host() -> Option<Rc<Host>> {
    HOST.with(|host| host.borrow().clone())
}

/// The autoload node of the shell-driven app.
#[derive(GodotClass)]
#[class(base=Node, init)]
pub struct EngineHost {
    base: Base<Node>,
}

#[godot_api]
impl INode for EngineHost {
    fn ready(&mut self) {
        if !globals::engine().bind().is_host_driven() {
            self.base_mut().set_process(false);
            return;
        }
        let me = self.to_gd();
        let host = Rc::new(Host {
            backpack: Backpack::new(me.clone().upcast()),
            hud: Rc::new(Hud::default()),
            watch: Rc::new(SessionWatch::default()),
            world_up: Cell::new(false),
            avatar_host_up: Cell::new(false),
            exiting: Cell::new(false),
            entering: Cell::new(false),
            pending_enter: RefCell::new(None),
            preview_url: RefCell::new(String::new()),
            state_elapsed: Cell::new(0.0),
        });
        HOST.with(|slot| *slot.borrow_mut() = Some(host.clone()));
        globals::engine()
            .signals()
            .core_command()
            .connect_other(&me, |_, kind, args| {
                if let Some(host) = host_rc() {
                    host.on_core_command(&kind.to_string(), &args);
                }
            });
        host.hud.connect(&me);
        host.watch.connect(&me);
        spawn_local(first_launch_benchmark());
    }

    fn process(&mut self, delta: f64) {
        if let Some(host) = host_rc() {
            host.process(delta as f32);
        }
    }
}

fn host_rc() -> Option<Rc<Host>> {
    host()
}

/// The hardware benchmark that picks the first graphics profile, once the engine is up.
async fn first_launch_benchmark() {
    next_frame().await;
    next_frame().await;
    let autotune = DclGlobal::singleton().bind().graphics_autotune.clone();
    if autotune.bind().should_run_first_launch_benchmark() {
        autotune.clone().bind_mut().run_first_launch_benchmark();
    }
}

/// Called by the avatar-only host scene once the identity and profile exist.
pub fn on_avatar_host_ready() {
    let Some(host) = host() else {
        return;
    };
    host.avatar_host_up.set(true);
    host.exiting.set(false);
    host.preload_backpack();
    let pending = host.pending_enter.borrow_mut().take();
    if let Some(args) = pending {
        host.clone().enter_world_now(args);
    }
}

impl Host {
    fn process(&self, delta: f32) {
        if self.avatar_host_up.get() {
            return;
        }
        if !self.world_up.get() {
            if state::current().is_none() {
                return;
            }
            self.world_up.set(true);
        }
        self.state_elapsed.set(self.state_elapsed.get() + delta);
        if self.state_elapsed.get() >= STATE_INTERVAL {
            self.state_elapsed.set(0.0);
            self.report_hud();
        }
    }

    fn on_core_command(self: &Rc<Self>, kind: &str, args: &VarDictionary) {
        let int = |key: &str| {
            args.get(key)
                .and_then(|v| v.try_to::<i64>().ok())
                .unwrap_or(0) as i32
        };
        let text = |key: &str| args.get(key).map(|v| v.to_string()).unwrap_or_default();
        match kind {
            "enter_world" => self.enter_world(EnterArgs {
                parcel: Vector2i::new(int("x"), int("y")),
                realm: text("realm"),
            }),
            "exit_world" => self.exit_world(),
            "reconnect" => self.watch.reconnect(),
            "set_world_secret" => DclNavigator::set_world_secret(
                text("realm").as_str().into(),
                text("secret").as_str().into(),
            ),
            "show_avatar_preview" => {
                let rect = Rect2::new(
                    Vector2::new(int("x") as f32, int("y") as f32),
                    Vector2::new(int("w") as f32, int("h") as f32),
                );
                let me = self.clone();
                if self.backpack.is_shown() {
                    self.backpack.place(rect);
                } else {
                    spawn_local(async move { me.backpack.open(rect).await });
                }
            }
            "hide_avatar_preview" => self.backpack.close(),
            "set_preview_url" => {
                *self.preview_url.borrow_mut() = text("url");
                if self.world_up.get() {
                    globals::scene_fetcher()
                        .bind_mut()
                        .set_preview_url(text("url").as_str().into());
                }
            }
            "focus_camera" => self.backpack.focus_camera(&text("focus")),
            _ if AVATAR_CONTROLS.contains(&kind) => self.run_avatar_control(kind, args),
            _ if self.world_up.get() => self.run_world_control(kind, args),
            _ => {}
        }
    }

    fn run_avatar_control(self: &Rc<Self>, kind: &str, args: &VarDictionary) {
        let text = |key: &str| args.get(key).map(|v| v.to_string()).unwrap_or_default();
        let me = self.clone();
        match kind {
            "apply_avatar" => {
                let args = args.clone();
                spawn_local(async move { me.backpack.apply_avatar(&args).await });
            }
            "play_preview_emote" => self.backpack.play_emote(&text("urn")),
            "set_name" => {
                let name = text("name");
                spawn_local(async move { me.backpack.set_profile_name(&name).await });
            }
            _ => {}
        }
    }

    fn run_world_control(&self, kind: &str, args: &VarDictionary) {
        let int = |key: &str| {
            args.get(key)
                .and_then(|v| v.try_to::<i64>().ok())
                .unwrap_or(0) as i32
        };
        let text = |key: &str| args.get(key).map(|v| v.to_string()).unwrap_or_default();
        match kind {
            "teleport" => globals::engine()
                .bind()
                .teleport(Vector2i::new(int("x"), int("y"))),
            "send_chat" => chat_commands::submit_message(&text("text")),
            "emote" => globals::engine()
                .bind()
                .play_local_emote(text("urn").as_str().into()),
            _ => {
                self.hud.handle(kind, args);
            }
        }
    }

    fn enter_world(self: &Rc<Self>, args: EnterArgs) {
        if self.world_up.get() {
            // The core replaced the destination while a world is up: leave it, enter on the host's
            // return.
            *self.pending_enter.borrow_mut() = Some(args);
            self.exit_world();
            return;
        }
        if !self.avatar_host_up.get() {
            return;
        }
        if self.exiting.get() {
            *self.pending_enter.borrow_mut() = Some(args);
            return;
        }
        self.clone().enter_world_now(args);
    }

    fn enter_world_now(self: Rc<Self>, args: EnterArgs) {
        self.backpack.close();
        self.avatar_host_up.set(false);
        self.entering.set(true);
        self.world_up.set(false);
        let preview = self.preview_url.borrow().clone();
        if !preview.is_empty() {
            DclGlobal::singleton()
                .bind()
                .deep_link_obj
                .clone()
                .bind_mut()
                .preview = preview.as_str().into();
        }
        spawn_local(async move {
            let me = self.clone();
            let started = enter::enter(args.parcel, args.realm, move || !me.entering.get()).await;
            if started {
                self.entering.set(false);
            } else {
                on_avatar_host_ready();
            }
        });
    }

    fn exit_world(&self) {
        if self.entering.get() {
            self.entering.set(false);
        } else if self.world_up.get() {
            self.backpack.close();
            self.hud.reset();
            self.world_up.set(false);
            self.avatar_host_up.set(true);
            self.exiting.set(true);
            spawn_local(enter::exit());
        }
    }

    fn preload_backpack(self: &Rc<Self>) {
        let me = self.clone();
        spawn_local(async move { me.backpack.preload().await });
    }

    fn report_hud(&self) {
        let Some(world) = state::current() else {
            return;
        };
        let position = world.player.get_global_position();
        let parcel = world.parcel.get();
        let scene_base = {
            let runner = globals::scene_runner();
            let runner = runner.bind();
            let id = runner.get_current_parcel_scene_id();
            (id >= 0).then(|| {
                let base = runner.get_scene_base_parcel(id);
                Parcel::new(base.x, base.y)
            })
        };
        let (camera_mode, joystick_hidden) = {
            let global = DclGlobal::singleton();
            let global = global.bind();
            let mode = match CameraMode::from_i32(global.current_camera_mode) {
                CameraMode::FirstPerson => HudCameraMode::FirstPerson,
                CameraMode::ThirdPerson => HudCameraMode::ThirdPerson,
                CameraMode::Cinematic => HudCameraMode::Cinematic,
            };
            (mode, global.touch_controls_hide_joystick)
        };
        let joystick = &world.joystick;
        globals::engine().bind().report_hud(HudSnapshot {
            parcel: Parcel::new(parcel.x, parcel.y),
            realm: globals::realm().bind().get_realm_string().to_string(),
            fps: godot::classes::Engine::singleton().get_frames_per_second() as f32,
            position: Vec3 {
                x: position.x,
                y: position.y,
                z: position.z,
            },
            scene_base,
            camera_mode,
            // A scene can hide the joystick's graphic without hiding the node.
            joystick_visible: joystick.is_visible_in_tree()
                && joystick.get_modulate().a > 0.0
                && !joystick_hidden,
        });
    }
}
