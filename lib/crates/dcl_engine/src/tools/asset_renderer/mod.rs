//! The asset renderer (`--asset-renderer`): renders wearables and emotes to PNGs from the cameras an
//! input file asks for, and writes a report of what came out. It runs as a service, so a bad item
//! is reported and the batch goes on; a hung one ends the batch.

mod camera;
pub mod input;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use godot::classes::viewport::Msaa;
use godot::classes::{camera_3d::ProjectionType, Camera3D, FileAccess, INode, Image, Json, Node};
use godot::prelude::*;
use serde_json::{json, Map, Value};

use self::camera::{place, FitBounds};
use self::input::{AssetInput, AssetItem, Shot};
use super::offscreen::{self, Quality};
use crate::avatars::avatar::build::update_avatar;
use crate::avatars::avatar::emote_load::async_freeze_on_emote;
use crate::avatars::avatar_type::DclAvatarWireFormat;
use crate::avatars::preview::AvatarPreview;
use crate::engine::local_task::{await_promise, next_frame, spawn_local};
use crate::fetcher::globals;
use crate::godot_classes::dcl_avatar::DclAvatar;
use crate::godot_classes::dcl_global::DclGlobal;
use crate::test_runner::testing_tools::ensure_parent_dir;

const TEST_INPUT: &str = "res://../tests/asset-renderer-test-input.json";
/// Per item: a hung content fetch or load marks the item errored and the batch ends (the caller
/// relies on the report).
const ITEM_TIMEOUT_MS: u64 = 60_000;
/// The camera layers without the outline layer (20).
const CAMERA_CULL_MASK: u32 = 524287;
const BASE_AVATARS: &str = "urn:decentraland:off-chain:base-avatars:";
/// Standalone items show no body, so a flat grey keeps stray pixels neutral.
const NEUTRAL_COLOR: f64 = 0.35;
/// With the body visible, the engine's default tones: a grey-skinned figure is what a consumer
/// would end up describing.
const NATURAL_SKIN: [f64; 3] = [1.0, 0.867, 0.737];
const NATURAL_HAIR: [f64; 3] = [0.596, 0.372, 0.215];
const NATURAL_EYES: [f64; 3] = [0.3, 0.22, 0.99];
/// Anything rendered on a body gets a dressed avatar: the missing-category fallbacks only restore
/// body meshes, so a bare wearable list would be described as underwear.
const DEFAULT_FEMALE_OUTFIT: [&str; 7] = [
    "f_sweater",
    "f_jeans",
    "bun_shoes",
    "standard_hair",
    "f_eyes_01",
    "f_eyebrows_00",
    "f_mouth_00",
];
const DEFAULT_MALE_OUTFIT: [&str; 7] = [
    "red_tshirt",
    "comfortablepants",
    "ruby_red_loafer",
    "tall_front_01",
    "eyes_00",
    "eyebrows_06",
    "mouth_01",
];

#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct AssetRenderer {
    base: Base<Node>,
}

#[godot_api]
impl INode for AssetRenderer {
    fn ready(&mut self) {
        godot_print!("spawning asset renderer scene");
        let Some(input) = read_input() else {
            godot_error!("param is missing or wrong, try with --asset-input-file [file]");
            quit(1);
            return;
        };
        if input.items.is_empty() && input.invalid_items.is_empty() {
            godot_error!("no assets to process");
            quit(2);
            return;
        }
        globals::scene_runner().bind_mut().set_pause(true);
        globals::realm().bind_mut().content_base_url = input.base_url.as_str().into();
        let Some(mut preview) = self
            .base()
            .try_get_node_as::<AvatarPreview>("%AvatarPreview")
        else {
            godot_error!("the asset renderer scene has no preview");
            quit(1);
            return;
        };
        // The preview's process lerps the camera toward UI targets; the tool drives it directly.
        preview.set_process(false);
        preview.bind_mut().fit_avatar = false;
        godot::classes::RenderingServer::singleton()
            .set_default_clear_color(Color::from_rgba(0.0, 0.0, 0.0, 0.0));
        // Like the avatar renderer, except the supersampling is the caller's (it defaults to 1.0
        // because software rendering pays 4x the raster cost at 2.0).
        offscreen::apply(
            &preview,
            Quality {
                msaa: Msaa::MSAA_4X,
                scaling_3d: (input.supersample as f32).clamp(0.5, 2.0),
            },
        );
        spawn_local(run(preview, input));
    }
}

fn quit(code: i32) {
    if let Some(mut tree) = DclGlobal::singleton().get_tree_or_null() {
        tree.quit_ex().exit_code(code).done();
    }
}

fn read_input() -> Option<AssetInput> {
    let cli = globals::cli();
    let path = if cli.bind().use_test_input {
        TEST_INPUT.to_string()
    } else {
        cli.bind().asset_input_file.to_string()
    };
    if path.is_empty() {
        return None;
    }
    let file = FileAccess::open(path.as_str(), godot::classes::file_access::ModeFlags::READ)?;
    match AssetInput::parse(&file.get_as_text().to_string()) {
        Ok(input) => Some(input),
        Err(reason) => {
            godot_error!("{reason}");
            None
        }
    }
}

#[derive(Clone, Debug)]
struct ItemResult {
    id: String,
    error: Option<String>,
    files: Vec<String>,
}

impl ItemResult {
    fn failed(id: &str, error: &str, files: Vec<String>) -> Self {
        Self {
            id: id.to_string(),
            error: Some(error.to_string()),
            files,
        }
    }

    fn to_json(&self) -> Value {
        match &self.error {
            None => json!({"id": self.id, "status": "ok", "files": self.files}),
            Some(error) => {
                json!({"id": self.id, "status": "error", "error": error, "files": self.files})
            }
        }
    }
}

/// The bookkeeping that keeps a timed-out item's detached future from committing into a later
/// item's slot: it cannot be cancelled, only ignored.
#[derive(Default)]
struct Guard {
    generation: Cell<u64>,
    pending: RefCell<Option<ItemResult>>,
}

impl Guard {
    fn commit(&self, generation: u64, result: ItemResult) {
        if generation == self.generation.get() {
            *self.pending.borrow_mut() = Some(result);
        }
    }
}

async fn run(preview: Gd<AvatarPreview>, input: AssetInput) {
    if let Some(mut camera) = preview.bind().camera_3d.clone() {
        camera.set_as_top_level(true);
        camera.set_cull_mask(CAMERA_CULL_MASK);
    }
    let mut results: Vec<ItemResult> = input
        .invalid_items
        .iter()
        .map(|item| ItemResult::failed(&item.id, &item.error, Vec::new()))
        .collect();
    let guard = Rc::new(Guard::default());
    // A timed-out item's future keeps running and rebuilds the shared skeleton, which would corrupt
    // a later capture and still report "ok". It cannot be cancelled, so a timeout ends the batch.
    let mut timed_out = false;
    for item in input.items {
        if timed_out {
            results.push(ItemResult::failed(
                &item.id,
                "skipped after a timeout",
                Vec::new(),
            ));
            continue;
        }
        let result = process_guarded(&guard, &preview, item.clone()).await;
        let icon = if result.error.is_none() {
            "🟢"
        } else {
            "🔴"
        };
        godot_print!(
            "{icon} {} {}",
            item.id,
            result.error.clone().unwrap_or_default()
        );
        timed_out = result.error.as_deref() == Some("timeout");
        if timed_out {
            godot_error!("aborting the batch: a detached item can corrupt later renders");
        } else if let Some(mut avatar) = preview.bind().avatar.clone() {
            // Nothing reloads the wire format between identical emote payloads, so emotes, props
            // and animations would pile up over the batch.
            avatar.bind_mut().clean_unused_emotes();
        }
        results.push(result);
    }
    write_report(&input.output_json_path, &results);
    let all_ok = results.iter().all(|r| r.error.is_none());
    DclGlobal::singleton()
        .bind()
        .testing_tools
        .bind()
        .exit_gracefully(if all_ok { 0 } else { 3 });
}

async fn process_guarded(
    guard: &Rc<Guard>,
    preview: &Gd<AvatarPreview>,
    item: AssetItem,
) -> ItemResult {
    let generation = guard.generation.get() + 1;
    guard.generation.set(generation);
    *guard.pending.borrow_mut() = None;
    spawn_local(process_item(
        guard.clone(),
        preview.clone(),
        item.clone(),
        generation,
    ));

    let deadline = godot::classes::Time::singleton().get_ticks_msec() + ITEM_TIMEOUT_MS;
    while guard.pending.borrow().is_none()
        && godot::classes::Time::singleton().get_ticks_msec() < deadline
    {
        next_frame().await;
    }
    let result = guard.pending.borrow_mut().take();
    result.unwrap_or_else(|| ItemResult::failed(&item.id, "timeout", Vec::new()))
}

fn outfit_for(body_shape: &str) -> Vec<String> {
    let outfit = if body_shape.contains("BaseMale") {
        &DEFAULT_MALE_OUTFIT
    } else {
        &DEFAULT_FEMALE_OUTFIT
    };
    outfit
        .iter()
        .map(|name| format!("{BASE_AVATARS}{name}"))
        .collect()
}

fn color(rgb: [f64; 3]) -> Value {
    json!({"color": {"r": rgb[0], "g": rgb[1], "b": rgb[2]}})
}

/// The avatar a shot is rendered on, as the wire format's JSON.
fn avatar_json(item: &AssetItem) -> Value {
    let neutral = color([NEUTRAL_COLOR; 3]);
    let mut avatar = Map::new();
    avatar.insert("bodyShape".into(), json!(item.body_shape));
    avatar.insert("eyes".into(), neutral.clone());
    avatar.insert("hair".into(), neutral.clone());
    avatar.insert("skin".into(), neutral);
    avatar.insert("wearables".into(), json!([]));
    avatar.insert("emotes".into(), json!([]));
    let dressed = |avatar: &mut Map<String, Value>, wearables: Vec<String>| {
        avatar.insert("wearables".into(), json!(wearables));
        avatar.insert("skin".into(), color(NATURAL_SKIN));
        avatar.insert("hair".into(), color(NATURAL_HAIR));
        avatar.insert("eyes".into(), color(NATURAL_EYES));
        for (key, value) in &item.avatar_overrides {
            avatar.insert(key.clone(), value.clone());
        }
    };
    match item.kind.as_str() {
        "wearable_standalone" => {
            avatar.insert("wearables".into(), json!([item.urn]));
            // Hides every body part: the only visible pixels are the item's.
            avatar.insert("showOnlyWearables".into(), json!(true));
        }
        "wearable_on_avatar" => {
            // The target first: the loader keeps the first entry per category, so the default
            // outfit only fills what the item leaves bare.
            let mut wearables = vec![item.urn.clone()];
            wearables.extend(outfit_for(&item.body_shape));
            dressed(&mut avatar, wearables);
        }
        _ => dressed(&mut avatar, outfit_for(&item.body_shape)),
    }
    Value::Object(avatar)
}

async fn process_item(
    guard: Rc<Guard>,
    preview: Gd<AvatarPreview>,
    item: AssetItem,
    generation: u64,
) {
    let outcome = render_item(&guard, &preview, &item, generation).await;
    if let Some(result) = outcome {
        guard.commit(generation, result);
    }
}

/// The item's shots; `None` when the item was abandoned (a newer item owns the preview now).
async fn render_item(
    guard: &Guard,
    preview: &Gd<AvatarPreview>,
    item: &AssetItem,
    generation: u64,
) -> Option<ItemResult> {
    let fail = |error: &str, files: Vec<String>| Some(ItemResult::failed(&item.id, error, files));
    let dictionary = Json::parse_string(avatar_json(item).to_string().as_str())
        .try_to::<VarDictionary>()
        .ok()?;
    let wire = DclAvatarWireFormat::from_godot_dictionary(dictionary);

    // A dictionary that fails to parse falls back to a default avatar, which would render a generic
    // mannequin as if it were the asset. Skipped when the caller supplied its own wearables, which
    // legitimately may not list the target urn.
    let expects_urn = item.kind != "emote" && !item.avatar_overrides.contains_key("wearables");
    let listed = wire
        .bind()
        .get_wearables()
        .as_slice()
        .iter()
        .any(|w| w.to_string() == item.urn);
    if expects_urn && !listed {
        return fail(
            "avatar payload did not parse (check the avatar overrides)",
            Vec::new(),
        );
    }

    let mut avatar: Gd<DclAvatar> = preview.bind().avatar.clone()?;
    update_avatar(avatar.clone(), wire, String::new(), None).await;
    if generation != guard.generation.get() {
        return None;
    }
    if item.kind != "emote" {
        avatar.bind_mut().freeze_on_idle();
    }
    let mut rotation = avatar.get_rotation();
    rotation.y = 0.0;
    avatar.set_rotation(rotation);
    next_frame().await;

    if item.kind == "wearable_standalone"
        && is_fallback_bounds(preview.bind().compute_current_fit_aabb())
    {
        return fail("wearable has no visible mesh", Vec::new());
    }

    let mut files = Vec::new();
    for shot in &item.shots {
        // A timed-out item keeps running detached: stop before it resizes the shared viewport
        // underneath the item being captured now.
        if generation != guard.generation.get() {
            return None;
        }
        if item.kind == "emote" {
            let frozen = async_freeze_on_emote(avatar.clone(), &item.urn, shot.at_time).await;
            if !frozen {
                return fail("emote could not be posed", files);
            }
            next_frame().await;
        }
        place_camera(preview, shot);
        let path = with_png_extension(&shot.dest_path);
        ensure_parent_dir(&path);
        let size = Vector2i::new(shot.width as i32, shot.height as i32);
        let promise = preview
            .clone()
            .bind_mut()
            .async_capture_current_view(size, 1);
        let Ok(image) = await_promise(promise).await.try_to::<Gd<Image>>() else {
            return fail("viewport produced no image", files);
        };
        if image.save_png(path.as_str()) != godot::global::Error::OK {
            return fail(&format!("could not save {path}"), files);
        }
        files.push(path);
    }
    Some(ItemResult {
        id: item.id.clone(),
        error: None,
        files,
    })
}

/// The preview's answer when there is nothing to measure.
fn is_fallback_bounds(aabb: Aabb) -> bool {
    aabb == Aabb::new(Vector3::new(-1.0, 0.0, -1.0), Vector3::new(2.0, 2.0, 2.0))
}

fn with_png_extension(path: &str) -> String {
    if path.ends_with(".png") {
        path.to_string()
    } else {
        format!("{path}.png")
    }
}

fn place_camera(preview: &Gd<AvatarPreview>, shot: &Shot) {
    let (camera, avatar, bounds) = {
        let preview = preview.bind();
        (
            preview.camera_3d.clone(),
            preview.avatar.clone(),
            preview.compute_current_fit_aabb(),
        )
    };
    let (Some(mut camera), Some(avatar)) = (camera, avatar) else {
        return;
    };
    let transform = avatar.get_global_transform();
    let fit = FitBounds {
        center: transform * bounds.center(),
        diagonal: bounds.size.length(),
    };
    let aspect = shot.width as f32 / shot.height.max(1) as f32;
    let placement = place(&shot.camera, aspect, fit);
    camera.set_projection(if placement.orthographic {
        ProjectionType::ORTHOGONAL
    } else {
        ProjectionType::PERSPECTIVE
    });
    camera.set_fov(placement.fov_degrees);
    camera.set_size(placement.size);
    look_from(&mut camera, placement.from, placement.target);
}

fn look_from(camera: &mut Gd<Camera3D>, from: Vector3, target: Vector3) {
    let mut up = Vector3::UP;
    if up.cross(target - from).is_zero_approx() {
        up = Vector3::FORWARD;
    }
    camera.set_global_position(from);
    camera.look_at_ex(target).up(up).done();
}

fn write_report(path: &str, results: &[ItemResult]) {
    ensure_parent_dir(path);
    let report = json!({"version": 1, "results": results.iter().map(ItemResult::to_json).collect::<Vec<_>>()});
    let Some(mut file) = FileAccess::open(path, godot::classes::file_access::ModeFlags::WRITE)
    else {
        godot_error!("could not write report to {path}");
        return;
    };
    file.store_string(
        serde_json::to_string_pretty(&report)
            .unwrap_or_default()
            .as_str(),
    );
    godot_print!("report written to {path}");
}
