//! Creation of the services the app runs on. `GlobalBoot` calls these from its `ready`, so the
//! services exist before the autoloads that follow it read them. Each service is held by its
//! `Global` field; the ones that are nodes join `/root` once everything exists.

use godot::prelude::*;

use crate::analytics::metrics::Metrics;
use crate::app::{
    analytics_controller::AnalyticsController, feature_flags::FeatureFlags,
    gltf_importer::CustomGltfImporter, music_player::MusicPlayer, snapshot::Snapshot,
};
use crate::engine::local_task::defer;
use crate::fetcher::scene_fetcher::SceneFetcher;
use crate::godot_classes::dcl_global::DclGlobal;
use crate::godot_classes::dcl_session_store::DclSessionStore;
use crate::sky::skybox_time::SkyboxTime;

/// Joins `/root` at the end of the frame, in the order the calls are made.
fn add_to_root_deferred(global: &Gd<DclGlobal>, node: Gd<Node>) {
    if let Some(mut root) = global.get_tree_or_null().and_then(|tree| tree.get_root()) {
        defer(move || root.add_child(&node));
    }
}

/// Starts the Genesis Plaza profiling benchmark runner under `Global`.
pub fn spawn_gp_benchmark_runner(global: &mut Gd<DclGlobal>) {
    let mut runner = crate::tools::gp_benchmark::GpBenchmarkRunner::new_alloc();
    runner.set_name("GPBenchmarkRunner");
    global.add_child(&runner);
}

/// The services every boot needs.
pub fn create_core_services(global: &mut Gd<DclGlobal>) {
    global.bind_mut().music_player = Some(MusicPlayer::new_alloc());
    let snapshot = Snapshot::new_alloc();
    global.add_child(&snapshot);
    global.bind_mut().snapshot = Some(snapshot);
}

/// Runs the boot that is not the app: the Rust test runner. Returns true when one started and the
/// boot should stop there.
pub fn start_special_profile(global: &mut Gd<DclGlobal>) -> bool {
    if !crate::fetcher::globals::cli().bind().test_runner {
        return false;
    }
    godot_print!("Running godot-tests...");
    // The #[itest]s only need a Godot context, not the whole explorer.
    let runner = crate::test_runner::test_suite::TestRunnerSuite::new_alloc();
    global.add_child(&runner);
    true
}

/// Creates the services of a full boot and queues the `/root` adds. Metrics are skipped when
/// telemetry is off.
pub fn create_app_services(global: &mut Gd<DclGlobal>, session_id: &str, telemetry_enabled: bool) {
    if telemetry_enabled {
        let mut user_id = DclSessionStore::load_analytics_user_id().to_string();
        if user_id.is_empty() {
            user_id = uuid::Uuid::new_v4().to_string();
            DclSessionStore::save_analytics_user_id(user_id.as_str().into());
        }
        let mut metrics = Metrics::create_metrics(user_id, session_id.to_string());
        metrics.bind_mut().set_debug_level(0);
        metrics.set_name("metrics");
        global.bind_mut().metrics = metrics;
    }

    // Hooked before the first load so the budget sees its loading_started.
    crate::engine::frame_budget::ensure_driver();
    // Rewrites glTF references before an import; the registry keeps it alive.
    godot::classes::GltfDocument::register_gltf_document_extension(&CustomGltfImporter::new_gd());
    let mut fetcher = SceneFetcher::new_alloc();
    fetcher.set_name("scene_fetcher");
    let mut skybox_time = SkyboxTime::new_alloc();
    skybox_time.set_name("skybox_time");
    {
        let mut g = global.bind_mut();
        g.scene_fetcher = Some(fetcher);
        g.skybox_time = Some(skybox_time);
    }

    let nodes: Vec<Gd<Node>> = {
        let g = global.bind();
        let mut nodes: Vec<Gd<Node>> = vec![g.cli.clone().upcast(), g.engine.clone().upcast()];
        nodes.extend(g.music_player.clone().map(|n| n.upcast()));
        nodes.extend(g.scene_fetcher.clone().map(|n| n.upcast()));
        nodes.extend(g.skybox_time.clone().map(|n| n.upcast()));
        nodes.extend([
            g.content_provider.clone().upcast(),
            g.scene_runner.clone().upcast(),
            g.realm.clone().upcast(),
            g.dcl_tokio_rpc.clone().upcast(),
            g.player_identity.clone().upcast(),
            g.comms.clone().upcast(),
            g.avatars.clone().upcast(),
            g.portable_experience_controller.clone().upcast(),
            g.testing_tools.clone().upcast(),
        ]);
        nodes
    };
    for node in nodes {
        add_to_root_deferred(global, node);
    }

    // Ranks the avatars by distance for the LOD pool and fills their impostor textures.
    let mut lod_system = crate::avatars::impostor_capture::AvatarLodSystem::new_alloc();
    lod_system.set_name("AvatarLodSystem");
    add_to_root_deferred(global, lod_system.upcast());
    let mut sounds = crate::ui::ui_sounds::UiSounds::new_alloc();
    sounds.set_name("UiSounds");
    add_to_root_deferred(global, sounds.upcast());
    let mut lights = crate::lights::manager::DclLightSourceManager::new_alloc();
    lights.set_name("LightSourceManager");
    add_to_root_deferred(global, lights.upcast());
    if telemetry_enabled {
        let metrics = global.bind().metrics.clone();
        add_to_root_deferred(global, metrics.upcast());
        // Firebase/Segment orchestration lives in AnalyticsController, a RefCounted kept alive by
        // this reference; it spawns its own transient timer under Global.
        let mut controller = AnalyticsController::new_gd();
        global.bind_mut().analytics_controller = Some(controller.clone());
        controller.bind_mut().setup();
    }
}

/// The nodes that follow the metrics block: feature flags under `Global`, the late `/root` nodes,
/// the scene-inspector bridge and the opt-in dev tools.
pub fn create_extra_services(global: &mut Gd<DclGlobal>) {
    // Feature flags fetch on `ready`.
    let mut flags = FeatureFlags::new_alloc();
    flags.set_name("feature_flags");
    global.add_child(&flags);
    global.bind_mut().feature_flags = Some(flags);

    let late: Vec<Gd<Node>> = {
        let g = global.bind();
        vec![
            g.network_inspector.clone().upcast(),
            g.scene_inspector_dispatcher.clone().upcast(),
            g.social_blacklist.clone().upcast(),
            g.dynamic_graphics_manager.clone().upcast(),
        ]
    };
    for node in late {
        add_to_root_deferred(global, node);
    }

    // Dial the configured hub from second 0, so the channel (and, in debug, boot-log capture) is
    // live from boot; the bridge also handles `?scene-inspector=` deeplinks.
    let mut bridge = crate::tools::debug_hub::SceneInspectorBridge::new_alloc();
    bridge.set_name("scene_inspector_bridge");
    bridge.bind_mut().activate_from_config();
    global.bind_mut().scene_inspector_bridge = Some(bridge.clone().upcast());
    add_to_root_deferred(global, bridge.upcast());

    #[cfg(feature = "use_memory_debugger")]
    {
        let (memory, report) = {
            let g = global.bind();
            (g.memory_debugger.clone(), g.benchmark_report.clone())
        };
        add_to_root_deferred(global, memory.upcast());
        if crate::fetcher::globals::cli().bind().benchmark_report {
            godot_print!("✓ BenchmarkReport initialized for full flow benchmarking");
            add_to_root_deferred(global, report.upcast());
            crate::engine::local_task::spawn_local(crate::tools::benchmark_flow::run());
        }
    }
    #[cfg(not(feature = "use_memory_debugger"))]
    if crate::fetcher::globals::cli().bind().benchmark_report {
        godot_error!(
            "BenchmarkReport requires --features use_memory_debugger to be enabled during build"
        );
    }
}
