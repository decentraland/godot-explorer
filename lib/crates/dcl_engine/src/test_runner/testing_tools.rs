//! The scene tests: the explorer visits each parcel of `--scene-test`, waits for the scene's own
//! test plan to finish and answers the snapshot comparisons the scenes ask for. Inactive without
//! `--scene-test`.

use godot::classes::node::ProcessMode;
use godot::classes::sub_viewport::UpdateMode;
use godot::classes::{
    Camera3D, Control, DirAccess, FileAccess, Image, Input, InputEventScreenDrag,
    InputEventScreenTouch, Node, Node3D, RenderingServer, SubViewport,
};
use godot::prelude::*;

use super::image_compare::{rgb_difference, rgb_similarity};
use super::scene_plan::{
    parse_parcels, summarize, SceneReport, SceneTest, SCENE_TESTS_TIMEOUT, SCENE_TIMEOUT,
};
use super::snapshot_files::{snapshot_file_name, SnapshotFolders};
use crate::dcl::components::proto_components::sdk::components::PbUiCanvasInformation;
use crate::dcl::SceneId;
use crate::engine::local_task::{await_all, defer, next_frame, sleep, spawn_local};
use crate::fetcher::globals;
use crate::godot_classes::dcl_avatar::DclAvatar;
use crate::godot_classes::dcl_global::DclGlobal;
use crate::godot_classes::rpc_sender::take_and_compare_snapshot_response::DclRpcSenderTakeAndCompareSnapshotResponse;
use crate::scene_runner::components::ui::scene_ui::relayout_for_capture;

/// The camera layers that show the world without the outline layer (20).
const CAMERA_CULL_MASK: u32 = 524287;
/// How often the content queue is checked before a capture, and how many times in a row it has
/// to be empty.
const LOAD_CHECKS: u32 = 100;
const STABLE_CHECKS: u32 = 3;
const LOAD_CHECK_INTERVAL: f64 = 0.1;
const SETTLE_FRAMES: u32 = 5;
const SETTLE_SECONDS: f64 = 0.2;
/// The player actions a scene test may press through `~system/Testing.simulateInput`: the ones
/// the on-screen joystick and buttons press.
const SIMULATED_ACTIONS: [&str; 7] = [
    "ia_forward",
    "ia_backward",
    "ia_left",
    "ia_right",
    "ia_jump",
    "ia_sprint",
    "ia_walk",
];

/// A snapshot a scene asked the test run to take and compare.
pub struct SnapshotRequest {
    pub scene_id: i32,
    pub name: String,
    pub camera_position: Vector3,
    pub camera_target: Vector3,
    pub size: Vector2,
    pub grey_pixel_diff: bool,
    pub sender: Gd<DclRpcSenderTakeAndCompareSnapshotResponse>,
}

#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct DclTestingTools {
    base: Base<Node>,
    plan: Vec<SceneTest>,
    realm_changed: bool,
    player_body: Option<Gd<Node3D>>,
    folders: Option<SnapshotFolders>,
}

#[godot_api]
impl INode for DclTestingTools {
    fn ready(&mut self) {
        self.base_mut().set_process_mode(ProcessMode::DISABLED);
        let mut this = self.to_gd();
        defer(move || this.bind_mut().start());
    }

    fn process(&mut self, _delta: f64) {
        self.step();
    }
}

impl DclTestingTools {
    fn start(&mut self) {
        let cli = globals::cli();
        if !cli.bind().scene_test_mode {
            return;
        }
        let from_cli = cli.bind().snapshot_folder.to_string();
        let folders = SnapshotFolders::resolve(&from_cli, "res://../tests/snapshots", "/snapshot");
        godot_print!("snapshot_folder=\"{}\"", folders.reference);
        godot_print!("snapshot_comparison_folder=\"{}\"", folders.comparison);
        self.folders = Some(folders);

        let argument = cli.bind().get_arg("--scene-test".into()).to_string();
        let parcels = match parse_parcels(&argument) {
            Ok((parcels, skipped)) => {
                for entry in skipped {
                    godot_error!("Scene to test '{entry}' not supported for now.");
                }
                parcels
            }
            Err(reason) => {
                godot_error!("The scenes to test could not be read ({reason}): {argument}");
                Vec::new()
            }
        };
        if parcels.is_empty() {
            godot_error!(
                "Couldn't get any scene to test in the scene-test mode. Please try --scene-test [[52,-52]]"
            );
            self.exit_gracefully(1);
            return;
        }
        godot_print!("Testing scenes: {}", parcels.len());
        self.plan = parcels.into_iter().map(SceneTest::new).collect();

        let me = self.to_gd();
        globals::realm()
            .signals()
            .realm_changed()
            .connect_other(&me, |this: &mut Self| this.on_realm_changed());
        spawn_local(realm_timeout(me));
        crate::dcl::common::set_scene_log_enabled(true);
    }

    fn on_realm_changed(&mut self) {
        self.realm_changed = true;
        self.base_mut().set_process_mode(ProcessMode::ALWAYS);
        let runner = globals::scene_runner();
        let (avatar, body) = {
            let runner = runner.bind();
            (
                runner.get_player_avatar_node(),
                runner.get_player_body_node(),
            )
        };
        self.player_body = body.clone();
        if let (Some(avatar), Some(body)) = (avatar, body) {
            let console = Callable::from_fn("scene_console", |args| {
                let [scene, level, timestamp, text] = args else {
                    return Variant::nil();
                };
                godot_print!("SCENE_LOG {scene} {level} {timestamp} {text}");
                Variant::nil()
            });
            runner
                .clone()
                .bind_mut()
                .set_player_node(avatar, body, console);
        }
        globals::scene_fetcher().bind_mut().set_scene_radius(0);
        for test in &mut self.plan {
            test.reset_timeout();
        }
    }

    /// One scene at a time: teleport to it, wait for its tests, then the next; the run ends when
    /// every scene is done.
    fn step(&mut self) {
        let Some(index) = self.plan.iter().position(|test| !test.finished) else {
            let (ok, lines) = summarize(&self.plan);
            for line in lines {
                godot_print!("{line}");
            }
            self.exit_gracefully(if ok { 0 } else { 1 });
            return;
        };
        let parcel = self.plan[index].parcel;
        let scene_id = globals::scene_fetcher()
            .bind()
            .get_parcel_scene_id(parcel.x, parcel.y);
        // The player goes to every scene: one that is already loaded may only run its tests
        // with the player on its parcel.
        if scene_id == -1 || !self.plan[index].teleported {
            self.wait_for_scene(index);
            return;
        }
        let runner = globals::scene_runner();
        if runner.bind().is_scene_tests_finished(scene_id) {
            let report =
                SceneReport::from_dictionary(&runner.bind().get_scene_tests_result(scene_id));
            let test = &mut self.plan[index];
            test.finished = true;
            test.report = Some(report);
            return;
        }
        // A scene without tests (or a different scene on this parcel) must fail the run, not
        // hang it.
        let test = &mut self.plan[index];
        if !test.seen {
            test.seen = true;
            test.reset_timeout();
        } else if test.tests_timed_out() {
            godot_error!(
                "Scene {scene_id} on ({}, {}) reported no test result in {} seconds",
                test.parcel.x,
                test.parcel.y,
                SCENE_TESTS_TIMEOUT.as_secs()
            );
            test.finished = true;
        }
    }

    fn wait_for_scene(&mut self, index: usize) {
        let test = &mut self.plan[index];
        if !test.teleported {
            test.teleported = true;
            test.reset_timeout();
            // A test that threw mid-hold must not keep walking the player into the next scene.
            release_simulated_input();
            if let Some(mut body) = self.player_body.clone().filter(|b| b.is_instance_valid()) {
                body.set_global_position(Vector3::new(
                    test.parcel.x as f32 * 16.0 + 8.0,
                    1.0,
                    -test.parcel.y as f32 * 16.0 - 8.0,
                ));
            }
        } else if (test.seen && test.tests_timed_out()) || (!test.seen && test.timed_out()) {
            godot_error!(
                "Scene test timeout after {} seconds ({}, {})",
                test.elapsed_secs(),
                test.parcel.x,
                test.parcel.y
            );
            test.finished = true;
        }
    }

    /// Ends the run with `code`.
    pub fn exit_gracefully(&self, code: i32) {
        match self.base().get_tree_or_null() {
            Some(mut tree) => tree.quit_ex().exit_code(code).done(),
            // No tree to quit: leave the process.
            None => std::process::exit(code),
        }
    }
}

async fn realm_timeout(tools: Gd<DclTestingTools>) {
    sleep(SCENE_TIMEOUT.as_secs_f64()).await;
    if tools.is_instance_valid() && !tools.bind().realm_changed {
        godot_error!("{} seconds realm changed timeout", SCENE_TIMEOUT.as_secs());
        tools.bind().exit_gracefully(1);
    }
}

/// Takes the picture a scene asked for, compares it with its stored one and answers the scene.
/// Presses (with a strength in 0..1) or releases one of the player's actions for a scene test. A
/// press with `frames` > 0 is released after exactly that many frames, here on the main thread:
/// the scene's own ticks run on another thread and would land a frame early or late.
pub fn simulate_input(action: &str, pressed: bool, strength: f32, frames: u32) {
    if !SIMULATED_ACTIONS.contains(&action) {
        tracing::warn!("simulateInput: '{action}' is not a player action");
        return;
    }
    let mut input = Input::singleton();
    if !pressed {
        input.action_release(action);
        return;
    }
    input
        .action_press_ex(action)
        .strength(strength.clamp(0.0, 1.0))
        .done();
    if frames > 0 {
        let action = action.to_string();
        spawn_local(async move {
            for _ in 0..frames {
                next_frame().await;
            }
            Input::singleton().action_release(&action);
        });
    }
}

/// First touch index of a simulated gesture, clear of the ones a real screen hands out first.
const GESTURE_TOUCH: i32 = 20;
/// Where a simulated look or pinch happens, as a fraction of the screen: the open camera area
/// right of the joystick's zone and above the action buttons.
const LOOK_POINT: (f32, f32) = (0.62, 0.35);
const PINCH_POINTS: [(f32, f32); 2] = [(0.56, 0.35), (0.72, 0.35)];

/// A named touch gesture for a scene test, played through the real mobile controls as touch
/// events: `joystick` pushes the stick toward `x,y` by `strength` and holds it `frames` frames;
/// `look` drags one finger by `x,y` pixels over `frames`; `pinch` spreads two fingers by `x`
/// pixels (negative closes them) over `frames`.
pub fn simulate_gesture(kind: &str, x: f32, y: f32, strength: f32, frames: u32) {
    let Some(ui) = globals::explorer()
        .and_then(|explorer| explorer.upcast::<Node>().get_node_or_null("UI"))
        .and_then(|node| node.try_cast::<Control>().ok())
    else {
        tracing::warn!("simulateGesture: the explorer UI is not loaded");
        return;
    };
    let size = ui.get_viewport_rect().size;
    let at = |(fx, fy): (f32, f32)| Vector2::new(size.x * fx, size.y * fy);
    let frames = frames.max(1);
    let gesture = match kind {
        "joystick" => {
            let Some(joystick) = ui.get_node_or_null("VirtualJoystick_Left") else {
                tracing::warn!("simulateGesture: no joystick in the explorer UI");
                return;
            };
            let Some(area) = joystick
                .get_node_or_null("ActiveArea")
                .and_then(|node| node.try_cast::<Control>().ok())
            else {
                return;
            };
            let reach = joystick
                .get("clampzone_size")
                .try_to::<f32>()
                .unwrap_or(75.0);
            let start = area.get_global_rect().center();
            let push = Vector2::new(x, y).normalized_or_zero() * strength.clamp(0.0, 1.0) * reach;
            Gesture {
                fingers: vec![(start, start + push)],
                move_frames: 1,
                hold_frames: frames,
            }
        }
        "look" => {
            let start = at(LOOK_POINT);
            Gesture {
                fingers: vec![(start, start + Vector2::new(x, y))],
                move_frames: frames,
                hold_frames: 0,
            }
        }
        "pinch" => {
            let [a, b] = PINCH_POINTS.map(at);
            let half = Vector2::new(x / 2.0, 0.0);
            Gesture {
                fingers: vec![(a, a - half), (b, b + half)],
                move_frames: frames,
                hold_frames: 0,
            }
        }
        _ => {
            tracing::warn!("simulateGesture: unknown gesture '{kind}'");
            return;
        }
    };
    // Touches arrive in window pixels; the viewport maps them back to the canvas the UI lives in.
    let to_window = ui.get_viewport_transform();
    spawn_local(play_gesture(gesture, to_window));
}

struct Gesture {
    /// Each finger's start and end, in canvas coordinates.
    fingers: Vec<(Vector2, Vector2)>,
    move_frames: u32,
    hold_frames: u32,
}

async fn play_gesture(gesture: Gesture, to_window: Transform2D) {
    let mut input = Input::singleton();
    let index = |finger: usize| GESTURE_TOUCH + finger as i32;
    for (finger, (start, _)) in gesture.fingers.iter().enumerate() {
        let mut touch = InputEventScreenTouch::new_gd();
        touch.set_index(index(finger));
        touch.set_position(to_window * *start);
        touch.set_pressed(true);
        input.parse_input_event(&touch);
    }
    next_frame().await;
    for step in 1..=gesture.move_frames {
        let t = step as f32 / gesture.move_frames as f32;
        let previous = (step - 1) as f32 / gesture.move_frames as f32;
        for (finger, (start, end)) in gesture.fingers.iter().enumerate() {
            let now = to_window * start.lerp(*end, t);
            let before = to_window * start.lerp(*end, previous);
            let mut drag = InputEventScreenDrag::new_gd();
            drag.set_index(index(finger));
            drag.set_position(now);
            drag.set_relative(now - before);
            drag.set_screen_relative(now - before);
            input.parse_input_event(&drag);
        }
        next_frame().await;
    }
    for _ in 0..gesture.hold_frames {
        next_frame().await;
    }
    for (finger, (_, end)) in gesture.fingers.iter().enumerate() {
        let mut touch = InputEventScreenTouch::new_gd();
        touch.set_index(index(finger));
        touch.set_position(to_window * *end);
        touch.set_pressed(false);
        input.parse_input_event(&touch);
    }
}

fn release_simulated_input() {
    let mut input = Input::singleton();
    for action in SIMULATED_ACTIONS {
        input.action_release(action);
    }
}

pub fn take_and_compare_snapshot(request: SnapshotRequest) {
    let tools = DclGlobal::singleton().bind().testing_tools.clone();
    defer(move || spawn_local(capture_and_answer(tools, request)));
}

async fn capture_and_answer(tools: Gd<DclTestingTools>, request: SnapshotRequest) {
    godot_print!(
        "async_take_and_compare_snapshot {} {} {:?} {:?} {:?}",
        request.scene_id,
        request.name,
        request.camera_position,
        request.camera_target,
        request.size
    );
    wait_for_content().await;
    let Some(image) = capture(&tools, &request).await else {
        return;
    };
    let folders = tools.bind().folders.clone();
    let mut result = VarDictionary::new();
    let Some(folders) = folders else {
        result.set("stored_snapshot_found", false);
        send(request, result);
        return;
    };
    let file = snapshot_file_name(&request.name);
    let current = format!("{}{file}", folders.comparison);
    let stored = format!("{}{file}", folders.reference);
    image.clone().save_png(current.as_str());
    let reference = FileAccess::file_exists(stored.as_str())
        .then(|| Image::load_from_file(stored.as_str()))
        .flatten();
    result.set("stored_snapshot_found", reference.is_some());
    if let (Some(reference), true) = (reference, request.grey_pixel_diff) {
        let diff_path = current.replace(".png", ".diff.png");
        let similarity = compare_images(reference, image, &diff_path);
        let mut grey = VarDictionary::new();
        grey.set("similarity", similarity);
        result.set("grey_pixel_diff", &grey);
    }
    send(request, result);
}

fn send(request: SnapshotRequest, result: VarDictionary) {
    let mut sender = request.sender;
    sender.bind_mut().send(result.to_variant());
}

/// Waits until no content promise is pending and no load has started or finished for a few
/// checks in a row. The load counters are running totals, and a failed load never counts as
/// finished, so "nothing in flight" is a counter that stopped moving, not one at zero.
async fn wait_for_content() {
    let mut stable = 0;
    let mut last = None;
    for _ in 0..LOAD_CHECKS {
        let (pending, counters) = {
            let provider = globals::content_provider();
            let provider = provider.bind();
            (
                provider.get_pending_promises(),
                (
                    provider.count_loading_resources(),
                    provider.count_loaded_resources(),
                ),
            )
        };
        if pending.is_empty() && last == Some(counters) {
            stable += 1;
            if stable >= STABLE_CHECKS {
                return;
            }
        } else {
            stable = 0;
            if !pending.is_empty() {
                await_all(pending.iter_shared().collect()).await;
            }
        }
        last = Some(counters);
        sleep(LOAD_CHECK_INTERVAL).await;
    }
    godot_warn!(
        "snapshot: content still loading after {:.0} s; capturing anyway",
        LOAD_CHECKS as f64 * LOAD_CHECK_INTERVAL
    );
}

/// The scene from the requested camera at exactly the requested size, drawn off-screen: the
/// window's own size (a phone shape, or what the display allows) never changes the picture. The
/// scene's UI is lent to the capture; the explorer's HUD and the avatars stay out of it.
async fn capture(tools: &Gd<DclTestingTools>, request: &SnapshotRequest) -> Option<Gd<Image>> {
    RenderingServer::singleton().set_default_clear_color(Color::from_rgba(0.0, 0.0, 0.0, 0.0));
    let window = tools.get_window()?;
    let mut viewport = SubViewport::new_alloc();
    viewport.set_size(Vector2i::new(request.size.x as i32, request.size.y as i32));
    viewport.set_world_3d(&window.find_world_3d()?);
    viewport.set_msaa_3d(window.get_msaa_3d());
    viewport.set_screen_space_aa(window.get_screen_space_aa());
    viewport.set_update_mode(UpdateMode::ALWAYS);
    tools.clone().add_child(&viewport);

    let mut camera = Camera3D::new_alloc();
    camera.set_cull_mask(CAMERA_CULL_MASK);
    viewport.add_child(&camera);
    camera.make_current();
    camera.set_global_position(request.camera_position);
    camera.look_at(request.camera_target);

    let lent_ui = lend_scene_ui(&viewport);
    if lent_ui.is_some() {
        relayout_scene_ui(request.scene_id, request.size);
    }
    freeze_avatars();
    for _ in 0..SETTLE_FRAMES {
        next_frame().await;
    }
    sleep(SETTLE_SECONDS).await;
    let image = viewport
        .get_texture()
        .and_then(|texture| texture.get_image());

    if let Some((mut ui, mut parent, index)) = lent_ui {
        ui.reparent(&parent);
        // Input picks scene UI by its place among its siblings.
        parent.move_child(&ui, index);
    }
    tools.clone().remove_child(&viewport);
    viewport.queue_free();
    image
}

/// Moves the scene's UI container into the capture viewport, where it fills the capture's size,
/// and returns where it came from.
fn lend_scene_ui(viewport: &Gd<SubViewport>) -> Option<(Gd<Control>, Gd<Node>, i32)> {
    let world = crate::world::state::current()?;
    let mut ui = world.scene_ui.clone();
    let parent = ui.get_parent()?;
    let index = ui.get_index();
    ui.reparent(viewport);
    Some((ui, parent, index))
}

/// The scene's UI laid out for the capture's size: percentages and centering follow it.
fn relayout_scene_ui(scene_id: i32, size: Vector2) {
    let canvas = PbUiCanvasInformation {
        width: size.x as i32,
        height: size.y as i32,
        device_pixel_ratio: 1.0,
        ..Default::default()
    };
    let mut runner = globals::scene_runner();
    let mut runner = runner.bind_mut();
    if let Some(scene) = runner.get_scene_mut(&SceneId(scene_id)) {
        relayout_for_capture(scene, &canvas);
    }
}

/// Still and hidden: the remote avatars and the player's.
fn freeze_avatars() {
    for child in globals::avatars_scene().get_children().iter_shared() {
        if let Ok(mut avatar) = child.try_cast::<DclAvatar>() {
            avatar.hide();
            avatar.bind_mut().freeze_on_idle();
        }
    }
    if let Some(player) = globals::scene_runner().bind().get_player_avatar_node() {
        if let Ok(mut avatar) = player.try_cast::<DclAvatar>() {
            avatar.bind_mut().freeze_on_idle();
            avatar.hide();
        }
    }
}

/// How alike two captures are (RGB), with the difference image saved at `diff_path`.
pub fn compare_images(mut a: Gd<Image>, mut b: Gd<Image>, diff_path: &str) -> f64 {
    use godot::classes::image::Format;
    if a.get_size() != b.get_size() {
        godot_print!("compute_image_similarity have different sizes");
        return 0.0;
    }
    for image in [&mut a, &mut b] {
        if image.get_format() != Format::RGB8 {
            image.convert(Format::RGB8);
        }
    }
    let (data_a, data_b) = (a.get_data(), b.get_data());
    if !diff_path.is_empty() {
        let diff = PackedByteArray::from(rgb_difference(data_a.as_slice(), data_b.as_slice()));
        if let Some(image) =
            Image::create_from_data(a.get_width(), a.get_height(), false, Format::RGB8, &diff)
        {
            image.save_png(diff_path);
        }
    }
    rgb_similarity(data_a.as_slice(), data_b.as_slice())
}

/// Makes sure the folder of `path` exists.
pub fn ensure_parent_dir(path: &str) {
    if let Some((dir, _)) = path.rsplit_once('/') {
        if !dir.is_empty() && !DirAccess::dir_exists_absolute(dir) {
            DirAccess::make_dir_recursive_absolute(dir);
        }
    }
}
