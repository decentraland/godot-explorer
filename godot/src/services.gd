extends Node

## Services autoload.
##
## Owns every long-lived dependency for the explorer: the demoted node services
## (UiSounds, NotificationsManager, ConnectionQualityMonitor, ImpostorCapturer,
## AvatarLODCoordinator — referenced via the still-existing autoload entries
## until the callsite-migration task removes those entries), the GDScript
## service objects (Realm, SceneFetcher, ModalManager, ...) and forwarding
## getters for the Rust GDExtension singletons that live on the DclGlobal base
## class of Global.
##
## Services.bootstrap() is awaited from main.gd (the boot splash). All heavy
## work — service construction, cache clear, telemetry, Rust singleton attach
## — runs here so iOS sees a responsive main loop and the boot is fully
## instrumented (Sentry GODOT-EXPLORER-3B / GH #1532).
##
## Migration is staged. Global retains thin forwarding getters for every
## migrated service so existing `Global.xxx` callsites keep compiling until the
## subsequent rename-sweep tasks redirect them to `Services.xxx`.

signal bootstrap_completed

# --- Packed-scene resource (preloaded; not a node) ---
const EMPTY_PARCEL_PROPS_SCENE: PackedScene = preload(
	"res://assets/empty-scenes/empty_parcel_props.tscn"
)

# --- GDScript node-service aliases populated in bootstrap() ---
var ui_sounds: Node
var notifications_manager: Node
var connection_quality_monitor: Node
var impostor_capturer: Node
var avatar_lod_coordinator: Node

# --- GDScript service objects (constructed in bootstrap()) ---
var raycast_debugger: RaycastDebugger
var scene_fetcher: SceneFetcher
var skybox_time: SkyboxTime
var nft_fetcher: OpenSeaFetcher
var nft_frame_loader: NftFrameStyleLoader
var snapshot: Snapshot
var music_player: MusicPlayer
var preload_assets: PreloadAssets
var locations: Node
var modal_manager: ModalManager
var analytics_controller: AnalyticsController

# Seeds Sentry user / context / tags from realm / scene_fetcher /
# player_identity / comms signals. RefCounted, kept alive by this reference.
var sentry_seeder: SentrySeeder

# Platform attestation orchestrator (App Attest / Play Integrity → mobile-bff session
# token). Owns its own EULA-gated dispatch and the FSM that runs attestation cycles —
# see attestation_service.gd. Other code obtains a token via
# `await Services.attestation.async_get_valid_jwt()`. A Node child of Services so it
# can use timers and native plugin signals across the session lifetime.
var attestation: AttestationService

# Eagerly instantiated (used in Global._ready, which runs before bootstrap).
var deep_link_router := DeepLinkRouter.new()

# --- Rust-inherited forwarding getters ---
# These live on DclGlobal (the Rust base class of Global). Services forwards
# so callsites read uniformly through Services.xxx. The underlying instance
# still lives on Global; construction (e.g. `Global.realm = Realm.new()`) is
# done from bootstrap(). Explicit types are required so GDScript's `:=` type
# inference works at call sites.
var cli: DclCli:
	get:
		return Global.cli
var config: ConfigData:
	get:
		return Global.config
var content_provider: ContentProvider:
	get:
		return Global.content_provider
var scene_runner: SceneManager:
	get:
		return Global.scene_runner
var realm: Realm:
	get:
		return Global.realm
var dcl_tokio_rpc: DclTokioRpc:
	get:
		return Global.dcl_tokio_rpc
var player_identity: PlayerIdentity:
	get:
		return Global.player_identity
var testing_tools: TestingTools:
	get:
		return Global.testing_tools
var portable_experience_controller: PortableExperienceController:
	get:
		return Global.portable_experience_controller
var comms: CommunicationManager:
	get:
		return Global.comms
var avatars: AvatarScene:
	get:
		return Global.avatars
var http_requester:
	get:
		return Global.http_requester
var metrics: Metrics:
	get:
		return Global.metrics
var network_inspector: NetworkInspector:
	get:
		return Global.network_inspector
var scene_inspector_dispatcher: SceneInspectorDispatcher:
	get:
		return Global.scene_inspector_dispatcher
var social_blacklist:
	get:
		return Global.social_blacklist
var social_service:
	get:
		return Global.social_service
var profile_service: ProfileService:
	get:
		return Global.profile_service
var dynamic_graphics_manager:
	get:
		return Global.dynamic_graphics_manager


## Heavy startup work. Awaited by main.gd under the visible splash so iOS sees
## a responsive main loop. Ordered and instrumented so a watchdog termination
## during boot leaves a `last_boot_step` tag pointing at the culprit.
# gdlint:ignore = async-function-name
func bootstrap() -> void:
	BootInstrumentation.mark("services.bootstrap.start")

	Global._dcl_swift_lib_smoke_test()
	BootInstrumentation.mark("services.bootstrap.swift_smoke_test_done")
	await get_tree().process_frame

	# GDScript-only helper objects. Cheap to construct; no scene-tree attach.
	nft_frame_loader = NftFrameStyleLoader.new()
	nft_fetcher = OpenSeaFetcher.new()
	music_player = MusicPlayer.new()
	snapshot = Snapshot.new()
	preload_assets = PreloadAssets.new()

	Global.realm = Realm.new()
	Global.realm.set_name("realm")
	Global.realm.realm_change_failed.connect(Global._on_realm_change_failed_toast)

	Global.dcl_tokio_rpc = DclTokioRpc.new()
	Global.dcl_tokio_rpc.set_name("dcl_tokio_rpc")

	Global.player_identity = PlayerIdentity.new()
	Global.player_identity.set_name("player_identity")
	Global.player_identity.profile_changed.connect(Global._on_player_profile_changed_sync_events)

	Global.testing_tools = TestingTools.new()
	Global.testing_tools.set_name("testing_tool")

	Global.portable_experience_controller = PortableExperienceController.new()
	Global.portable_experience_controller.set_name("portable_experience_controller")

	scene_fetcher = SceneFetcher.new()
	scene_fetcher.set_name("scene_fetcher")

	skybox_time = SkyboxTime.new()
	skybox_time.set_name("skybox_time")

	locations = load("res://src/helpers_components/locations.gd").new()
	locations.set_name("locations")

	modal_manager = load("res://src/ui/components/organisms/modal/modal_manager.gd").new()
	modal_manager.set_name("modal_manager")
	BootInstrumentation.mark("services.bootstrap.core_singletons_created")
	await get_tree().process_frame

	# Ensure the content cache folder exists before clearing — clear runs against
	# this directory and would log an error if it doesn't exist yet (fresh install).
	if not DirAccess.dir_exists_absolute("user://content/"):
		DirAccess.make_dir_absolute("user://content/")

	BootInstrumentation.mark("services.bootstrap.clear_cache_start")
	await Global._async_clear_cache_if_needed()
	BootInstrumentation.mark("services.bootstrap.clear_cache_end")

	Global.session_id = DclConfig.generate_uuid_v4()
	# Skip Segment metrics + Sentry tagging in asset-server mode, or when
	# telemetry is disabled at build time (CI desktop builds use the
	# `disable_telemetry` cargo feature).
	var telemetry_enabled := not Global.cli.asset_server and not DclGlobal.is_telemetry_disabled()

	if telemetry_enabled:
		Global.metrics = Metrics.create_metrics(Global.config.analytics_user_id, Global.session_id)
		Global.metrics.set_debug_level(0)  # 0 off - 1 on
		Global.metrics.set_name("metrics")

		var sentry_user = SentryUser.new()
		sentry_user.id = Global.config.analytics_user_id
		SentrySDK.set_tag("dcl_session_id", Global.session_id)

		# RefCounted, kept alive by the Services reference. Seeds Sentry user /
		# context / tags from the runtime signals exposed by the subsystems
		# created above. No scene-tree presence.
		sentry_seeder = SentrySeeder.new()
		sentry_seeder.setup()
	BootInstrumentation.mark("services.bootstrap.telemetry_initialized")

	# Instantiate the 5 demoted node services as children of Services. Each
	# script's _ready is intentionally a no-op; real setup happens in their
	# initialize_async() which we await here so the boot order is deterministic.
	ui_sounds = load("res://src/helpers_components/ui_sounds.gd").new()
	ui_sounds.name = "UiSounds"
	add_child(ui_sounds)
	await ui_sounds.initialize_async()
	await get_tree().process_frame

	notifications_manager = load("res://src/notifications_manager.gd").new()
	notifications_manager.name = "NotificationsManager"
	add_child(notifications_manager)
	await notifications_manager.initialize_async()
	await get_tree().process_frame

	connection_quality_monitor = load("res://src/connection_quality_monitor.gd").new()
	connection_quality_monitor.name = "ConnectionQualityMonitor"
	add_child(connection_quality_monitor)
	await connection_quality_monitor.initialize_async()
	await get_tree().process_frame

	impostor_capturer = (
		load("res://src/decentraland_components/avatar/impostor/impostor_capturer.gd").new()
	)
	impostor_capturer.name = "ImpostorCapturer"
	add_child(impostor_capturer)

	avatar_lod_coordinator = (
		load("res://src/decentraland_components/avatar/impostor/avatar_lod_coordinator.gd").new()
	)
	avatar_lod_coordinator.name = "AvatarLODCoordinator"
	add_child(avatar_lod_coordinator)

	BootInstrumentation.mark("services.bootstrap.node_services_initialized")
	await get_tree().process_frame

	# Attach the singletons to the scene tree. We own the timing here, so use
	# direct add_child instead of call_deferred and yield every few attaches so
	# the main loop renders + the iOS watchdog stays happy.
	# NOTE: still attaching under /root, not under Services. Moving to
	# /root/Services/... is gated on the Rust patch in global_get_node_helper.rs
	# and will land together in a later task.
	add_child(Global.cli)
	add_child(music_player)
	add_child(scene_fetcher)
	add_child(skybox_time)
	add_child(locations)
	add_child(modal_manager)
	await get_tree().process_frame
	BootInstrumentation.mark("services.bootstrap.attach_batch_1_done")

	add_child(Global.content_provider)
	add_child(Global.scene_runner)
	add_child(Global.realm)
	add_child(Global.dcl_tokio_rpc)
	add_child(Global.player_identity)
	await get_tree().process_frame
	BootInstrumentation.mark("services.bootstrap.attach_batch_2_done")

	add_child(Global.comms)
	add_child(Global.avatars)
	add_child(Global.portable_experience_controller)
	add_child(Global.testing_tools)
	await get_tree().process_frame
	BootInstrumentation.mark("services.bootstrap.attach_batch_3_done")

	if Global.metrics != null:
		add_child(Global.metrics)
		# Fire install attribution once per install (Android only).
		if Global.is_android() and not Global.config.install_referrer_sent:
			Global.metrics.track_install_referrer.call_deferred()
			Global.config.install_referrer_sent = true
			Global.config.save_to_settings_file()
		# All Firebase/Segment orchestration lives in AnalyticsController — see its docstring.
		# RefCounted, kept alive by this strong reference. No scene-tree presence by default;
		# spawns a transient Timer under Services only while polling for first_move_in_world.
		analytics_controller = AnalyticsController.new()
		analytics_controller.setup()

		# iOS only: report the StoreKit environment (production/sandbox) to
		# analytics once at startup. StoreKit's environment is fixed by how the
		# binary was distributed and can't be chosen by the app, so this is the
		# ground truth for which IAP backend a device will hit. We ship this
		# BEFORE any purchase flow to validate in prod that real App Store
		# installs report `production`. Deferred so it runs after every autoload
		# (incl. Iap) is in the tree. See docs/iap-zone-submission/.
		if Global.is_ios():
			Iap.report_environment_to_analytics.call_deferred()

	# Platform attestation: needs to be a Node (uses timers + native plugin signals).
	# The service self-gates on EULA acceptance and caches the issued session token on disk.
	attestation = AttestationService.new()
	add_child(attestation)

	add_child(Global.network_inspector)
	add_child(Global.scene_inspector_dispatcher)
	add_child(Global.social_blacklist)
	add_child(Global.dynamic_graphics_manager)

	if "memory_debugger" in Global:
		add_child(Global.memory_debugger)
	BootInstrumentation.mark("services.bootstrap.attach_batch_4_done")
	await get_tree().process_frame

	# Initialize BenchmarkReport singleton if benchmarking is enabled (requires use_memory_debugger feature)
	if Global.cli.benchmark_report and "benchmark_report" in Global:
		print("✓ BenchmarkReport initialized for full flow benchmarking")
		add_child(Global.benchmark_report)

		# Add benchmark flow controller to orchestrate the full benchmark flow
		var benchmark_flow_controller = load("res://src/tools/benchmark_flow_controller.gd").new()
		benchmark_flow_controller.set_name("BenchmarkFlowController")
		add_child(benchmark_flow_controller)
	elif Global.cli.benchmark_report:
		push_error(
			"BenchmarkReport requires --features use_memory_debugger to be enabled during build"
		)

	# Add stress test controller if stress testing is enabled
	if Global.cli.stress_test:
		print("✓ StressTest initialized for scene loading/unloading stress test")
		var stress_test_controller = load("res://src/tools/stress_test_controller.gd").new()
		stress_test_controller.set_name("StressTestController")
		add_child(stress_test_controller)

	Global._init_dynamic_graphics_manager()
	BootInstrumentation.mark("services.bootstrap.dynamic_graphics_initialized")

	var custom_importer = load("res://src/logic/custom_gltf_importer.gd").new()
	GLTFDocument.register_gltf_document_extension(custom_importer)
	BootInstrumentation.mark("services.bootstrap.gltf_importer_registered")

	if Global.cli.raycast_debugger:
		Global.set_raycast_debugger_enable(true)
	BootInstrumentation.mark("services.bootstrap.raycast_debugger_done")

	if Global.cli.network_debugger:
		Global.network_inspector.set_is_active(true)
		Global.open_network_inspector_ui()
	else:
		Global.network_inspector.set_is_active(false)
	BootInstrumentation.mark("services.bootstrap.network_inspector_configured")

	# Yield before the synchronous primitive-shape allocation so the watchdog
	# sees a rendered frame even if Rust takes time to upload the meshes.
	await get_tree().process_frame
	DclMeshRenderer.init_primitive_shapes()
	BootInstrumentation.mark("services.bootstrap.mesh_primitives_initialized")
	BootInstrumentation.mark("services.bootstrap.end")
	bootstrap_completed.emit()
