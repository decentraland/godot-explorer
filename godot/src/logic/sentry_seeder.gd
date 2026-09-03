class_name SentrySeeder
extends RefCounted

## Seeds Sentry user/context/tag state from Decentraland runtime signals.
## RefCounted, kept alive by Global's strong reference; signal connections
## do not own RefCounted targets on their own. No scene-tree presence except
## the memory-poll Timer it parents under Global.
##
## Sentry SDK init + static (process-lifetime) tags live in
## project_main_loop.gd — this controller only handles the dynamic state
## that depends on Global subsystems being ready: which scene the player is
## in, memory pressure, the device's graphics tier, and the structured
## events (scene crashes, previous-run Android exit reasons) that
## _before_send recognises by their `event_kind` tag.

# Sentry caps tag values at 200 chars and they must be single-line; scene
# titles are user content.
const TAG_VALUE_MAX := 200
const NO_SCENE := "none"

# Memory context refresh period. Writes are skipped when nothing moved (see
# _on_memory_poll), so this is only an upper bound on scope writes.
const MEMORY_POLL_SECONDS := 5.0
const MEMORY_RSS_DELTA_MB := 32
const PRESSURE_NAMES := ["ok", "warning", "critical"]

# Structured scene-crash events: one per scene per session, hard cap per
# session. They bypass the remote sample rate in _before_send, so the bound
# has to live here.
const MAX_SCENE_CRASH_REPORTS := 10
const SCENE_CRASH_MESSAGE_REASON_MAX := 160
const EXIT_DESCRIPTION_MESSAGE_MAX := 160

# Android exit reasons that are not a session death worth an event.
const BENIGN_EXIT_REASONS := [
	"exit_self",
	"user_requested",
	"user_stopped",
	"freezer",
	"permission_change",
	"package_state_change",
	"package_updated",
]
# ApplicationExitInfo.importance <= IMPORTANCE_FOREGROUND (100): the app was
# on screen when it died, as opposed to a cached process the OS reclaimed.
const IMPORTANCE_FOREGROUND := 100
# onTrimMemory levels >= TRIM_MEMORY_RUNNING_CRITICAL (15): the OS is about
# to start killing processes.
const TRIM_LEVEL_CRITICAL := 15

# scene_id -> title, filled on spawn: the scene is already gone from the
# runner when the kill/crash signals fire.
var _scene_titles: Dictionary = {}
var _scene_crash_reported: Dictionary = {}
var _memory_timer: Timer
var _last_rss_mb := -1
var _last_pressure := -1
var _total_ram_mb := -1


## Called by Global from _ready() after realm / scene_fetcher /
## player_identity / comms are constructed and `session_id` is assigned.
func setup() -> void:
	var sentry_user := SentryUser.new()
	sentry_user.id = Global.config.analytics_user_id
	SentrySDK.set_user(sentry_user)
	SentrySDK.set_tag("dcl_session_id", Global.session_id)
	# Refreshed by _on_wallet_connected once auth fires.
	SentrySDK.set_tag("is_guest", "true")

	Global.realm.realm_changed.connect(_on_realm_changed)
	Global.scene_fetcher.player_parcel_changed.connect(_on_parcel_changed)
	Global.comms.on_adapter_changed.connect(_on_adapter_changed)
	Global.player_identity.wallet_connected.connect(_on_wallet_connected)
	Global.player_identity.profile_changed.connect(_on_profile_changed)
	Global.player_identity.logout.connect(_on_logout)

	# Scene identity plus a spawn/kill/crash breadcrumb trail. scene_runner is
	# allocated in Rust before this runs, so connecting here cannot race.
	Global.scene_runner.on_change_scene_id.connect(_on_change_scene_id)
	Global.scene_runner.scene_spawned.connect(_on_scene_spawned)
	Global.scene_runner.scene_killed.connect(_on_scene_killed)
	Global.scene_runner.scene_crash_report.connect(_on_scene_crash_report)
	Global.scene_runner.low_memory_warning.connect(_on_low_memory_warning)

	# Config-derived tags. Global.config is the on-disk ConfigData by now,
	# which is not the case in ProjectMainLoop._initialize() (it runs before
	# any autoload _ready()).
	Global.config.param_changed.connect(_on_config_param_changed)
	_refresh_profile_tags()

	_setup_memory_polling()
	_setup_android_exit_diagnostics()


func _on_realm_changed() -> void:
	var realm_ctx := {
		"name": Global.realm.realm_name,
		"url": Global.realm.realm_url,
		"network_id": Global.realm.network_id,
		"content_base_url": Global.realm.content_base_url,
	}
	SentrySDK.set_context("realm", realm_ctx)
	SentrySDK.set_tag("realm", Global.realm.realm_name)


func _on_parcel_changed(new_position: Vector2i) -> void:
	var location_ctx := {
		"parcel": "%d,%d" % [new_position.x, new_position.y],
		"scene_entity_id": Global.scene_fetcher.current_scene_entity_id,
	}
	SentrySDK.set_context("location", location_ctx)


func _on_adapter_changed(_voice_chat_enabled: bool, new_adapter: String) -> void:
	SentrySDK.set_tag("comms_adapter", new_adapter)


# Keep user.id pinned to analytics_user_id across auth changes so
# "Users affected" stays attributed to a single install. Only username
# tracks the wallet/profile state.
func _on_wallet_connected(address: String, _chain_id: int, is_guest_value: bool) -> void:
	var sentry_user := SentryUser.new()
	sentry_user.id = Global.config.analytics_user_id
	if not is_guest_value:
		sentry_user.username = address
	SentrySDK.set_user(sentry_user)
	SentrySDK.set_tag("is_guest", "true" if is_guest_value else "false")


func _on_profile_changed(new_profile: DclUserProfile) -> void:
	var sentry_user := SentryUser.new()
	sentry_user.id = Global.config.analytics_user_id
	if new_profile != null:
		var display := new_profile.get_name()
		if not display.is_empty():
			sentry_user.username = display
	SentrySDK.set_user(sentry_user)


func _on_logout() -> void:
	var sentry_user := SentryUser.new()
	sentry_user.id = Global.config.analytics_user_id
	SentrySDK.set_user(sentry_user)
	SentrySDK.set_tag("is_guest", "true")


# --- Scene identity --------------------------------------------------------


func _tag_value(value: String) -> String:
	var single_line := value.replace("\n", " ").replace("\r", " ").strip_edges()
	if single_line.is_empty():
		return NO_SCENE
	return single_line.left(TAG_VALUE_MAX)


func _add_breadcrumb(message: String, category: String, level: int, data: Dictionary) -> void:
	var crumb := SentryBreadcrumb.create(message)
	crumb.category = category
	crumb.level = level
	crumb.set_data(data)
	SentrySDK.add_breadcrumb(crumb)


# The runner's current parcel scene: the one the player is standing in. The
# `location` context above is the player's parcel; this is the scene itself.
func _on_change_scene_id(scene_id: int) -> void:
	if scene_id < 0:
		SentrySDK.set_tag("scene_urn", NO_SCENE)
		SentrySDK.set_tag("scene_title", NO_SCENE)
		SentrySDK.set_tag("scene_parcel", NO_SCENE)
		SentrySDK.set_context("scene", {"id": -1, "urn": "", "title": "", "base_parcel": ""})
		return
	var urn: String = Global.scene_runner.get_scene_entity_id(scene_id)
	var title: String = Global.scene_runner.get_scene_title(scene_id)
	var base: Vector2i = Global.scene_runner.get_scene_base_parcel(scene_id)
	var parcel := "%d,%d" % [base.x, base.y]
	SentrySDK.set_tag("scene_urn", _tag_value(urn))
	SentrySDK.set_tag("scene_title", _tag_value(title))
	SentrySDK.set_tag("scene_parcel", parcel)
	SentrySDK.set_context(
		"scene", {"id": scene_id, "urn": urn, "title": title, "base_parcel": parcel}
	)


func _on_scene_spawned(scene_id: int, entity_id: String) -> void:
	var title: String = Global.scene_runner.get_scene_title(scene_id)
	_scene_titles[scene_id] = title
	_add_breadcrumb(
		"Scene spawned: %s" % title,
		"scene",
		SentrySDK.LEVEL_INFO,
		{"scene_id": scene_id, "urn": entity_id}
	)


func _on_scene_killed(scene_id: int, entity_id: String) -> void:
	var title: String = _scene_titles.get(scene_id, "")
	_scene_titles.erase(scene_id)
	_add_breadcrumb(
		"Scene killed: %s" % title,
		"scene",
		SentrySDK.LEVEL_INFO,
		{"scene_id": scene_id, "urn": entity_id}
	)


# Emitted by the Rust scene manager for every abnormal scene exit (JS thread
# panic, runtime error, exit without kill signal) with the identifiers still
# in hand. One Sentry issue per scene: the fingerprint is the entity id, so a
# redeploy of the same land is a new issue, and `scene_parcel` groups by land.
func _on_scene_crash_report(
	scene_id: int,
	entity_id: String,
	title: String,
	base_parcel: Vector2i,
	reason: String,
	uptime_s: int
) -> void:
	var parcel := "%d,%d" % [base_parcel.x, base_parcel.y]
	_add_breadcrumb(
		"Scene crashed: %s (%s)" % [title, parcel],
		"scene",
		SentrySDK.LEVEL_WARNING,
		{"scene_id": scene_id, "urn": entity_id, "reason": reason}
	)
	if _scene_crash_reported.has(entity_id):
		return
	if _scene_crash_reported.size() >= MAX_SCENE_CRASH_REPORTS:
		return
	_scene_crash_reported[entity_id] = true

	var event := SentrySDK.create_event()
	event.level = SentrySDK.LEVEL_ERROR
	event.message = (
		"Scene crashed: %s (%s): %s" % [title, parcel, reason.left(SCENE_CRASH_MESSAGE_REASON_MAX)]
	)
	event.set_fingerprint(PackedStringArray(["scene-crash", entity_id]))
	event.set_tag("event_kind", "scene_crash")
	event.set_tag("scene_urn", _tag_value(entity_id))
	event.set_tag("scene_title", _tag_value(title))
	event.set_tag("scene_parcel", parcel)
	(
		event
		. set_context(
			"scene_crash",
			{
				"scene_id": scene_id,
				"urn": entity_id,
				"title": title,
				"base_parcel": parcel,
				"reason": reason,
				"uptime_s": uptime_s,
			}
		)
	)
	SentrySDK.capture_event(event)


# --- Graphics profile / device tier ---------------------------------------


func _on_config_param_changed(param: ConfigData.ConfigParams) -> void:
	if param == ConfigData.ConfigParams.GRAPHIC_PROFILE:
		_refresh_profile_tags()


# The first-launch benchmark writes both scores and then applies a profile,
# which emits GRAPHIC_PROFILE, so one signal refreshes both tags. Runtime
# downgrades by the dynamic graphics manager arrive the same way.
func _refresh_profile_tags() -> void:
	var names := GraphicSettings.PROFILE_NAMES
	var index: int = clampi(Global.config.graphic_profile, 0, names.size() - 1)
	SentrySDK.set_tag("graphic_profile", names[index])

	var gpu_score: float = Global.config.benchmark_gpu_score
	if gpu_score < 0.0:
		SentrySDK.set_tag("benchmark_profile", "not_run")
		return
	var tier := HardwareBenchmark.determine_profile(gpu_score, Global.config.benchmark_ram_gb)
	SentrySDK.set_tag("benchmark_profile", HardwareBenchmark.PROFILE_THRESHOLDS[tier]["name"])


# --- Memory ----------------------------------------------------------------


func _setup_memory_polling() -> void:
	_total_ram_mb = _read_total_ram_mb()
	# OS.get_memory_info() reports -1 on Android, so the tag set at SDK init
	# only covers desktop; the native plugins know the real figure here.
	if _total_ram_mb > 0:
		SentrySDK.set_tag("total_ram_mb", str(_total_ram_mb))
	_memory_timer = Timer.new()
	_memory_timer.name = "SentryMemoryPoll"
	_memory_timer.wait_time = MEMORY_POLL_SECONDS
	_memory_timer.process_mode = Node.PROCESS_MODE_ALWAYS
	_memory_timer.timeout.connect(_on_memory_poll)
	Global.add_child(_memory_timer)
	_memory_timer.start()
	_on_memory_poll()


# Contexts are not synced to the native (NDK) scope, only tags are - so the
# pressure level is mirrored as a tag for the crashes that matter most.
func _on_memory_poll() -> void:
	var rss_mb: int = Global.scene_runner.get_process_memory_mb()
	var available_mb: int = Global.scene_runner.get_available_memory_mb()
	var level: int = clampi(Global.scene_runner.get_memory_pressure_level(), 0, 2)
	if level == _last_pressure and absi(rss_mb - _last_rss_mb) < MEMORY_RSS_DELTA_MB:
		return
	_last_rss_mb = rss_mb
	(
		SentrySDK
		. set_context(
			"memory",
			{
				"rss_mb": rss_mb,
				"available_mb": available_mb,
				"pressure_level": level,
				"total_ram_mb": _total_ram_mb,
			}
		)
	)
	if level != _last_pressure:
		_last_pressure = level
		SentrySDK.set_tag("memory_pressure", PRESSURE_NAMES[level])


func _on_low_memory_warning(
	scene_id: int, entity_id: String, footprint_mb: int, available_mb: int
) -> void:
	_add_breadcrumb(
		"Low memory warning: available=%dMB footprint=%dMB" % [available_mb, footprint_mb],
		"memory",
		SentrySDK.LEVEL_WARNING,
		{
			"scene_id": scene_id,
			"urn": entity_id,
			"footprint_mb": footprint_mb,
			"available_mb": available_mb,
		}
	)
	SentrySDK.set_tag("memory_pressure", "critical")
	_last_pressure = 2


# Same three-way lookup as HardwareBenchmark._get_system_ram_gb.
func _read_total_ram_mb() -> int:
	var ram_mb := -1
	if DclAndroidPlugin.is_available():
		ram_mb = DclAndroidPlugin.get_total_ram_mb()
	elif DclIosPlugin.is_available():
		ram_mb = DclIosPlugin.get_total_ram_mb()
	if ram_mb > 0:
		return ram_mb
	var physical_bytes: int = OS.get_memory_info().get("physical", -1)
	if physical_bytes > 0:
		return physical_bytes / 1048576
	return -1


# --- Android: previous-run exit reasons + memory trim -----------------------


# The Kotlin plugin reads ActivityManager's exit record for the previous run
# (API 30+) and hands it over; the SDK is only initialised on this side, so
# the plugin never captures itself. Guarded method by method: a stale AAR
# must not error. Every abnormal exit becomes one WARNING event fingerprinted
# by reason, so `event_kind:exit_reason` split by `exit_reason` is the
# session-death breakdown (LMK vs ANR vs native crash) with one denominator.
func _setup_android_exit_diagnostics() -> void:
	if not Global.is_android() or not Engine.has_singleton("dcl-godot-android"):
		return
	var plugin = Engine.get_singleton("dcl-godot-android")
	if plugin == null:
		return
	# JNISingleton dispatches plugin methods through callp() only: has_method()
	# and get_method_list() never see them, so probing getPreviousExitReasons
	# would disable this path on every device. The memory_trim signal ships in
	# the same plugin revision and IS introspectable, so it is the capability
	# marker for the exit-reason methods too.
	if not plugin.has_signal("memory_trim"):
		return
	plugin.connect("memory_trim", _on_memory_trim)
	var newest_timestamp := 0
	for exit_info in plugin.getPreviousExitReasons():
		newest_timestamp = maxi(newest_timestamp, int(exit_info.get("timestamp", 0)))
		_report_exit_reason(exit_info)
	# Acked only after capture, so an exit is never lost; if the process dies
	# before the envelope leaves the outbox it is reported again next launch.
	if newest_timestamp > 0:
		plugin.ackExitReasons(newest_timestamp)


func _report_exit_reason(exit_info: Dictionary) -> void:
	var reason: String = str(exit_info.get("reason", "other"))
	if reason in BENIGN_EXIT_REASONS:
		return
	var description: String = str(exit_info.get("description", ""))
	var importance: int = int(exit_info.get("importance", 1000))

	var event := SentrySDK.create_event()
	# A death of the previous run, never a crash of this session: WARNING keeps
	# it out of crash-free rates while the fingerprint keeps it countable.
	event.level = SentrySDK.LEVEL_WARNING
	# ANR descriptions carry the whole dispatcher reason; the title only needs
	# the head of it, the full text is in the exit_info context.
	event.message = (
		"Previous run ended: %s (%s)" % [reason, description.left(EXIT_DESCRIPTION_MESSAGE_MAX)]
	)
	event.set_fingerprint(PackedStringArray(["android-exit", reason]))
	event.set_tag("event_kind", "exit_reason")
	event.set_tag("exit_reason", reason)
	event.set_tag("exit_foreground", "true" if importance <= IMPORTANCE_FOREGROUND else "false")
	event.set_context("exit_info", exit_info)
	SentrySDK.capture_event(event)


func _on_memory_trim(level: int) -> void:
	var crumb_level := (
		SentrySDK.LEVEL_WARNING if level >= TRIM_LEVEL_CRITICAL else SentrySDK.LEVEL_INFO
	)
	_add_breadcrumb(
		"onTrimMemory level=%d" % level,
		"memory",
		crumb_level,
		{"level": level, "rss_mb": Global.scene_runner.get_process_memory_mb()}
	)
	SentrySDK.set_tag("last_trim_level", str(level))
