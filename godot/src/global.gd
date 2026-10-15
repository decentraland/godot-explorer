extends DclGlobal

signal on_menu_open
signal on_menu_close
signal loading_started
signal loading_finished
signal change_parcel(new_parcel: Vector2i)
signal open_profile_by_avatar(avatar: DclAvatar)
signal open_profile_by_address(address: String)
signal on_chat_message(address: String, message: String, timestamp: float)
signal change_virtual_keyboard(height: int)
signal notification_clicked(notification: Dictionary)
signal notification_received(notification: Dictionary)
signal open_chat
signal close_chat
signal open_notifications_panel
signal open_settings
signal open_settings_panel
signal open_backpack(on_emotes: bool)
signal open_discover
signal open_discover_panel  # Landscape-only: Discover side panel docked by the navbar
## Carries the entry point, reported by Menu.async_show_credits.
signal open_credits(source: String)
signal open_own_profile
signal open_profile_editor
signal open_navbar_silently
signal close_menu
signal close_navbar
signal close_combo
signal delete_account
signal upgrade_to_otp
## Fired by GuestUpgradeCard after the thirdweb network check resolves so other
## UI (e.g. UpgradeBadge) can sync without duplicating the network call.
signal guest_upgrade_state_refreshed(is_upgraded: bool)
## Sync settings "Hide UI" checkbox with explorer session state (no config persistence).
signal session_hide_ui_toggle_sync(pressed: bool)
## Sync settings "Hide View Profile" / "Hide World Interactions" / etc. checkboxes.
signal session_hide_ui_options_sync(
	hide_view_profile: bool,
	hide_world_interactions: bool,
	hide_player_names: bool,
	hide_scene_ui: bool
)
signal camera_mode_set(camera_mode: Global.CameraMode)
signal camera_mode_block_changed(blocked: bool)
signal favorite_destination_set
signal orientation_changed(is_portrait: bool)
signal chat_write_mode_changed(is_writing: bool)
signal device_support_status_resolved

enum CameraMode { FIRST_PERSON = 0, THIRD_PERSON = 1, CINEMATIC = 2 }

# Only for debugging purpose, Godot editor doesn't include a custom param debugging
const FORCE_TEST = false
const FORCE_TEST_ARG = "[[52,-52],[52,-54],[52,-56],[52,-58],[52,-60],[52,-62],[52,-64],[52,-66],[52,-68],[54,-52],[54,-54],[54,-56],[54,-58],[54,-60]]"
const FORCE_TEST_REALM = "https://sdk-team-cdn.decentraland.org/ipfs/goerli-plaza-main-latest"
const FORCE_TEST_LOCATION = Vector2i(54, -55)
#const FORCE_TEST_ARG = "[[52,-56]]"
# const FORCE_TEST_REALM = "http://localhost:8000"

const FORCE_DEEPLINK = ""
#const FORCE_DEEPLINK = "decentraland://open?rust-log=dclgodot::analytics::metrics=debug,warn"
#const FORCE_DEEPLINK = "decentraland://open?dclenv=zone&fake-owned-wearables=urn:decentraland:amoy:collections-v2:0x81004ea82f4af8337e357bef49cc746fce881dee:5"

# DEBUG ONLY. When `true`, get_device_anchor_id() ignores the platform-native
# device anchor (Android SSAID / iOS Keychain UUID) and uses the per-install
# UUID in `user://device_anchor.txt` on EVERY platform, so the guest wallet is
# tied to the app's user data: deleting `user://` (clear app data / reinstall —
# or the "RESET GUEST WALLET" debug button in the lobby) mints a fresh anchor
# and a fresh wallet. When `false` the original shipping behavior returns: the
# device-bound native anchor that survives reinstall.
#
# HARD-GATED to non-production: get_device_anchor_id() (and the lobby debug
# button) only honor this flag when `not is_production()`, so a release cut from
# main always uses the device-bound native anchor even if this is left `true`.
const DEBUG_GUEST_ROTATE_ANCHOR_ID: bool = false

# Increase this value for new terms and conditions
const TERMS_AND_CONDITIONS_VERSION: int = 1

# Increase this value when local assets cache format changes (invalidates cache)
# v5: emote .tscn baked before the case-insensitive prop lookup lack prop geometry.
# v7: optimized assets are raw {hash}.opt.scn/.opt.res files from the /v6 bucket.
const LOCAL_ASSETS_CACHE_VERSION: int = 7

# On-disk guest identity artifacts, owned by Rust (keep in sync with
# lib/src/auth/device_anchor.rs + thirdweb_guest.rs) plus the mobile-BFF
# attestation session. Wiped when a non-upgraded guest deletes their account
# (issue #2335) and by the non-prod lobby "reset guest wallet" debug button.
const GUEST_DEVICE_STORAGE_FILES: Array[String] = [
	"user://device_anchor.txt",
	"user://thirdweb_session.json",
	"user://attest_session.json",
]

## Global classes (singleton pattern)

var raycast_debugger: RaycastDebugger

var scene_fetcher: SceneFetcher
var skybox_time: SkyboxTime = null

var nft_fetcher: OpenSeaFetcher
var nft_frame_loader: NftFrameStyleLoader

var snapshot: Snapshot
var music_player: MusicPlayer
var locations: Node

var standalone = false

var network_inspector_window: Window = null

var last_emitted_height: int = 0
var current_height: int = -1
var previous_height: int = -1
var previous_height_2: int = -1

var deep_link_obj: DclParseDeepLink = DclParseDeepLink.new()
var deep_link_url: String = ""
var deep_link_router := DeepLinkRouter.new()
var graphics_autotune := GraphicsAutotune.new()

## true when the env came from --dclenv or a deeplink dclenv=… (incl. dclenv=org).
## When true, Iap's sandbox auto-switch must NOT override the explicit choice.
var dcl_env_explicit: bool = false

var player_camera_node: DclCamera3D
var current_camera_mode: CameraMode = CameraMode.THIRD_PERSON
var camera_mode_blocked: bool = false
var session_id: String

# Fixed width (canvas px) of the chat's notifications column, reported by the chat so the
# scene interactable area can be computed without hardcoding it. Stable per device.
var chat_notifications_width: int = 0

# Orchestrates the Firebase / Segment glue (EULA gate, login suppression on session recovery,
# first_move_in_world detection). Instantiated after `metrics` is created.
var analytics_controller: AnalyticsController = null

# Platform attestation orchestrator (App Attest / Play Integrity → mobile-bff session
# token). Owns its own EULA-gated dispatch and the FSM that runs attestation cycles —
# see attestation_service.gd. Other code obtains a token via
# `await Global.attestation.async_get_valid_jwt()`. Instantiated in _ready as a Node
# child of Global so it can use timers and signals across the session lifetime.
var attestation: AttestationService = null

# Remote feature flags fetched from the mobile-bff at startup. Query with
# `Global.feature_flags.is_enabled("flag-name")`; comms-affecting flags are
# applied automatically when the response arrives — see feature_flags.gd.
var feature_flags: FeatureFlags = null

var _is_portrait: bool = true

# Opt-in, set by a `decentraland://open?enable-upgraded-deletion=true` deeplink
# (see deep_link_router.gd). Gates the account-deletion flow so an UPGRADED
# (email-linked) guest can be fully deleted via a double unlink — see
# account_deletion_popup.gd and is_upgraded_deletion_enabled(). Sticky for the
# session once seen; only takes effect on a NON-production build.
var _enable_upgraded_deletion: bool = false

# Scene Inspector bridge, created lazily at boot when a target is configured.
var _scene_inspector_bridge: Node = null

# Logging self-test trigger (cli flag or deeplink); runs at most once.
var _logging_selftest := LoggingSelftest.new()

# Cached reference to SafeAreaPresets (loaded dynamically to avoid export issues)
var _safe_area_presets: GDScript = null

var _safe_margin_debug_overlay: SafeMarginDebugOverlay = null

# Startup instrumentation timestamp (set once at load time)
var _startup_time: int = Time.get_ticks_msec()

## Coalescer for GltfContainer load-timeouts. Lazy-init child node.
## Adding `_process` directly on DclGlobal triggers a Godot Vulkan crash —
## the coalescer must live on a separate Node to keep `_process` off the
## autoload itself.
var _gltf_load_timeout_coalescer: Node = null

# Guards sign_out() against re-entrancy. comms.disconnect(true) emits the
# player_identity.logout signal deferred, which can route back into sign_out() a
# frame later; without this the whole teardown + scene swap would run twice.
# Cleared at the end of sign_out, once the session is gone.
var _signing_out: bool = false

# Guards return_to_discover() against re-entrancy (e.g. a double tap on the Dev
# Tools button before the deferred scene swap runs). Cleared in menu._ready once
# we're back on the standalone Discover screen.
var _returning_to_discover: bool = false


func is_xr() -> bool:
	return OS.has_feature("xr") or get_viewport().use_xr


func is_emulating_safe_area() -> bool:
	return should_emulate_ios() or should_emulate_android()


## Emulation flags are desktop-preview only; ignore them on a real device (the
## editor forwards its run args to one-click device deploys, which would
## otherwise force the emulated window size on the phone).
func _is_native_mobile() -> bool:
	return is_android() or is_ios()


func should_emulate_ios() -> bool:
	return cli.emulate_ios and not _is_native_mobile()


func should_emulate_android() -> bool:
	return cli.emulate_android and not _is_native_mobile()


## True when GP benchmark was triggered, either via desktop CLI (`--gp-benchmark`)
## or mobile deep link (`decentraland://open?gp-benchmark=true&...`).
func is_gp_benchmark() -> bool:
	return cli.gp_benchmark or (deep_link_obj != null and deep_link_obj.gp_benchmark)


## Lazy-init the GltfContainer load-timeout coalescer. Replaces the
## per-container Timer (~1419 in Genesis Plaza). Called from
## gltf_container.gd; created on first use, persists for the app's lifetime.
func get_gltf_load_timeout_coalescer() -> Node:
	if _gltf_load_timeout_coalescer == null:
		var coalescer_script = load(
			"res://src/decentraland_components/gltf_load_timeout_coalescer.gd"
		)
		_gltf_load_timeout_coalescer = coalescer_script.new()
		_gltf_load_timeout_coalescer.name = "GltfLoadTimeoutCoalescer"
		add_child(_gltf_load_timeout_coalescer)
	return _gltf_load_timeout_coalescer


func _get_safe_area_presets() -> GDScript:
	if _safe_area_presets == null:
		_safe_area_presets = load("res://assets/no-export/safe_area_presets.gd")
	return _safe_area_presets


func _generated_deeplink() -> String:
	# Baked launch params from `cargo run -- run/build --deeplink|--log-stream`.
	# Gitignored and optional; absent on a fresh checkout until the first build.
	# Never honored in a production build: the iOS self-hosted runner exports with
	# `clean: false`, so a stale build_config.gd left by a prior debug build could
	# otherwise leak a baked deeplink into a prod IPA.
	if is_production():
		return ""
	var gen_path := "res://src/generated/build_config.gd"
	if not ResourceLoader.exists(gen_path):
		return ""
	var gen_script: GDScript = load(gen_path)
	if gen_script == null:
		return ""
	return gen_script.get_script_constant_map().get("DEEPLINK", "")


func get_safe_area() -> Rect2i:
	if should_emulate_ios():
		var presets := _get_safe_area_presets()
		return presets.get_ios_safe_area(is_orientation_portrait(), get_window().size)
	if should_emulate_android():
		var presets := _get_safe_area_presets()
		return presets.get_android_safe_area(is_orientation_portrait(), get_window().size)
	return DisplayServer.get_display_safe_area()


func _instantiate_phone_frame_overlay() -> void:
	var overlay_scene = load("res://assets/no-export/phone_frame_overlay.tscn")
	if overlay_scene:
		var overlay = overlay_scene.instantiate()
		add_child(overlay)


## Vibrate handheld device
func send_haptic_feedback(duration_ms: int = 20, amplitude: float = -1.0) -> void:
	if is_mobile():
		Input.vibrate_handheld(duration_ms, amplitude)


# gdlint: ignore=async-function-name
func _ready():
	print("[Startup] global._ready start: %dms" % (Time.get_ticks_msec() - _startup_time))
	# Bench-only: uncap FPS / disable vsync before any code path can re-pin
	# Engine.max_fps. Real users keep their saved cap + vsync; mobile bench
	# uncaps via gp_benchmark_runner at the load->settling transition.
	if cli.bench_mode:
		Engine.max_fps = 0
		DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)
		OS.low_processor_usage_mode = false
	# Use CLI singleton for command-line args
	if cli.force_mobile:
		_set_is_mobile(true)

	# Handle safe area emulation (enables mobile mode and resizes window)
	if should_emulate_ios():
		_set_is_mobile(true)
		var presets := _get_safe_area_presets()
		var target_size: Vector2i = presets.get_ios_window_size(is_orientation_portrait())
		get_window().size = target_size
		get_window().move_to_center()
		_instantiate_phone_frame_overlay()
	elif should_emulate_android():
		_set_is_mobile(true)
		var presets := _get_safe_area_presets()
		var target_size: Vector2i = presets.get_android_window_size(is_orientation_portrait())
		get_window().size = target_size
		get_window().move_to_center()
		_instantiate_phone_frame_overlay()

	if cli.landscape and (should_emulate_ios() or should_emulate_android()):
		set_orientation_landscape()

	# Handle fake deep link. Precedence: --fake-deeplink CLI arg > generated
	# build_config (baked by `cargo run -- run/build --deeplink|--log-stream`) >
	# FORCE_DEEPLINK constant. The generated path works on mobile/TestFlight where
	# no CLI args are available.
	var fake_deeplink = cli.fake_deeplink
	if fake_deeplink.is_empty():
		fake_deeplink = _generated_deeplink()
	if fake_deeplink.is_empty() and not FORCE_DEEPLINK.is_empty():
		fake_deeplink = FORCE_DEEPLINK
	if not fake_deeplink.is_empty():
		deep_link_url = fake_deeplink
		deep_link_obj = DclParseDeepLink.parse_decentraland_link(fake_deeplink)
		print(
			"[DEEPLINK] Parsed fake deep_link_obj: location=",
			deep_link_obj.location,
			" realm=",
			deep_link_obj.realm,
			" preview=",
			deep_link_obj.preview
		)
		print("[DEEPLINK] All params: ", deep_link_obj.params)

		# Apply rust-log from deeplink params
		var rust_log_value = deep_link_obj.params.get("rust-log", "")
		if not rust_log_value.is_empty():
			print("[DEEPLINK] Found rust-log param: ", rust_log_value)
			DclGlobal.set_rust_log_filter(rust_log_value)
		else:
			print("[DEEPLINK] No rust-log param in deeplink")

		deep_link_router._apply_optimized_content_base_url(deep_link_obj)

		# Pulse transport params (this fake-deeplink path doesn't route through
		# deep_link_router.process_deep_link).
		deep_link_router._apply_comms_deeplink_params(deep_link_obj)

		print("[DEEPLINK] safemargindebug=", deep_link_obj.safe_margin_debug)
		if deep_link_obj.safe_margin_debug:
			set_safe_margin_debug_enable(true)

		# scene-stats overlay is applied by explorer.gd:_update_scene_stats_ui();
		# just log here so CLI/QA can confirm the param parsed.
		print("[DEEPLINK] scene-stats=", deep_link_obj.scene_stats)

	# Connect to iOS deeplink signal
	if DclIosPlugin.is_available():
		var dcl_ios_singleton = Engine.get_singleton("DclGodotiOS")
		if dcl_ios_singleton:
			# Warm path only; iOS reads the cold-start link in _notification(READY).
			dcl_ios_singleton.deeplink_received.connect(deep_link_router.process_deep_link)

	_dcl_swift_lib_smoke_test()
	# The services every boot needs (config, realm, identity, ...) are created in Rust.
	DclBootstrap.create_core_services(self)

	var args = cli.get_all_args()
	if args.size() == 1 and args[0].begins_with("res://"):
		if args[0] != "res://src/dcl_root.tscn":
			self.standalone = true

	if FORCE_TEST:
		Global.testing_scene_mode = true

	config.load_from_settings_file()

	# Restore profile particle budgets + avatar dust gate: the Rust atomics and
	# the GDScript toggle default to full-budget, and nothing re-applies them
	# until the user picks a profile again.
	GraphicSettings.apply_particle_quality(config.particle_quality)
	AvatarAnimHelpers.apply_particles_enabled(config.particle_quality > 0)

	# Campaign token (#2670). Deliberately after the config load: the fake/baked deeplink is
	# parsed further up in _ready, long before ConfigData exists, so capturing there wrote to
	# a config that was then replaced by the one read from disk. deep_link_obj is eagerly
	# constructed, so this is a no-op when no deeplink carried a token. The live mobile path
	# captures from deep_link_router instead, which already runs well after this point.
	deep_link_router._capture_debug_guest_rotate(deep_link_obj)
	deep_link_router._capture_campaign_token(deep_link_obj)

	# Resolve the UI language before any scene renders. Godot picks the OS locale at boot, which
	# would surface a partially-translated locale the moment its .po has content; LocaleSettings
	# gates on SUPPORTED_LOCALES so an incomplete language is never selected (see #270, #2062).
	LocaleSettings.apply_locale()
	# Bench-only: keep limit_fps at NO_LIMIT after the settings file load (which
	# would otherwise restore a saved FPS_18/FPS_30 cap) so no later
	# `apply_fps_limit()` re-pins the engine. Real users keep their saved cap.
	if cli.bench_mode:
		config.limit_fps = ConfigData.FpsLimitMode.NO_LIMIT
		Engine.max_fps = 0
		DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)

	# Initialize environment. Precedence: --dclenv CLI flag > deeplink dclenv param > "org".
	var env := "org"
	var env_source := "default"
	if not cli.dcl_env.is_empty():
		env = cli.dcl_env
		env_source = "--dclenv"
	elif not deep_link_obj.dclenv.is_empty():
		env = deep_link_obj.dclenv
		env_source = "deeplink"
	DclGlobal.set_dcl_environment(env)
	dcl_env_explicit = env_source != "default"
	if env != "org":
		print("[GLOBAL] Environment set to: ", env, " (source: ", env_source, ")")

	# Dev/testing: disable profile deploys so fake-owned wearables never publish.
	if deep_link_obj.disable_profile_deploy:
		ProfileService.set_deploy_disabled(true)
		print("[GLOBAL] Profile deploy DISABLED (local-only profile updates)")

	self.realm.realm_access_denied.connect(_on_realm_access_denied)

	# Ensure the content cache folder exists before clearing — clear runs against
	# this directory and would log an error if it doesn't exist yet (fresh install).
	if not DirAccess.dir_exists_absolute("user://content/"):
		DirAccess.make_dir_absolute("user://content/")

	# Clear cache if needed (startup flag or version changed) - await completion
	print(
		"[Startup] global._async_clear_cache start: %dms" % (Time.get_ticks_msec() - _startup_time)
	)
	await _async_clear_cache_if_needed()
	print("[Startup] global._async_clear_cache end: %dms" % (Time.get_ticks_msec() - _startup_time))

	# The Rust test runner and the floating-islands benchmark boot without the app services.
	if DclBootstrap.start_special_profile(self):
		return

	# Genesis Plaza profiling benchmark (issue #1862). Lives alongside the full
	# explorer flow: it primes realm/parcel, then samples once the scene loads.
	# Skip if the deeplink path already spawned the runner (mobile cold-start race).
	if is_gp_benchmark() and get_node_or_null("GPBenchmarkRunner") == null:
		print("Running Genesis Plaza Benchmark...")
		var gp_runner = load("res://src/tools/gp_benchmark_runner.gd").new()
		gp_runner.set_name("GPBenchmarkRunner")
		add_child(gp_runner)

	if not DirAccess.dir_exists_absolute("user://content/"):
		DirAccess.make_dir_absolute("user://content/")

	session_id = DclConfig.generate_uuid_v4()
	# Skip Segment metrics + Sentry tagging in asset-server mode, or when
	# telemetry is disabled at build time (CI desktop builds use the
	# `disable_telemetry` cargo feature).
	var telemetry_enabled := not cli.asset_server and not DclGlobal.is_telemetry_disabled()

	DclBootstrap.create_app_services(self, session_id, telemetry_enabled)
	if self.metrics != null:
		# Fire install attribution once per install (Android only).
		if self.is_android() and not self.config.install_referrer_sent:
			self.metrics.track_install_referrer.call_deferred()
			self.config.install_referrer_sent = true
			self.config.save_to_settings_file()
	DclBootstrap.create_extra_services(self)

	# Initialize dynamic graphics manager after config is loaded. Self-skips in
	# bench mode (the function early-returns on cli.bench_mode), so bench results
	# stay comparable while production keeps thermal-cap + adaptive downgrade.
	graphics_autotune.init_dynamic_graphics.call_deferred()

	var custom_importer = load("res://src/logic/custom_gltf_importer.gd").new()
	GLTFDocument.register_gltf_document_extension(custom_importer)

	if cli.raycast_debugger:
		set_raycast_debugger_enable(true)

	self.network_inspector.set_is_active(cli.network_debugger)

	# Logging self-test (debug aid). When --test-logging / ?test-logging=true is
	# set, exercise every logging form in every stack so we can confirm each pipes
	# into the unified channel + Sentry. Checked here (cli, e.g. iOS baked arg) AND
	# on deeplink arrival (Android delivers it async, after _ready), run once.
	_logging_selftest.maybe_run()
	deep_link_router.deep_link_received.connect(_logging_selftest.maybe_run)

	DclMeshRenderer.init_primitive_shapes()
	print("[Startup] global._ready end: %dms" % (Time.get_ticks_msec() - _startup_time))


# Smoke test for the Swift GDExtension. Runs only on iOS where DclSwiftLibPlugin
# can actually reach the underlying Swift class; no-ops on every other platform.
func _dcl_swift_lib_smoke_test() -> void:
	if not DclSwiftLibPlugin.is_available():
		return
	print(
		"[DclSwiftLib] ping() -> ",
		DclSwiftLibPlugin.ping(),
		" | version() -> ",
		DclSwiftLibPlugin.version()
	)


## Async helper to clear cache and wait for completion before anything loads.
func _async_clear_cache_if_needed() -> void:
	# Skip cache clearing in asset server mode - we want to preserve the download cache
	if cli.asset_server:
		return

	var should_clear_startup = cli.clear_cache_startup
	var version_changed = config.local_assets_cache_version != Global.LOCAL_ASSETS_CACHE_VERSION

	if should_clear_startup or version_changed:
		if should_clear_startup:
			prints("Clear cache startup!")
		if version_changed:
			prints("Local assets cache version changed, clearing cache!")

		var clear_promise = Global.content_provider.clear_cache_folder()
		await PromiseUtils.async_awaiter(clear_promise)
		prints("Cache cleared successfully!")

		if version_changed:
			config.local_assets_cache_version = Global.LOCAL_ASSETS_CACHE_VERSION
			config.save_to_settings_file()


func set_raycast_debugger_enable(enable: bool):
	var current_enabled = is_instance_valid(raycast_debugger)
	if current_enabled == enable:
		return

	if enable:
		raycast_debugger = RaycastDebugger.new()
		add_child(raycast_debugger)
	else:
		remove_child(raycast_debugger)
		raycast_debugger.queue_free()
		raycast_debugger = null


func set_safe_margin_debug_enable(enable: bool) -> void:
	var current_enabled = is_instance_valid(_safe_margin_debug_overlay)
	if current_enabled == enable:
		return

	if enable:
		_safe_margin_debug_overlay = (load("res://src/tool/safe_margin_debug_overlay.gd").new())
		add_child(_safe_margin_debug_overlay)
		print("[SafeMarginDebug] overlay instantiated")
	else:
		remove_child(_safe_margin_debug_overlay)
		_safe_margin_debug_overlay.queue_free()
		_safe_margin_debug_overlay = null


func add_raycast(id: int, time: float, from: Vector3, to: Vector3) -> void:
	if is_instance_valid(raycast_debugger):
		raycast_debugger.add_raycast(id, time, from, to)


func get_explorer() -> Explorer:
	var explorer = get_node_or_null("/root/explorer")
	if is_instance_valid(explorer):
		return explorer
	return null


## Single canonical sign-out / logout entry point. Tears down the live session —
## explorer signals & timers, social streams, every running DCL scene, comms,
## realm, scene-fetcher state and the Rust player identity — then returns to a
## fresh lobby so the next login starts as if the app had restarted. Every UI
## logout button and the disconnect handler funnel through here.
## Deletes every on-disk guest identity artifact so the next "Play as guest"
## derives a BRAND-NEW wallet instead of rehydrating the old one, and drops the
## cached guest look. Returns how many files were removed. This does NOT end the
## session — pair it with [sign_out] (see the guest account-deletion flow) to
## actually log the user out. Shared by that flow and the lobby debug reset.
func clear_guest_device_storage() -> int:
	var removed := 0
	for path in GUEST_DEVICE_STORAGE_FILES:
		if FileAccess.file_exists(path):
			var err := DirAccess.remove_absolute(path)
			if err == OK:
				removed += 1
			else:
				push_error("[guest] delete: failed to remove %s (err %d)" % [path, err])
	# The cached guest profile would otherwise be reused by the fresh wallet.
	get_config().guest_profile = {}
	get_config().save_to_settings_file()
	return removed


## True only when deletion of an UPGRADED (email/social-linked) guest is allowed:
## the `enable-upgraded-deletion=true` deeplink was seen AND this is a
## NON-production build. Off by default and hard-disabled in prod, so a
## recoverable account is never force-deleted on a release cut; the upgraded path
## does a double unlink (guest + email) — see account_deletion_popup.gd.
func is_upgraded_deletion_enabled() -> bool:
	return _enable_upgraded_deletion and not is_production()


func sign_out() -> void:
	if _signing_out:
		return
	_signing_out = true

	# 1. Tear down the live Explorer first, while it is still in the tree, so its
	#    autoload-signal callbacks and retry timers are severed before the steps
	#    below re-emit any of those signals (orientation, realm clear, scene kill)
	#    or free the Explorer node.
	var explorer := get_explorer()
	if explorer != null:
		explorer.prepare_for_logout()

	# 2. Stop the session's background work.
	# The analytics first-move poll (a Timer under this autoload) reads
	# scene_runner.player_body_node; left running it would poll the freed Player
	# after the swap, panicking the #[var] getter. It restarts on the next login.
	if analytics_controller != null:
		analytics_controller.cancel_first_move_poll()
	social_blacklist.clear_blocked()
	social_blacklist.clear_muted()

	# 3. Kill every running DCL scene. kill_all_scenes() marks them ToKill; the
	#    SceneManager reaps them in the background (reap_dying_scenes runs even
	#    while the lobby pauses the runner), tearing down their V8/Deno threads.
	scene_runner.kill_all_scenes()

	# 4. Reset realm and scene-fetcher state so nothing from the old session leaks
	#    into the next login. async_clear_realm() runs synchronously here (it only
	#    zeroes realm fields and kills scenes — no network await before returning).
	realm.async_clear_realm()
	scene_fetcher.reset_for_logout()

	# 5. Close comms (MainRoom / SceneRoom / LiveKit) AND clear the Rust player
	#    identity (wallet + ephemeral keys). disconnect(true) is idempotent: a
	#    second logout() no-ops once the address is already cleared.
	comms.disconnect(true)

	# 6. Erase the persisted session (ephemeral keys / wallet bytes) from disk.
	#    Everything else (graphics, last realm, content cache) is intentionally
	#    kept — this is a sign-out, not a factory reset.
	#
	#    Note: the scene_runner's player-node handles (and base_ui) are deliberately
	#    left as-is — once the Explorer is freed they report is_instance_valid()==false
	#    and the SceneManager's existing guards skip them; explorer._ready() reassigns
	#    them on the next login.
	get_config().session_account = {}
	# Forget the previous identity so the next account starts fresh — otherwise
	# the prior profile/avatar lingers in memory and leaks into the new account's
	# avatar editor (issue #1658). Also drop the saved guest look on explicit
	# sign-out to close the guest->guest leak vector.
	get_config().guest_profile = {}
	player_identity.reset_identity()
	get_config().save_to_settings_file()

	# Lobby/login is portrait-only; reset orientation so logging out from a
	# landscape screen (e.g. settings panel) doesn't strand the user there.
	set_orientation_portrait()

	# 8. Detach scene_runner.base_ui from the dying Explorer so freeing it can't
	#    free UI nodes Rust still references — a cache-hot scene spawn racing the
	#    next explorer._ready() would panic on the freed control
	#    (GODOT-EXPLORER-1DY). The orphan is freed by the next recreate_base_ui().
	scene_runner.detach_base_ui()

	_signing_out = false


func explorer_has_focus() -> bool:
	var explorer = get_explorer()
	if explorer == null:
		return false

	return explorer.ui_root.has_focus()


func explorer_grab_focus() -> void:
	var explorer = get_explorer()
	if explorer == null:
		return
	# A scene text input being typed in keeps focus (and its keyboard) until it blurs.
	if engine.is_text_input_focused():
		return

	explorer.ui_root.grab_focus.call_deferred()


func explorer_release_focus() -> void:
	var explorer = get_explorer()
	if explorer == null:
		return

	explorer.ui_root.release_focus.call_deferred()


func capture_mouse():
	var explorer = get_node_or_null("/root/explorer")
	if is_instance_valid(explorer):
		explorer.capture_mouse()
	elif DisplayServer.has_feature(DisplayServer.FEATURE_MOUSE):
		Input.set_mouse_mode(Input.MOUSE_MODE_CAPTURED)


func release_mouse():
	var explorer = get_node_or_null("/root/explorer")
	if is_instance_valid(explorer):
		explorer.release_mouse()
	elif DisplayServer.has_feature(DisplayServer.FEATURE_MOUSE):
		Input.set_mouse_mode(Input.MOUSE_MODE_CAPTURED)


func open_webview_url(url):
	if DclIosPlugin.is_available():
		DclIosPlugin.open_webview_url(url)
	elif DclAndroidPlugin.is_available():
		DclAndroidPlugin.open_custom_tab_url(url)
	else:
		OS.shell_open(url)


func open_url(url: String, use_webkit: bool = false):
	if use_webkit and not Global.is_xr():
		if DclIosPlugin.is_available():
			DclIosPlugin.open_safari_auth_url(url)
		elif DclAndroidPlugin.is_available():
			if "provider=wallet-connect" in url:
				DclAndroidPlugin.open_webview(url, "")  # FOR WALLET CONNECT
			else:
				DclAndroidPlugin.open_custom_tab_url(url)  # FOR SOCIAL
		else:
			OS.shell_open(url)
	else:
		if DclAndroidPlugin.is_available():
			DclAndroidPlugin.show_decentraland_mobile_toast()
			DclAndroidPlugin.open_url(url)
		else:
			OS.shell_open(url)


func set_orientation_landscape():
	# While tearing down the session toward a portrait-only screen (sign-out ->
	# lobby, or return-to-discover -> Discover), ignore stray landscape flips. The
	# explorer's loading-screen/chat machinery reacts to the realm being cleared
	# during teardown and would otherwise flip back to landscape right before the
	# swap, stranding the lobby/login in landscape (issue #2508).
	if _signing_out or _returning_to_discover:
		return
	if engine.is_host_driven():
		_follow_host_orientation()
		return
	# Set orientation BEFORE changing window size so listeners get correct value
	_is_portrait = false
	if Global.is_mobile() and !Global.is_virtual_mobile():
		DisplayServer.screen_set_orientation(DisplayServer.SCREEN_SENSOR_LANDSCAPE)
	elif should_emulate_ios():
		var presets := _get_safe_area_presets()
		get_window().size = presets.get_ios_window_size(false)
		get_window().move_to_center()
	elif should_emulate_android():
		var presets := _get_safe_area_presets()
		get_window().size = presets.get_android_window_size(false)
		get_window().move_to_center()
	else:
		get_window().size = Vector2i(1280, 720)
		get_window().move_to_center()
	orientation_changed.emit(false)


# The RN host owns the device orientation: only mirror the window shape into the layout state.
func _follow_host_orientation() -> void:
	var window := get_window()
	if not window.size_changed.is_connected(_follow_host_orientation):
		window.size_changed.connect(_follow_host_orientation)
	var portrait := window.size.y >= window.size.x
	if portrait != _is_portrait:
		_is_portrait = portrait
		orientation_changed.emit(portrait)


func is_orientation_portrait() -> bool:
	return _is_portrait


func set_orientation_portrait():
	if engine.is_host_driven():
		_follow_host_orientation()
		return
	# Set orientation BEFORE changing window size so listeners get correct value
	_is_portrait = true
	if Global.is_mobile() and !Global.is_virtual_mobile():
		DisplayServer.screen_set_orientation(DisplayServer.SCREEN_PORTRAIT)
	elif should_emulate_ios():
		var presets := _get_safe_area_presets()
		get_window().size = presets.get_ios_window_size(true)
		get_window().move_to_center()
	elif should_emulate_android():
		var presets := _get_safe_area_presets()
		get_window().size = presets.get_android_window_size(true)
		get_window().move_to_center()
	else:
		get_window().size = Vector2i(720, 1280)
		get_window().move_to_center()
	orientation_changed.emit(true)


## Entry points the emulator sink calls by name.
func async_teleport_to(parcel_position: Vector2i, new_realm: String) -> void:
	await WorldRequests.async_teleport_to(parcel_position, new_realm)


func async_join_world(world_realm: String) -> void:
	await WorldRequests.async_join_world(world_realm)


func _process(_delta: float) -> void:
	if Global.is_mobile() and !Global.is_virtual_mobile():
		var virtual_keyboard_height: int = DisplayServer.virtual_keyboard_get_height()

		# Shift the values
		previous_height_2 = previous_height
		previous_height = current_height
		current_height = virtual_keyboard_height

		# Check if stable (same for 3 frames) and different from last emitted
		if (
			current_height == previous_height
			and current_height == previous_height_2
			and current_height != last_emitted_height
		):
			last_emitted_height = current_height
			change_virtual_keyboard.emit(last_emitted_height)


## Check if the deep link contains a dclenv change. If it does, apply the new
## environment and restart back to the lobby (sign out). Returns true if a
## restart was triggered so the caller can skip further deep-link processing.
func _check_dclenv_change() -> bool:
	var new_env := deep_link_obj.dclenv as String
	if new_env.is_empty():
		return false

	var current_env := DclGlobal.get_dcl_environment() as String
	if new_env == current_env:
		return false

	# A sign-in callback (`decentraland://open?signin=...&dclenv=...`) echoes the env the
	# web flow authenticated against. That is a byproduct of the round trip, never a
	# request to switch. Acting on it restarts the session in the middle of the login:
	# sign_out() rebuilds the lobby, and the `true` returned below makes the router drop
	# the deeplink before it reaches _handle_signin_deep_link, so the auth goes with it.
	# The callback also carries the plain env (`zone`), which can never match a composed
	# one (`catalyst::org,...,profile::zone,zone`), so on those setups it fired every time.
	if deep_link_obj.is_signin_request():
		print("[DEEPLINK] dclenv=%s on a signin callback, keeping %s" % [new_env, current_env])
		return false

	# An env chosen explicitly (--dclenv, or an earlier deeplink) outranks what a later
	# deeplink carries. Switching from the default env still works; switching again
	# afterwards needs an app restart.
	if dcl_env_explicit:
		print("[DEEPLINK] dclenv=%s ignored, %s was set explicitly" % [new_env, current_env])
		return false

	print("[DEEPLINK] Environment changed: %s -> %s, restarting..." % [current_env, new_env])
	DclGlobal.set_dcl_environment(new_env)
	dcl_env_explicit = true
	sign_out()
	return true


func _notification(what: int) -> void:
	if what == NOTIFICATION_APPLICATION_FOCUS_IN:
		# Mobile OSes cut app networking shortly after backgrounding, killing every comms
		# session. Tell the comms manager we're back so it forgives transient reconnect
		# failures and treats Duplicate* evictions as our own stale session being reclaimed.
		# Real mobile only: on desktop every alt-tab fires FOCUS_IN, and a 30s grace window
		# armed that often would retry genuine evictions instead of surfacing the modal.
		if Global.is_mobile() and !Global.is_virtual_mobile():
			comms.notify_app_resumed()

	if what == NOTIFICATION_APPLICATION_FOCUS_IN or what == NOTIFICATION_READY:
		if Global.is_mobile() and !Global.is_virtual_mobile():
			var new_url: String = ""
			if DclAndroidPlugin.is_available():
				new_url = DclAndroidPlugin.get_deeplink_args().get("data", "")
			elif DclIosPlugin.is_available():
				new_url = DclIosPlugin.get_deeplink_args().get("data", "")

			# Only process a new URL: overwriting deep_link_url with empty would clobber
			# what the iOS signal path (process_deep_link) already set.
			if new_url.is_empty():
				return

			# On cold start, set the environment from the deeplink BEFORE processing it, or
			# _check_dclenv_change() sees default "org" vs deeplink env and calls sign_out()
			# prematurely, skipping the orientation/UI zoom setup in DclRoot.
			if what == NOTIFICATION_READY:
				var parsed = DclParseDeepLink.parse_decentraland_link(new_url)
				if not parsed.dclenv.is_empty():
					DclGlobal.set_dcl_environment(parsed.dclenv)
					dcl_env_explicit = true

			# READY = launch intent (the tap started it); FOCUS_IN = already running.
			deep_link_router.process_deep_link(
				new_url, "cold" if what == NOTIFICATION_READY else "warm"
			)


func _on_realm_access_denied(new_realm_string: String, _world_name: String) -> void:
	Navigator.recover_from_refusal(new_realm_string)


## Checks a realm's private-world access BEFORE any navigation UI is shown, so a world the
## user can't enter blocks with the modal instantly instead of first flashing a loading
## transition (#1725). The answer is cached, so the safety-net gate in Realm.async_set_realm
## reuses it without a second fetch. Non-worlds resolve synchronously with no network hit.
## Returns false when access was denied (the modal is already up); true otherwise.
func _async_precheck_realm_access(new_realm_string: String) -> bool:
	var world_name := WorldPermissionsHelper.world_name_from_realm(
		new_realm_string, Realm.resolve_realm_url(new_realm_string)
	)
	if world_name.is_empty():
		return true
	if await WorldPermissionsHelper.async_is_allowed(world_name):
		return true
	_on_realm_access_denied(new_realm_string, world_name)
	return false


## Fire-and-forget prefetch of a world's access, meant to run when its jump-in card opens.
## By the time the user taps JUMP IN the answer is cached, so the click-time precheck resolves
## synchronously: an allowed world goes straight to the loading screen with no visible gap, a
## private one shows the modal instantly. No-op (and no network) for non-worlds. Silent — it
## never shows a modal; the decision surfaces only on the actual navigation.
func warm_realm_access(realm_string: String) -> void:
	var world_name := WorldPermissionsHelper.world_name_from_realm(
		realm_string, Realm.resolve_realm_url(realm_string)
	)
	if world_name.is_empty():
		return
	WorldPermissionsHelper.async_is_allowed(world_name)


func set_camera_mode(camera_mode: Global.CameraMode) -> void:
	current_camera_mode = camera_mode
	camera_mode_set.emit(camera_mode)


func set_camera_mode_blocked(blocked: bool) -> void:
	if camera_mode_blocked == blocked:
		return
	camera_mode_blocked = blocked
	camera_mode_block_changed.emit(blocked)


# Anchor used to derive the thirdweb guest session/wallet. Shared by the lobby
# guest-login flow and the "Upgrade to OTP" modal so both derive the same wallet.
#
# Resolution order:
#   1. DEBUG_GUEST_ROTATE_ANCHOR_ID on AND non-production build → return "" so
#      Rust falls back to the resettable per-install UUID in
#      `user://device_anchor.txt` (delete user:// → fresh guest wallet) on every
#      platform. The production gate means this can never ship accidentally.
#   2. Otherwise (shipping): the device-bound native anchor (Android SSAID / iOS
#      Keychain UUID), which survives reinstall. Desktop has none → returns ""
#      and Rust uses the user:// UUID anyway.
#
# Android note: `has_method()` always returns false for JNISingleton methods
# (Object.has_method consults ClassDB, the Android plugin method_map is
# separate). Don't guard the Android call with has_method or it silently no-ops.
# See: https://github.com/godotengine/godot/issues/106436
func get_device_anchor_id() -> String:
	# 1. DEBUG rotate mode: resettable user:// anchor on every platform. Either the
	#    compile-time constant or the persisted `rotate-guest=true` deeplink override.
	#    Gated to non-production so neither can ever ship even if left on.
	if (
		(DEBUG_GUEST_ROTATE_ANCHOR_ID or get_config().debug_rotate_guest_anchor)
		and not is_production()
	):
		return ""
	# 2. Shipping: device-bound native anchor (persists across reinstall).
	var native_anchor := ""
	if self.is_android():
		var plugin = Engine.get_singleton("dcl-godot-android")
		if plugin != null:
			native_anchor = plugin.getDeviceAnchorId()
	elif self.is_ios():
		var plugin = Engine.get_singleton("DclGodotiOS")
		if plugin != null and plugin.has_method("get_device_anchor_id"):
			native_anchor = plugin.get_device_anchor_id()

	return native_anchor
