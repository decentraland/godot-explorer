//! Creation of the services the app runs on. `Global` (a GDScript autoload) calls these from its
//! `_ready`, so the services exist before the autoloads that follow it read them; the property
//! assignment goes through `Global`'s own setters, exactly as the GDScript `.new()` lines did.

use godot::classes::{GDScript, ResourceLoader};
use godot::prelude::*;

use crate::analytics::metrics::Metrics;
use crate::app::{
    gltf_importer::CustomGltfImporter, music_player::MusicPlayer, snapshot::Snapshot,
};
use crate::fetcher::scene_fetcher::SceneFetcher;
use crate::godot_classes::dcl_cli::DclCli;
use crate::godot_classes::dcl_session_store::DclSessionStore;

/// Where a created node goes.
#[derive(Clone, Copy)]
enum Attach {
    /// Held by the property only.
    None,
    /// A child of `Global`, added immediately.
    GlobalChild,
}

/// Where a service comes from.
#[derive(Clone, Copy)]
enum Source {
    /// A GDScript class, under `res://`.
    Script(&'static str),
    /// A native class.
    Native(fn() -> Variant),
}

struct ServiceSpec {
    /// The `Global` property that holds the service.
    property: &'static str,
    source: Source,
    /// `Node.name` to set, when the old code set one.
    node_name: Option<&'static str>,
    attach: Attach,
}

const fn spec(
    property: &'static str,
    script: &'static str,
    node_name: Option<&'static str>,
    attach: Attach,
) -> ServiceSpec {
    ServiceSpec {
        property,
        source: Source::Script(script),
        node_name,
        attach,
    }
}

const fn native(
    property: &'static str,
    make: fn() -> Variant,
    node_name: Option<&'static str>,
    attach: Attach,
) -> ServiceSpec {
    ServiceSpec {
        property,
        source: Source::Native(make),
        node_name,
        attach,
    }
}

/// Services every boot needs, in creation order.
const CORE_SERVICES: [ServiceSpec; 4] = [
    native(
        "music_player",
        || MusicPlayer::new_alloc().to_variant(),
        None,
        Attach::None,
    ),
    native(
        "snapshot",
        || Snapshot::new_alloc().to_variant(),
        None,
        Attach::GlobalChild,
    ),
    spec(
        "player_identity",
        "src/logic/player/player_identity.gd",
        Some("player_identity"),
        Attach::None,
    ),
    spec(
        "testing_tools",
        "src/test/testing_api.gd",
        Some("testing_tool"),
        Attach::None,
    ),
];

fn create_scene_fetcher(global: &mut Gd<Node>) {
    let mut fetcher = SceneFetcher::new_alloc();
    fetcher.set_name("scene_fetcher");
    global.set("scene_fetcher", &fetcher.to_variant());
}

/// A new instance of the GDScript class at `res://src/<script>`.
fn instantiate_script(script: &str) -> Result<Variant, String> {
    let path = GString::from(&format!("res://{script}"));
    let resource = ResourceLoader::singleton()
        .load_ex(&path)
        .type_hint("Script")
        .done()
        .ok_or_else(|| format!("{path} did not load"))?;
    let mut script = resource
        .try_cast::<GDScript>()
        .map_err(|_| format!("{path} is not a GDScript"))?;
    let instance = script.call("new", &[]);
    if instance.is_nil() {
        return Err(format!("{path}: new() returned nothing"));
    }
    Ok(instance)
}

fn create(global: &mut Gd<Node>, specs: &[ServiceSpec]) -> bool {
    let mut ok = true;
    for service in specs {
        let made = match service.source {
            Source::Script(script) => instantiate_script(script),
            Source::Native(make) => Ok(make()),
        };
        let instance = match made {
            Ok(instance) => instance,
            Err(reason) => {
                godot_error!("bootstrap: cannot create `{}`: {reason}", service.property);
                ok = false;
                continue;
            }
        };
        if let (Some(name), Ok(mut node)) = (service.node_name, instance.try_to::<Gd<Node>>()) {
            node.set_name(name);
        }
        global.set(service.property, &instance);
        if let (Attach::GlobalChild, Ok(node)) = (service.attach, instance.try_to::<Gd<Node>>()) {
            global.add_child(&node);
        }
    }
    ok
}

/// Services of a full boot, created after the config exists. Each is held by its `Global` property.
const APP_SERVICES: [ServiceSpec; 1] = [spec(
    "skybox_time",
    "assets/environment/time.gd",
    Some("skybox_time"),
    Attach::None,
)];

/// Nodes added to `/root` (deferred, in this order) once the app services exist. The order is
/// the order the old `global.gd` used, and later deferred calls rely on it.
const ROOT_NODES: [&str; 14] = [
    "cli",
    "engine",
    "music_player",
    "scene_fetcher",
    "skybox_time",
    "content_provider",
    "scene_runner",
    "realm",
    "dcl_tokio_rpc",
    "player_identity",
    "comms",
    "avatars",
    "portable_experience_controller",
    "testing_tools",
];

/// Rust-owned nodes that join `/root` after the Global-child services.
const LATE_ROOT_NODES: [&str; 4] = [
    "network_inspector",
    "scene_inspector_dispatcher",
    "social_blacklist",
    "dynamic_graphics_manager",
];

fn node_of(global: &Gd<Node>, property: &str) -> Option<Gd<Node>> {
    global.get(property).try_to::<Gd<Node>>().ok()
}

fn add_to_root_deferred(global: &Gd<Node>, property: &str) {
    let Some(node) = node_of(global, property) else {
        godot_error!("bootstrap: `{property}` is not a node, not added to the tree");
        return;
    };
    if let Some(mut root) = global.get_tree_or_null().and_then(|tree| tree.get_root()) {
        root.call_deferred("add_child", &[node.to_variant()]);
    }
}

fn root_add_instance(global: &Gd<Node>, node: &Gd<Node>) {
    if let Some(mut root) = global.get_tree_or_null().and_then(|tree| tree.get_root()) {
        root.call_deferred("add_child", &[node.to_variant()]);
    }
}

fn cli(global: &Gd<Node>) -> Option<Gd<DclCli>> {
    global.get("cli").try_to::<Gd<DclCli>>().ok()
}

/// A script instance named and added to `/root`, with an optional call on it. Failures are logged.
fn spawn_on_root(global: &Gd<Node>, script: &str, name: &str) -> Option<Gd<Node>> {
    let mut node = instantiate_script(script)
        .ok()
        .and_then(|instance| instance.try_to::<Gd<Node>>().ok());
    match node.as_mut() {
        Some(node) => {
            node.set_name(name);
            root_add_instance(global, node);
        }
        None => godot_error!("bootstrap: cannot create `{name}` from {script}"),
    }
    node
}

/// Static entry points for `global.gd`.
#[derive(GodotClass)]
#[class(init, base=RefCounted)]
pub struct DclBootstrap {
    _base: Base<RefCounted>,
}

#[godot_api]
impl DclBootstrap {
    /// Creates the services every boot needs and assigns them to `global`. Returns false if any
    /// could not be created (each failure is logged).
    #[func]
    pub fn create_core_services(mut global: Gd<Node>) -> bool {
        create(&mut global, &CORE_SERVICES)
    }

    /// Runs the boot that is not the app: the Rust test runner, or the floating-islands
    /// benchmark. Returns true when one started and `Global._ready` should stop there.
    #[func]
    pub fn start_special_profile(mut global: Gd<Node>) -> bool {
        let Some(cli) = cli(&global) else {
            return false;
        };
        let (test_runner, fi_benchmark) = {
            let cli = cli.bind();
            (cli.test_runner, cli.fi_benchmark_size >= 0)
        };

        if test_runner {
            godot_print!("Running godot-tests...");
            // The #[itest]s only need a Godot context, not the whole explorer.
            if let Ok(instance) = instantiate_script("src/test/test_runner.gd") {
                if let Ok(mut runner) = instance.try_to::<Gd<Node>>() {
                    global.add_child(&runner);
                    runner.call_deferred("start", &[]);
                }
            }
            return true;
        }

        if fi_benchmark {
            godot_print!("Running Floating Islands Benchmark...");
            // Only the components the benchmark needs.
            create_scene_fetcher(&mut global);
            for property in ["scene_fetcher", "scene_runner", "content_provider"] {
                add_to_root_deferred(&global, property);
            }
            spawn_on_root(
                &global,
                "src/tools/fi_benchmark_runner.gd",
                "FIBenchmarkRunner",
            );
            return true;
        }
        false
    }

    /// Creates the services of a full boot, assigns them to `global`, and queues the `/root`
    /// adds. Metrics are skipped when telemetry is off.
    #[func]
    pub fn create_app_services(
        mut global: Gd<Node>,
        session_id: GString,
        telemetry_enabled: bool,
    ) -> bool {
        if telemetry_enabled {
            let mut user_id = DclSessionStore::load_analytics_user_id().to_string();
            if user_id.is_empty() {
                user_id = uuid::Uuid::new_v4().to_string();
                DclSessionStore::save_analytics_user_id(user_id.as_str().into());
            }
            let mut metrics = Metrics::create_metrics(user_id, session_id.to_string());
            metrics.bind_mut().set_debug_level(0);
            metrics.set_name("metrics");
            global.set("metrics", &metrics.to_variant());
        }

        // Rewrites glTF references before an import; the registry keeps it alive.
        godot::classes::GltfDocument::register_gltf_document_extension(
            &CustomGltfImporter::new_gd(),
        );
        create_scene_fetcher(&mut global);
        let mut ok = create(&mut global, &APP_SERVICES);

        for property in ROOT_NODES {
            add_to_root_deferred(&global, property);
        }
        // Ranks the avatars by distance for the LOD pool and fills their impostor textures.
        let mut lod_system = crate::avatars::impostor_capture::AvatarLodSystem::new_alloc();
        lod_system.set_name("AvatarLodSystem");
        root_add_instance(&global, &lod_system.upcast());
        let mut sounds = crate::ui::ui_sounds::UiSounds::new_alloc();
        sounds.set_name("UiSounds");
        root_add_instance(&global, &sounds.upcast());
        let mut lights = crate::lights::manager::DclLightSourceManager::new_alloc();
        lights.set_name("LightSourceManager");
        root_add_instance(&global, &lights.upcast());
        if let Some(metrics) = node_of(&global, "metrics") {
            root_add_instance(&global, &metrics);
            // Firebase/Segment orchestration lives in AnalyticsController, a RefCounted kept
            // alive by this reference; it spawns its own transient timer under Global.
            match instantiate_script("src/logic/metrics/analytics_controller.gd") {
                Ok(controller) => {
                    global.set("analytics_controller", &controller);
                    controller.call("setup", &[]);
                }
                Err(reason) => {
                    godot_error!("bootstrap: cannot create `analytics_controller`: {reason}");
                    ok = false;
                }
            }
        }
        ok
    }

    /// The nodes that follow the metrics block: feature flags under `Global`, the late `/root`
    /// nodes, the scene-inspector bridge and the opt-in dev tools.
    #[func]
    pub fn create_extra_services(mut global: Gd<Node>) -> bool {
        let mut ok = true;
        // Feature flags fetch on `_ready`.
        for (property, script, name) in [(
            "feature_flags",
            "src/feature_flags.gd",
            Some("feature_flags"),
        )] {
            // The host-driven core runs attestation itself (the host provides the OS calls).
            if property == "attestation" && super::DclEngine::core_is_driving() {
                continue;
            }
            match instantiate_script(script) {
                Ok(instance) => {
                    if let Ok(mut node) = instance.try_to::<Gd<Node>>() {
                        if let Some(name) = name {
                            node.set_name(name);
                        }
                        global.add_child(&node);
                    }
                    global.set(property, &instance);
                }
                Err(reason) => {
                    godot_error!("bootstrap: cannot create `{property}`: {reason}");
                    ok = false;
                }
            }
        }

        for property in LATE_ROOT_NODES {
            add_to_root_deferred(&global, property);
        }

        // Dial the configured hub from second 0, so the channel (and, in debug, boot-log
        // capture) is live from boot; the bridge also handles `?scene-inspector=` deeplinks.
        match instantiate_script("src/tool/scene_inspector_bridge.gd") {
            Ok(bridge) => {
                if let Ok(mut node) = bridge.try_to::<Gd<Node>>() {
                    node.set_name("scene_inspector_bridge");
                }
                global.set("_scene_inspector_bridge", &bridge);
                bridge.call("activate_from_config", &[]);
                if let Ok(node) = bridge.try_to::<Gd<Node>>() {
                    root_add_instance(&global, &node);
                }
            }
            Err(reason) => {
                godot_error!("bootstrap: cannot create the scene inspector bridge: {reason}");
                ok = false;
            }
        }

        if global.has_method("get_memory_debugger")
            || global.get("memory_debugger").get_type() != VariantType::NIL
        {
            add_to_root_deferred(&global, "memory_debugger");
        }

        let (benchmark_report, stress_test) = cli(&global)
            .map(|cli| {
                let cli = cli.bind();
                (cli.benchmark_report, cli.stress_test)
            })
            .unwrap_or_default();

        if benchmark_report && global.get("benchmark_report").get_type() != VariantType::NIL {
            godot_print!("✓ BenchmarkReport initialized for full flow benchmarking");
            add_to_root_deferred(&global, "benchmark_report");
        } else if benchmark_report {
            godot_error!(
                "BenchmarkReport requires --features use_memory_debugger to be enabled during build"
            );
        }

        if stress_test {
            godot_print!("✓ StressTest initialized for scene loading/unloading stress test");
            spawn_on_root(
                &global,
                "src/tools/stress_test_controller.gd",
                "StressTestController",
            );
        }
        ok
    }
}
