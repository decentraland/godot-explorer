//! The Genesis Plaza profiling benchmark (issue #1862). `--gp-benchmark` on the desktop, or the
//! `gp-benchmark=true` deeplink parameter on a phone, starts it; `bench/genesis_plaza.config.json`
//! holds the knobs and the deeplink overrides them. The realm and the location come from
//! `--realm`/`--location` or the preview/realm/position parameters.
//!
//! The flow: wait for the explorer, wait for the scene runner's genuine loading complete, let the
//! glTF containers settle, warm up, sample for a while, then write the JSON result and dump a
//! summary line by line to stdout, where it surfaces in `adb logcat`.

mod config;
mod summary;

use std::collections::HashMap;

use godot::classes::node::ProcessMode;
use godot::classes::performance::Monitor;
use godot::classes::rendering_server::{ViewportRenderInfo, ViewportRenderInfoType};
use godot::classes::viewport::DebugDraw;
use godot::classes::{
    DirAccess, DisplayServer, Engine, FileAccess, INode, Node, Os, Performance, RenderingServer,
};
use godot::prelude::*;
use serde_json::{json, Map, Value};

use self::config::Config;
use self::summary::summarize;
use crate::app::graphics;
use crate::fetcher::globals;
use crate::godot_classes::dcl_global::DclGlobal;
use crate::godot_classes::dcl_gltf_container::DclGltfContainer;
use crate::world;

const CONFIG_PATH: &str = "res://bench/genesis_plaza.config.json";
const EXPLORER_SCENE: &str = "res://src/ui/explorer.tscn";
/// A hard cap on the load: a mobile cold start is slow.
const LOAD_TIMEOUT_MS: u64 = 1_800_000;
/// The canonical benchmark pose. The scene's spawn point randomizes within a range, so two devices
/// land in different spots; screenshots need a fixed viewpoint. The midpoint of the range, in Godot
/// world coordinates.
const POSE_POSITION: Vector3 = Vector3::new(2.5, 0.5, 6.7);
const POSE_LOOK_AT: Vector3 = Vector3::new(8.0, 1.5, -28.0);
const ANDROID_PUBLIC_DIR: &str = "/sdcard/Download/gp-benchmark";
const MEGABYTE: f64 = 1_048_576.0;
const NO_LIMIT: i32 = 0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    WaitingForExplorer,
    WaitingForLoad,
    Settling,
    Warmup,
    Sampling,
    Done,
}

impl Phase {
    fn name(self) -> &'static str {
        match self {
            Phase::WaitingForExplorer => "waiting_for_explorer",
            Phase::WaitingForLoad => "waiting_for_load",
            Phase::Settling => "settling",
            Phase::Warmup => "warmup",
            Phase::Sampling => "sampling",
            Phase::Done => "done",
        }
    }
}

#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct GpBenchmarkRunner {
    base: Base<Node>,
    config: Config,
    samples: Vec<Map<String, Value>>,
    phase: Option<Phase>,
    phase_started_ms: u64,
    loading_complete: bool,
    load_started_ms: u64,
    load_seconds: f64,
    pinned: Option<(Transform3D, Basis)>,
    /// Whether any glTF container was seen loading since settling began: a count of zero means
    /// nothing until then (the scene runner announces completion before the wrappers of its
    /// entities are in the tree).
    settling_saw_loading: bool,
    settling_peak: usize,
}

#[godot_api]
impl INode for GpBenchmarkRunner {
    fn ready(&mut self) {
        self.base_mut().set_process_mode(ProcessMode::ALWAYS);
        let Some(mut config) =
            FileAccess::open(CONFIG_PATH, godot::classes::file_access::ModeFlags::READ)
                .and_then(|f| Config::parse(&f.get_as_text().to_string()))
        else {
            godot_error!("GP Benchmark: failed to load {CONFIG_PATH}; aborting");
            self.base_mut().queue_free();
            return;
        };
        let params = deep_link_params();
        let effects = config.apply_overrides(&params);
        if let Some(skip) = effects.skip_gltf {
            globals::cli().bind_mut().skip_gltf_load = skip;
        }
        if let Some(fixed) = effects.fixed_skybox_time {
            DclGlobal::singleton().bind_mut().fixed_skybox_time = fixed;
        }
        self.config = config;
        log(&format!(
            "config loaded: tag={} warmup={}s sample={}s disable_tweens={} disable_transforms={}",
            self.config.string("tag", ""),
            self.config.seconds("warmup_seconds", 0),
            self.config.seconds("sample_seconds", 0),
            self.config.toggle("disable_tweens"),
            self.config.toggle("disable_transforms"),
        ));
        let me = self.to_gd();
        globals::scene_runner()
            .signals()
            .loading_complete()
            .connect_other(&me, |this: &mut Self, session| {
                this.on_loading_complete(session)
            });
        self.set_phase(Phase::WaitingForExplorer);
    }

    fn process(&mut self, _delta: f64) {
        match self.phase {
            Some(Phase::WaitingForExplorer) => self.wait_for_explorer(),
            Some(Phase::WaitingForLoad) => self.wait_for_load(),
            Some(Phase::Settling) => self.settle(),
            Some(Phase::Warmup) => self.warm_up(),
            Some(Phase::Sampling) => self.sample(),
            _ => {}
        }
    }
}

fn is_android() -> bool {
    Os::singleton().get_name() == "Android"
}

fn log(message: &str) {
    godot_print!("[GP Benchmark] {message}");
}

fn now_ms() -> u64 {
    godot::classes::Time::singleton().get_ticks_msec()
}

fn deep_link_params() -> HashMap<String, String> {
    let link = DclGlobal::singleton().bind().deep_link_obj.clone();
    let params = link.bind().params.clone();
    params
        .iter_shared()
        .map(|(key, value)| (key.to_string(), value.stringify().to_string()))
        .collect()
}

fn root_viewport() -> Option<Gd<godot::classes::Window>> {
    DclGlobal::singleton().get_tree_or_null()?.get_root()
}

impl GpBenchmarkRunner {
    fn set_phase(&mut self, phase: Phase) {
        self.phase = Some(phase);
        self.phase_started_ms = now_ms();
        log(&format!("phase -> {}", phase.name()));
        // Markers the profiling scripts watch to record exactly during the sampling window.
        match phase {
            Phase::Sampling => log(&format!(
                "PROFILE_WINDOW_BEGIN duration_s={} pid={}",
                self.config.seconds("sample_seconds", 30),
                Os::singleton().get_process_id()
            )),
            Phase::Done => log("PROFILE_WINDOW_END"),
            _ => {}
        }
    }

    fn phase_elapsed_ms(&self) -> u64 {
        now_ms() - self.phase_started_ms
    }

    /// The scene runner also announces `-1` when there is nothing loadable yet, right at a cold
    /// start; only a positive id is a genuine session completion.
    fn on_loading_complete(&mut self, session_id: i64) {
        if session_id < 0 {
            return;
        }
        self.loading_complete = true;
        if self.load_started_ms > 0 {
            self.load_seconds = (now_ms() - self.load_started_ms) as f64 / 1000.0;
        }
        log(&format!(
            "scene_runner.loading_complete session={session_id} load_seconds={:.2}",
            self.load_seconds
        ));
    }

    fn wait_for_explorer(&mut self) {
        let in_explorer = DclGlobal::singleton()
            .get_tree_or_null()
            .and_then(|tree| tree.get_current_scene())
            .is_some_and(|scene| scene.get_scene_file_path() == EXPLORER_SCENE);
        if !in_explorer {
            return;
        }
        let mut runner = globals::scene_runner();
        {
            let mut runner = runner.bind_mut();
            runner.bench_disable_tweens = self.config.toggle("disable_tweens");
            runner.bench_disable_transforms = self.config.toggle("disable_transforms");
        }
        self.load_started_ms = now_ms();
        self.set_phase(Phase::WaitingForLoad);
    }

    fn wait_for_load(&mut self) {
        if self.loading_complete {
            self.pin_pose();
            // After the hardware benchmark has written its own pick, which would clobber it.
            self.apply_forced_graphics();
            if self.config.flag("uncap_fps", true) {
                self.uncap_fps();
            }
            self.set_phase(Phase::Settling);
        } else if self.phase_elapsed_ms() >= LOAD_TIMEOUT_MS {
            log(&format!(
                "WARN: loading_complete never fired in {LOAD_TIMEOUT_MS} ms; aborting"
            ));
            self.write_error("loading_timeout");
            quit(2);
        }
    }

    /// Measures the device's real ceiling, not a profile's FPS mode and not the display's refresh.
    fn uncap_fps(&mut self) {
        let manager = DclGlobal::singleton()
            .bind()
            .dynamic_graphics_manager
            .clone();
        {
            // The dynamic graphics manager would cascade into `Engine.max_fps` during the run, and
            // its thermal cap runs independently of its own state.
            let mut manager = manager.clone();
            let mut manager = manager.bind_mut();
            manager.set_enabled(false);
            manager.set_thermal_fps_cap_enabled(false);
            log(&format!(
                "DG disabled + thermal_cap_enabled=false (cap was={})",
                manager.get_thermal_fps_cap()
            ));
        }
        graphics::apply_fps_limit_with_thermal_cap(NO_LIMIT, 0);
        let autotune = DclGlobal::singleton().bind().graphics_autotune.clone();
        autotune.clone().bind_mut().set_thermal_cap_applies(false);
        let mut engine = Engine::singleton();
        engine.set_max_fps(0);
        // Vsync is set once, here: on Mali the driver rejects it and falls back, and re-setting it
        // every frame would rebuild the swap chain ten times a second.
        DisplayServer::singleton()
            .window_set_vsync_mode(godot::classes::display_server::VSyncMode::DISABLED);
        Os::singleton().set_low_processor_usage_mode(false);
        log(&format!(
            "FPS cap state: max_fps={} vsync_mode={:?}",
            engine.get_max_fps(),
            DisplayServer::singleton().window_get_vsync_mode()
        ));
        if let Some(root) = root_viewport() {
            RenderingServer::singleton()
                .viewport_set_measure_render_time(root.get_viewport_rid(), true);
            log("viewport measure_render_time enabled");
        }
    }

    fn settle(&mut self) {
        let timeout_ms = self.config.seconds("settling_timeout_seconds", 60) as u64 * 1000;
        let floor_ms = self.config.seconds("settling_min_seconds", 5) as u64 * 1000;
        let loading = count_loading_containers();
        if loading > 0 {
            if !self.settling_saw_loading {
                self.settling_saw_loading = true;
                log(&format!(
                    "settling: first observed {loading} loading containers @ {} ms",
                    self.phase_elapsed_ms()
                ));
            }
            self.settling_peak = self.settling_peak.max(loading);
        }
        let floor_reached = self.phase_elapsed_ms() >= floor_ms;
        if self.settling_saw_loading && loading == 0 && floor_reached {
            log(&format!(
                "settling: peak={}, all finished after {} ms",
                self.settling_peak,
                self.phase_elapsed_ms()
            ));
            self.set_phase(Phase::Warmup);
        } else if self.phase_elapsed_ms() >= timeout_ms {
            log(&format!(
                "WARN: settling_timeout ({timeout_ms}ms) — {loading} GltfContainers still loading; advancing to warmup anyway"
            ));
            self.set_phase(Phase::Warmup);
        }
    }

    fn warm_up(&mut self) {
        self.enforce_pose();
        if self.phase_elapsed_ms() < self.config.seconds("warmup_seconds", 30) as u64 * 1000 {
            return;
        }
        // The window numbers must not carry the load-time spikes.
        {
            let mut runner = globals::scene_runner();
            let mut runner = runner.bind_mut();
            runner.reset_state_timing();
            runner.reset_crdt_metrics();
        }
        let cli = globals::cli();
        if cli.bind().skip_gltf_load {
            purge_containers();
        }
        if cli.bind().kill_sky {
            purge_skies();
        }
        self.set_phase(Phase::Sampling);
    }

    fn sample(&mut self) {
        self.enforce_pose();
        self.samples.push(collect_sample(self.phase_elapsed_ms()));
        if self.phase_elapsed_ms() >= self.config.seconds("sample_seconds", 30) as u64 * 1000 {
            self.finish();
        }
    }

    fn pin_pose(&mut self) {
        let Some(mut player) = world::state::player() else {
            log("WARN: no player node when pinning pose");
            return;
        };
        player.set_global_position(POSE_POSITION);
        player.look_at(POSE_LOOK_AT);
        let camera_basis = DclGlobal::singleton()
            .bind()
            .player_camera()
            .map(|camera| camera.get_global_transform().basis);
        self.pinned = Some((
            player.get_global_transform(),
            camera_basis.unwrap_or(Basis::IDENTITY),
        ));
        log(&format!(
            "pose pinned: pos={POSE_POSITION:?} look={POSE_LOOK_AT:?}"
        ));
    }

    /// Every frame of the window renders the same viewpoint.
    fn enforce_pose(&mut self) {
        let Some((transform, basis)) = self.pinned else {
            return;
        };
        if let Some(mut player) = world::state::player() {
            player.set_global_transform(transform);
            player.set_velocity(Vector3::ZERO);
        }
        let camera = DclGlobal::singleton().bind().player_camera();
        if let Some(mut camera) = camera {
            let mut pose = camera.get_global_transform();
            pose.basis = basis;
            camera.set_global_transform(pose);
        }
    }

    fn apply_forced_graphics(&self) {
        let engine = globals::engine();
        if let Some(index) = self.config.int("force_graphic_profile") {
            graphics::apply_graphic_profile(index as i32);
            // The profile's FPS mode would silently cap the run.
            engine
                .bind()
                .adopt_setting("limit_fps".into(), NO_LIMIT.to_variant());
            log(&format!(
                "forced graphic_profile={index}, limit_fps=NO_LIMIT (post-load)"
            ));
        }
        if let (Some(scale), Some(mut root)) =
            (self.config.float("viewport_scale_3d"), root_viewport())
        {
            root.set_scaling_3d_scale(scale as f32);
            log(&format!("viewport scaling_3d_scale={scale:.2} (post-load)"));
        }
        if let (Some(threshold), Some(mut root)) =
            (self.config.float("mesh_lod_threshold"), root_viewport())
        {
            root.set_mesh_lod_threshold(threshold as f32);
            log(&format!(
                "viewport mesh_lod_threshold={threshold:.2} (post-load)"
            ));
        }
        // After the forced profile, so "Medium with feature X off" isolates that feature's cost.
        for (key, setting) in [
            ("gfx-aa", "anti_aliasing"),
            ("gfx-shadow", "shadow_quality"),
            ("gfx-bloom", "bloom_quality"),
            ("gfx-skybox", "skybox"),
            ("gfx-texture", "texture_quality"),
        ] {
            if let Some(level) = self.config.int(key) {
                engine
                    .bind()
                    .adopt_setting(setting.into(), level.to_variant());
                log(&format!("{key} override = {level}"));
            }
        }
        // Last, so it overrides any rendering mode the profile applied: the screenshot then shows
        // whatever heatmap is active.
        let draw = self.config.string("debug_draw", "");
        if !draw.is_empty() {
            match (debug_draw(&draw), root_viewport()) {
                (Some(mode), Some(mut root)) => {
                    root.set_debug_draw(mode);
                    log(&format!("debug_draw override = {draw}"));
                }
                _ => log(&format!(
                    "WARN: unknown debug-draw value '{draw}', leaving disabled"
                )),
            }
        }
    }

    fn finish(&mut self) {
        log(&format!("sampling done: {} samples", self.samples.len()));
        self.set_phase(Phase::Done);
        // The per-state CPU timing and the CRDT throughput of the sampling window only.
        let (state_timing, crdt_metrics, crdt_breakdown) = {
            let mut runner = globals::scene_runner();
            let mut runner = runner.bind_mut();
            (
                runner.drain_state_timing().to_string(),
                runner.drain_crdt_metrics().to_string(),
                runner.drain_crdt_component_breakdown().to_string(),
            )
        };
        let summary = summarize(&self.samples);
        let result = json!({
            "tag": self.config.string("tag", ""),
            "genesis_plaza_commit": self.config.string("genesis_plaza_commit", ""),
            "toggles": self.config.0.get("toggles").cloned().unwrap_or_default(),
            "state_timing_us": state_timing,
            "crdt_metrics": crdt_metrics,
            "crdt_component_breakdown": crdt_breakdown,
            "warmup_seconds": self.config.seconds("warmup_seconds", 0),
            "sample_seconds": self.config.seconds("sample_seconds", 0),
            "samples_collected": self.samples.len(),
            "load_seconds": self.load_seconds,
            "build_mode": if Os::singleton().is_debug_build() { "debug" } else { "release" },
            "platform": Os::singleton().get_name().to_string(),
            "summary": summary,
            "samples": self.samples,
        });
        self.write_result(&result);
        if is_android() {
            self.mirror_to_public_path(&result);
        }
        print_summary(&result);
        // Last, so a slow screenshot cannot delay the result or contaminate the sample window.
        self.save_screenshot();
        if self.config.flag("keep_running", false) {
            log("keep_running=true — skipping force-quit, free-roam now");
            return;
        }
        quit(0);
    }

    fn write_result(&self, result: &Value) {
        let path = self.config.output_path();
        write_json(&path, result, true);
        log(&format!("wrote {path}"));
    }

    fn write_error(&self, reason: &str) {
        let report = json!({
            "tag": self.config.string("tag", ""),
            "error": reason,
            "phase": self.phase.map(Phase::name).unwrap_or("init"),
            "loading_complete_seen": self.loading_complete,
        });
        write_json(&self.config.output_path(), &report, false);
    }

    /// The private sandbox of a release APK cannot be pulled; the public folder can.
    fn mirror_to_public_path(&self, result: &Value) {
        let path = format!(
            "{ANDROID_PUBLIC_DIR}/{}.json",
            self.config.string("tag", "result")
        );
        write_json(&path, result, false);
        log(&format!("mirrored result to {path}"));
    }

    /// A sanity-check image, compared with the previous run's by the profiling scripts: a run that
    /// diverges is flagged (another scene loaded, the character moved, an asset failed).
    fn save_screenshot(&self) {
        let Some(image) = self
            .base()
            .get_viewport()
            .and_then(|v| v.get_texture())
            .and_then(|t| t.get_image())
        else {
            log("WARN: no image for the screenshot");
            return;
        };
        let tag = self.config.string("tag", "result");
        let path = format!("user://output/gp-benchmark/{tag}.png");
        crate::test_runner::testing_tools::ensure_parent_dir(&path);
        if image.save_png(path.as_str()) == godot::global::Error::OK {
            log(&format!("screenshot -> {path}"));
        }
        if is_android() {
            let public = format!("{ANDROID_PUBLIC_DIR}/{tag}.png");
            if image.save_png(public.as_str()) == godot::global::Error::OK {
                log(&format!("mirrored screenshot to {public}"));
            }
        }
    }
}

fn write_json(path: &str, value: &Value, pretty: bool) {
    if let Some((dir, _)) = path.rsplit_once('/') {
        DirAccess::make_dir_recursive_absolute(dir);
    }
    let Some(mut file) = FileAccess::open(path, godot::classes::file_access::ModeFlags::WRITE)
    else {
        godot_error!("GP Benchmark: cannot open {path} for writing");
        return;
    };
    let text = if pretty {
        serde_json::to_string_pretty(value)
    } else {
        serde_json::to_string(value)
    };
    file.store_string(text.unwrap_or_default().as_str());
}

/// A clean quit first; on iOS the OS sometimes suspends instead, so a kill follows to make sure
/// the client closes between matrix runs.
fn quit(code: i32) {
    if let Some(mut tree) = DclGlobal::singleton().get_tree_or_null() {
        tree.quit_ex().exit_code(code).done();
    }
    crate::engine::local_task::spawn_local(async {
        crate::engine::local_task::sleep(2.0).await;
        Os::singleton().kill(Os::singleton().get_process_id());
    });
}

fn count_loading_containers() -> usize {
    containers()
        .iter()
        .filter(|c| c.bind().is_loading())
        .count()
}

/// Every glTF container of the tree.
fn containers() -> Vec<Gd<DclGltfContainer>> {
    let mut found = Vec::new();
    let mut stack: Vec<Gd<Node>> = root_viewport().into_iter().map(|r| r.upcast()).collect();
    while let Some(node) = stack.pop() {
        match node.clone().try_cast::<DclGltfContainer>() {
            Ok(container) => found.push(container),
            Err(_) => stack.extend(node.get_children().iter_shared()),
        }
    }
    found
}

fn purge_containers() {
    let found = containers();
    for mut container in found.clone() {
        container.queue_free();
    }
    log(&format!(
        "purged {} existing DclGltfContainer nodes",
        found.len()
    ));
}

fn purge_skies() {
    use godot::classes::environment::BgMode;
    use godot::classes::WorldEnvironment;
    let mut stomped = 0;
    let mut stack: Vec<Gd<Node>> = root_viewport().into_iter().map(|r| r.upcast()).collect();
    while let Some(node) = stack.pop() {
        if let Ok(world) = node.clone().try_cast::<WorldEnvironment>() {
            if let Some(mut environment) = world.get_environment() {
                environment.set_background(BgMode::COLOR);
                environment.set_bg_color(Color::BLACK);
                environment.set_bg_energy_multiplier(0.0);
                environment.set_glow_enabled(false);
                stomped += 1;
            }
        }
        stack.extend(node.get_children().iter_shared());
    }
    log(&format!("purged {stomped} WorldEnvironments to BG_COLOR"));
}

fn debug_draw(name: &str) -> Option<DebugDraw> {
    Some(match name {
        "unshaded" => DebugDraw::UNSHADED,
        "lighting" => DebugDraw::LIGHTING,
        "overdraw" => DebugDraw::OVERDRAW,
        "wireframe" => DebugDraw::WIREFRAME,
        "normal-buffer" => DebugDraw::NORMAL_BUFFER,
        "shadow-atlas" => DebugDraw::SHADOW_ATLAS,
        "directional-shadow-atlas" => DebugDraw::DIRECTIONAL_SHADOW_ATLAS,
        "scene-luminance" => DebugDraw::SCENE_LUMINANCE,
        "ssao" => DebugDraw::SSAO,
        "ssil" => DebugDraw::SSIL,
        "motion-vectors" => DebugDraw::MOTION_VECTORS,
        "disable-lod" => DebugDraw::DISABLE_LOD,
        "cluster-omni" => DebugDraw::CLUSTER_OMNI_LIGHTS,
        "cluster-spot" => DebugDraw::CLUSTER_SPOT_LIGHTS,
        "cluster-decals" => DebugDraw::CLUSTER_DECALS,
        "cluster-reflection-probes" => DebugDraw::CLUSTER_REFLECTION_PROBES,
        "occluders" => DebugDraw::OCCLUDERS,
        "disabled" | "off" => DebugDraw::DISABLED,
        _ => return None,
    })
}

/// The resident set of the process: the C++ heap alone misses the Java and native allocations of
/// an Android app.
fn process_rss_mb() -> f64 {
    let Ok(status) = std::fs::read_to_string("/proc/self/status") else {
        return 0.0;
    };
    status
        .lines()
        .find_map(|line| line.strip_prefix("VmRSS:"))
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|kb| kb.parse::<f64>().ok())
        .map_or(0.0, |kb| kb / 1024.0)
}

fn collect_sample(elapsed_ms: u64) -> Map<String, Value> {
    let performance = Performance::singleton();
    let monitor = |m: Monitor| performance.get_monitor(m);
    let mut server = RenderingServer::singleton();
    let viewport = root_viewport()
        .map(|r| r.get_viewport_rid())
        .unwrap_or(Rid::Invalid);
    // Per-pass split: the shadow, visible (opaque and transparent together) and canvas passes.
    let mut info = |kind: ViewportRenderInfoType, what: ViewportRenderInfo| {
        server.viewport_get_render_info(viewport, kind, what) as i64
    };
    let visible = ViewportRenderInfoType::VISIBLE;
    let shadow = ViewportRenderInfoType::SHADOW;
    let (visible_draws, visible_objects, visible_prim) = (
        info(visible, ViewportRenderInfo::DRAW_CALLS_IN_FRAME),
        info(visible, ViewportRenderInfo::OBJECTS_IN_FRAME),
        info(visible, ViewportRenderInfo::PRIMITIVES_IN_FRAME),
    );
    let (shadow_draws, shadow_objects, shadow_prim) = (
        info(shadow, ViewportRenderInfo::DRAW_CALLS_IN_FRAME),
        info(shadow, ViewportRenderInfo::OBJECTS_IN_FRAME),
        info(shadow, ViewportRenderInfo::PRIMITIVES_IN_FRAME),
    );
    let canvas_draws = info(
        ViewportRenderInfoType::CANVAS,
        ViewportRenderInfo::DRAW_CALLS_IN_FRAME,
    );
    let os = Os::singleton();
    let loaded_scenes = DclGlobal::singleton()
        .bind()
        .scene_fetcher
        .clone()
        .map_or(0, |f| f.bind().get_loaded_scene_count());
    let mut sample = Map::new();
    let mut put = |key: &str, value: Value| {
        sample.insert(key.to_string(), value);
    };
    put("t_ms", json!(elapsed_ms));
    put("fps", json!(monitor(Monitor::TIME_FPS)));
    put(
        "frame_time_process_ms",
        json!(monitor(Monitor::TIME_PROCESS) * 1000.0),
    );
    put(
        "frame_time_physics_ms",
        json!(monitor(Monitor::TIME_PHYSICS_PROCESS) * 1000.0),
    );
    put(
        "render_cpu_ms",
        json!(RenderingServer::singleton().viewport_get_measured_render_time_cpu(viewport)),
    );
    put(
        "render_gpu_ms",
        json!(RenderingServer::singleton().viewport_get_measured_render_time_gpu(viewport)),
    );
    put(
        "memory_static_mb",
        json!(monitor(Monitor::MEMORY_STATIC) / MEGABYTE),
    );
    put(
        "memory_rss_mb",
        json!(os.get_static_memory_usage() as f64 / MEGABYTE),
    );
    put(
        "memory_peak_mb",
        json!(os.get_static_memory_peak_usage() as f64 / MEGABYTE),
    );
    put(
        "video_mem_mb",
        json!(monitor(Monitor::RENDER_VIDEO_MEM_USED) / MEGABYTE),
    );
    put(
        "texture_mem_mb",
        json!(monitor(Monitor::RENDER_TEXTURE_MEM_USED) / MEGABYTE),
    );
    put(
        "buffer_mem_mb",
        json!(monitor(Monitor::RENDER_BUFFER_MEM_USED) / MEGABYTE),
    );
    put("process_rss_mb", json!(process_rss_mb()));
    put("object_count", json!(monitor(Monitor::OBJECT_COUNT)));
    put(
        "resource_count",
        json!(monitor(Monitor::OBJECT_RESOURCE_COUNT)),
    );
    put("node_count", json!(monitor(Monitor::OBJECT_NODE_COUNT)));
    put(
        "orphan_node_count",
        json!(monitor(Monitor::OBJECT_ORPHAN_NODE_COUNT)),
    );
    put(
        "draw_calls",
        json!(monitor(Monitor::RENDER_TOTAL_DRAW_CALLS_IN_FRAME)),
    );
    put(
        "render_objects_in_frame",
        json!(monitor(Monitor::RENDER_TOTAL_OBJECTS_IN_FRAME)),
    );
    put(
        "primitives",
        json!(monitor(Monitor::RENDER_TOTAL_PRIMITIVES_IN_FRAME)),
    );
    put("visible_draws", json!(visible_draws));
    put("visible_objects", json!(visible_objects));
    put("visible_prim", json!(visible_prim));
    put("shadow_draws", json!(shadow_draws));
    put("shadow_objects", json!(shadow_objects));
    put("shadow_prim", json!(shadow_prim));
    put("canvas_draws", json!(canvas_draws));
    put(
        "physics_active_objects",
        json!(monitor(Monitor::PHYSICS_3D_ACTIVE_OBJECTS)),
    );
    put(
        "physics_collision_pairs",
        json!(monitor(Monitor::PHYSICS_3D_COLLISION_PAIRS)),
    );
    put(
        "physics_island_count",
        json!(monitor(Monitor::PHYSICS_3D_ISLAND_COUNT)),
    );
    put("loaded_scene_count", json!(loaded_scenes));
    put("engine_max_fps", json!(Engine::singleton().get_max_fps()));
    put(
        "engine_physics_ticks",
        json!(Engine::singleton().get_physics_ticks_per_second()),
    );
    put(
        "vsync_mode",
        json!(DisplayServer::singleton().window_get_vsync_mode().ord()),
    );
    sample
}

/// The summary one field per line: logcat truncates long lines, and a single-line JSON would lose
/// everything past about 4 KB.
fn print_summary(result: &Value) {
    println!("[GP Benchmark] BEGIN_RESULT_JSON");
    for key in [
        "tag",
        "platform",
        "build_mode",
        "genesis_plaza_commit",
        "samples_collected",
    ] {
        let value = &result[key];
        let text = value
            .as_str()
            .map_or_else(|| value.to_string(), str::to_string);
        println!("[GP Benchmark] {key}={text}");
    }
    if let Some(summary) = result["summary"].as_object() {
        for (key, stat) in summary {
            let field = |name: &str| stat[name].as_f64().unwrap_or(0.0);
            println!(
                "[GP Benchmark] {key} mean={:.3} p50={:.3} p95={:.3} min={:.3} max={:.3}",
                field("mean"),
                field("p50"),
                field("p95"),
                field("min"),
                field("max")
            );
        }
    }
    println!("[GP Benchmark] END_RESULT_JSON");
}
