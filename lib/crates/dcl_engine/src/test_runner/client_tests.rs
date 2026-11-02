//! The client visual tests (`--client-test`): the avatar with and without its outline, the bloom of
//! an unlit surface and the `uiBackground` wrap mode, each rendered off-screen and compared with
//! the stored snapshot. A missing reference is written, not failed.

use godot::classes::environment::{AmbientSource, BgMode};
use godot::classes::sub_viewport::UpdateMode;
use godot::classes::{
    base_material_3d::ShadingMode, Camera3D, Environment, FileAccess, INode3D, Image,
    MeshInstance3D, Node3D, PackedScene, QuadMesh, RenderingServer, StandardMaterial3D,
    SubViewport, WorldEnvironment,
};
use godot::prelude::*;

use super::image_compare::matching_pixels;
use super::snapshot_files::SnapshotFolders;
use super::ui_background_test_probe::DclUiBackgroundTestProbe;
use crate::avatars::avatar::build::update_avatar;
use crate::avatars::avatar_type::DclAvatarWireFormat;
use crate::avatars::dcl_user_profile::DclUserProfile;
use crate::avatars::preview::AvatarPreview;
use crate::engine::local_task::{next_frame, sleep, spawn_local};
use crate::fetcher::globals;
use crate::godot_classes::dcl_global::DclGlobal;

const PREVIEW_SCENE: &str = "res://src/ui/pages/backpack/avatar_preview.tscn";
const BASE_AVATARS: &str = "urn:decentraland:off-chain:base-avatars:";
/// Share of matching pixels a capture needs to pass.
const PASS_SIMILARITY: f32 = 0.99;
/// How different two pixels may be (the four channels summed, 0..1 each) and still match.
const PIXEL_TOLERANCE: f32 = 0.01;

struct TestResult {
    name: String,
    stored_found: bool,
    similarity: f32,
    passed: bool,
}

#[derive(GodotClass)]
#[class(init, base=Node3D)]
pub struct ClientTestScene {
    base: Base<Node3D>,
}

#[godot_api]
impl INode3D for ClientTestScene {
    fn ready(&mut self) {
        godot_print!("🔧 Client Test Mode - Avatar Snapshot Testing");
        let from_cli = globals::cli().bind().snapshot_folder.to_string();
        let folders = SnapshotFolders::resolve(
            &from_cli,
            "res://../tests/snapshots/client/",
            "/snapshots/client/",
        );
        godot_print!("snapshot_folder=\"{}\"", folders.reference);
        godot_print!("snapshot_comparison_folder=\"{}\"", folders.comparison);
        crate::dcl::common::set_scene_log_enabled(false);
        globals::scene_runner().bind_mut().set_pause(true);

        let mut preview = load::<PackedScene>(PREVIEW_SCENE).instantiate_as::<AvatarPreview>();
        {
            let mut p = preview.bind_mut();
            p.hide_name = true;
            p.can_move = false;
            p.with_light = false;
        }
        self.base_mut().add_child(&preview);
        // Quality over speed: nothing here is realtime.
        if let Some(mut viewport) = preview.bind().subviewport.clone() {
            viewport.set_use_debanding(true);
            viewport.set_scaling_3d_scale(2.0);
        }
        RenderingServer::singleton().screen_space_roughness_limiter_set_active(true, 4.0, 1.0);
        let me = self.to_gd();
        spawn_local(run(me, preview, folders));
    }
}

async fn run(scene: Gd<ClientTestScene>, preview: Gd<AvatarPreview>, folders: SnapshotFolders) {
    sleep(1.0).await;
    let mut logs = Vec::new();
    let mut results = Vec::new();
    load_test_avatar(&preview).await;
    sleep(2.0).await;

    let viewport = preview.bind().subviewport.clone();
    if let Some(viewport) = viewport {
        results.push(compare_capture("avatar_no_outline", &viewport, &folders, &mut logs).await);
        preview.clone().bind_mut().enable_outline();
        godot_print!("Outline enabled!");
        sleep(1.0).await;
        results.push(compare_capture("avatar_with_outline", &viewport, &folders, &mut logs).await);
    }
    results.push(bloom_unlit_test(&scene, &folders, &mut logs).await);
    results.push(ui_background_repeat_test(&scene, &folders, &mut logs).await);

    for line in logs {
        godot_print!("{line}");
    }
    let ok = report(&results);
    let tools = DclGlobal::singleton().bind().testing_tools.clone();
    tools.bind().exit_gracefully(if ok { 0 } else { 1 });
}

/// A fixed avatar, so the snapshots do not depend on the account.
async fn load_test_avatar(preview: &Gd<AvatarPreview>) {
    godot_print!("Loading test avatar...");
    let profile = DclUserProfile::new_gd();
    let mut wire = profile.bind().get_avatar();
    {
        let mut avatar: godot::obj::GdMut<DclAvatarWireFormat> = wire.bind_mut();
        avatar.set_body_shape(format!("{BASE_AVATARS}BaseFemale").as_str().into());
        avatar.set_skin_color(Color::from_rgb(0.490, 0.365, 0.278));
        avatar.set_hair_color(Color::from_rgb(0.596, 0.373, 0.216));
        avatar.set_eyes_color(Color::from_rgb(0.3, 0.8, 0.5));
        let wearables: PackedStringArray = [
            "f_sweater",
            "f_jeans",
            "bun_shoes",
            "standard_hair",
            "f_eyes_01",
            "f_eyebrows_00",
        ]
        .iter()
        .map(|name| GString::from(format!("{BASE_AVATARS}{name}").as_str()))
        .collect();
        avatar.set_wearables(wearables);
    }
    let Some(mut avatar) = preview.bind().avatar.clone() else {
        return;
    };
    update_avatar(avatar.clone(), wire, String::new(), None).await;
    godot_print!("Avatar updated!");
    // Without animation the snapshots are the same every run.
    avatar.bind_mut().freeze_on_idle();
    let mut rotation = avatar.get_rotation();
    rotation.y = 0.0;
    avatar.set_rotation(rotation);
}

async fn frames_after_draw() {
    let _ = RenderingServer::singleton()
        .signals()
        .frame_post_draw()
        .to_fallible_future()
        .await;
    next_frame().await;
    next_frame().await;
}

async fn compare_capture(
    name: &str,
    viewport: &Gd<SubViewport>,
    folders: &SnapshotFolders,
    logs: &mut Vec<String>,
) -> TestResult {
    godot_print!("Testing {name}...");
    frames_after_draw().await;
    let image = viewport.get_texture().and_then(|t| t.get_image());
    compare_image(name, image, folders, logs)
}

async fn bloom_unlit_test(
    scene: &Gd<ClientTestScene>,
    folders: &SnapshotFolders,
    logs: &mut Vec<String>,
) -> TestResult {
    godot_print!("Testing bloom_unlit_no_halo...");
    let mut viewport = SubViewport::new_alloc();
    viewport.set_use_own_world_3d(true);
    viewport.set_transparent_background(false);
    viewport.set_size(Vector2i::new(256, 256));
    viewport.set_update_mode(UpdateMode::ALWAYS);
    scene.clone().add_child(&viewport);

    // A flat color instead of the game's sky, so the snapshot is the same on every machine.
    let mut environment = Environment::new_gd();
    environment.set_background(BgMode::COLOR);
    environment.set_bg_color(Color::from_rgb(0.05, 0.05, 0.05));
    environment.set_ambient_source(AmbientSource::COLOR);
    environment.set_ambient_light_color(Color::WHITE);
    environment.set_ambient_light_energy(0.3);
    environment.set_glow_enabled(true);
    environment.set_glow_intensity(1.5);
    environment.set_glow_hdr_bleed_threshold(1.0);
    environment.set_glow_hdr_bleed_scale(2.0);
    for (level, value) in [(1, 0.4), (2, 0.3), (3, 0.0), (5, 0.0)] {
        environment.set_glow_level(level, value);
    }
    let mut world_environment = WorldEnvironment::new_alloc();
    world_environment.set_environment(&environment);
    viewport.add_child(&world_environment);

    let mut camera = Camera3D::new_alloc();
    viewport.add_child(&camera);
    camera.set_position(Vector3::new(0.0, 0.0, 2.0));
    camera.look_at(Vector3::ZERO);

    // A white unlit quad, like an SDK billboard or an avatar's name.
    let mut quad = QuadMesh::new_gd();
    quad.set_size(Vector2::new(1.0, 1.0));
    let mut material = StandardMaterial3D::new_gd();
    material.set_shading_mode(ShadingMode::UNSHADED);
    material.set_albedo(Color::WHITE);
    let mut mesh = MeshInstance3D::new_alloc();
    mesh.set_mesh(&quad);
    mesh.set_material_override(&material);
    viewport.add_child(&mesh);

    sleep(0.5).await;
    frames_after_draw().await;
    let image = viewport.get_texture().and_then(|t| t.get_image());
    viewport.queue_free();
    compare_image("bloom_unlit_no_halo", image, folders, logs)
}

async fn ui_background_repeat_test(
    scene: &Gd<ClientTestScene>,
    folders: &SnapshotFolders,
    logs: &mut Vec<String>,
) -> TestResult {
    godot_print!("Testing uibackground_repeat_stretch...");
    // 1.5 atlas widths wide: the tiled result is 1:1 with the atlas texels and the wrapped region
    // is a full half-tile.
    let size = Vector2i::new(192, 64);
    let mut viewport = SubViewport::new_alloc();
    viewport.set_size(size);
    viewport.set_update_mode(UpdateMode::ALWAYS);
    viewport.set_transparent_background(false);
    viewport.set_disable_3d(true);
    scene.clone().add_child(&viewport);
    let mut probe = DclUiBackgroundTestProbe::new_alloc();
    viewport.add_child(&probe);
    probe
        .bind_mut()
        .build_repeat_background(Vector2::new(size.x as f32, size.y as f32));

    // The texture resolves through a deferred call: the waits let the real load path run first.
    sleep(0.5).await;
    frames_after_draw().await;
    let image = viewport.get_texture().and_then(|t| t.get_image());
    viewport.queue_free();
    compare_image("uibackground_repeat_stretch", image, folders, logs)
}

/// Saves the capture, compares it with the stored one and writes it as the reference when there is
/// none.
fn compare_image(
    name: &str,
    image: Option<Gd<Image>>,
    folders: &SnapshotFolders,
    logs: &mut Vec<String>,
) -> TestResult {
    let mut result = TestResult {
        name: name.to_string(),
        stored_found: false,
        similarity: 0.0,
        passed: false,
    };
    let Some(mut captured) = image else {
        logs.push(format!("🔴 {name}: no image captured"));
        return result;
    };
    let current = format!("{}{name}.png", folders.comparison);
    let stored = format!("{}{name}.png", folders.reference);
    let diff_path = format!("{}{name}.diff.png", folders.comparison);
    captured.save_png(current.as_str());
    godot_print!("Current snapshot saved to: {current}");
    if !FileAccess::file_exists(stored.as_str()) {
        logs.push(format!(
            "⚠️ {name}: No reference snapshot found at {stored}"
        ));
        logs.push("   Creating initial snapshot...".to_string());
        captured.save_png(stored.as_str());
        return result;
    }
    result.stored_found = true;
    let Some(mut reference) = Image::load_from_file(stored.as_str()) else {
        logs.push(format!("🔴 {name}: the stored snapshot cannot be read"));
        return result;
    };
    if let Some(similarity) = similarity(&mut reference, &mut captured, &diff_path) {
        result.similarity = similarity;
        result.passed = similarity > PASS_SIMILARITY;
    } else {
        godot_print!("Images have different sizes!");
    }
    let percent = result.similarity * 100.0;
    if result.passed {
        logs.push(format!("🟢 {name}: PASSED (similarity: {percent:.2}%)"));
    } else {
        logs.push(format!("🔴 {name}: FAILED (similarity: {percent:.2}%)"));
        logs.push(format!("   Diff saved to: {diff_path}"));
    }
    result
}

/// The share of matching pixels, with the difference image saved; `None` for different sizes.
fn similarity(a: &mut Gd<Image>, b: &mut Gd<Image>, diff_path: &str) -> Option<f32> {
    use godot::classes::image::Format;
    if a.get_size() != b.get_size() {
        return None;
    }
    for image in [&mut *a, &mut *b] {
        if image.get_format() != Format::RGBA8 {
            image.convert(Format::RGBA8);
        }
    }
    let (share, diff) = matching_pixels(
        a.get_data().as_slice(),
        b.get_data().as_slice(),
        PIXEL_TOLERANCE,
    )?;
    let diff = PackedByteArray::from(diff);
    if let Some(image) =
        Image::create_from_data(a.get_width(), a.get_height(), false, Format::RGBA8, &diff)
    {
        image.save_png(diff_path);
    }
    Some(share)
}

/// Prints the summary; true when no comparison failed.
fn report(results: &[TestResult]) -> bool {
    let failed: Vec<&TestResult> = results
        .iter()
        .filter(|r| r.stored_found && !r.passed)
        .collect();
    let passed = results
        .iter()
        .filter(|r| r.stored_found && r.passed)
        .count();
    let skipped = results.iter().filter(|r| !r.stored_found).count();
    godot_print!(
        "🔧 Client Tests: {} total, {passed} passed, {} failed, {skipped} skipped",
        results.len(),
        failed.len()
    );
    if failed.is_empty() {
        godot_print!("All client tests passed!");
        return true;
    }
    let mut detail = String::from("❌ Client test failures:\n");
    for result in &failed {
        detail.push_str(&format!(
            "  - {}: {:.2}% similarity\n",
            result.name,
            result.similarity * 100.0
        ));
    }
    godot_print!("{detail}");
    godot_print!("Some client tests failed");
    false
}
