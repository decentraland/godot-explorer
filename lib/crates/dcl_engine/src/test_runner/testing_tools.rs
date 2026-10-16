//! The scene tests: the explorer visits each parcel of `--scene-test`, waits for the scene's own
//! test plan to finish and answers the snapshot comparisons the scenes ask for. Inactive without
//! `--scene-test`.

use godot::classes::node::ProcessMode;
use godot::classes::{Camera3D, DirAccess, FileAccess, Image, Node, Node3D, RenderingServer};
use godot::prelude::*;

use super::image_compare::{rgb_difference, rgb_similarity};
use super::scene_plan::{parse_parcels, summarize, SceneReport, SceneTest, SCENE_TIMEOUT};
use super::snapshot_files::{snapshot_file_name, SnapshotFolders};
use crate::engine::local_task::{await_all, defer, next_frame, sleep, spawn_local};
use crate::fetcher::globals;
use crate::godot_classes::dcl_avatar::DclAvatar;
use crate::godot_classes::dcl_global::DclGlobal;
use crate::godot_classes::rpc_sender::take_and_compare_snapshot_response::DclRpcSenderTakeAndCompareSnapshotResponse;

/// The camera layers that show the world without the outline layer (20).
const CAMERA_CULL_MASK: u32 = 524287;
/// How often the content queue is checked before a capture, and how many times in a row it has
/// to be empty.
const LOAD_CHECKS: u32 = 100;
const STABLE_CHECKS: u32 = 3;
const LOAD_CHECK_INTERVAL: f64 = 0.1;
const SETTLE_FRAMES: u32 = 5;
const SETTLE_SECONDS: f64 = 0.2;

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
        if scene_id == -1 {
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
        }
    }

    fn wait_for_scene(&mut self, index: usize) {
        let test = &mut self.plan[index];
        if !test.teleported {
            test.teleported = true;
            test.reset_timeout();
            if let Some(mut body) = self.player_body.clone().filter(|b| b.is_instance_valid()) {
                body.set_global_position(Vector3::new(
                    test.parcel.x as f32 * 16.0 + 8.0,
                    1.0,
                    -test.parcel.y as f32 * 16.0 - 8.0,
                ));
            }
        } else if test.timed_out() {
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

/// Waits until the content queue has been empty for a few checks in a row.
async fn wait_for_content() {
    let mut stable = 0;
    for _ in 0..LOAD_CHECKS {
        let (pending, loading) = {
            let provider = globals::content_provider();
            let provider = provider.bind();
            (
                provider.get_pending_promises(),
                provider.count_loading_resources(),
            )
        };
        if pending.is_empty() && loading == 0 {
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
        sleep(LOAD_CHECK_INTERVAL).await;
    }
}

/// The scene from the requested camera, alone: the world UI and the avatars hidden.
async fn capture(tools: &Gd<DclTestingTools>, request: &SnapshotRequest) -> Option<Gd<Image>> {
    RenderingServer::singleton().set_default_clear_color(Color::from_rgba(0.0, 0.0, 0.0, 0.0));
    let mut window = tools.get_window()?;
    let previous_camera = window.get_camera_3d();
    let previous_size = window.get_size();

    let mut camera = Camera3D::new_alloc();
    camera.set_cull_mask(CAMERA_CULL_MASK);
    tools.clone().add_child(&camera);
    camera.make_current();
    window.set_size(Vector2i::new(request.size.x as i32, request.size.y as i32));
    camera.set_global_position(request.camera_position);
    camera.look_at(request.camera_target);

    crate::world::state::current().inspect(|w| w.ui_root.clone().set_visible(false));
    freeze_avatars();
    for _ in 0..SETTLE_FRAMES {
        next_frame().await;
    }
    sleep(SETTLE_SECONDS).await;
    let image = window.get_texture().and_then(|texture| texture.get_image());

    crate::world::state::current().inspect(|w| w.ui_root.clone().set_visible(true));
    window.set_size(previous_size);
    if let Some(mut previous) = previous_camera {
        previous.make_current();
    }
    tools.clone().remove_child(&camera);
    camera.queue_free();
    image
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
