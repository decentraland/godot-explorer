class_name FeatureFlags
extends Node

## Remote feature flags fetched from the mobile-bff at app startup.
##
## The fetch is fire-and-forget: it starts when the node enters the tree
## (global.gd) and flags with runtime effects (comms, Sentry sampling) are
## applied as soon as the response arrives — normally long before the first
## comms connection. If a flag lands
## after comms already connected, the Rust-side runtime toggles reconcile
## gracefully (e.g. tearing archipelago down while keeping scene rooms alive).
##
## Fail-open: on timeout, network error or malformed payload every flag keeps
## its default and the feature stays enabled. Exceptions: `pulse`,
## `sentry-error-events` and `async-shader-compilation` are fail-closed — they
## activate only when the payload explicitly enables them (see _apply_flags).

signal flags_loaded

const TIMEOUT_SECONDS := 5.0

# Flag names exactly as served by the mobile-bff payload.
const FLAG_ARCHIPELAGO := "archipielago"
const FLAG_PULSE := "pulse"
const FLAG_DUAL_CHANNEL := "dual-channel"
# Sentry error-event sampling, served as a number in [0, 1].
const FLAG_SENTRY_SAMPLE_RATE := "sentry-sample-rate"
# Report ERROR-level Sentry events (the engine/Rust error firehose). Fail-closed:
# absent flag = crash/fatal-only reporting — see ProjectMainLoop._before_send.
const FLAG_SENTRY_ERROR_EVENTS := "sentry-error-events"
# The bff also serves `sentry-traces-sample-rate`, but sentry-godot exposes no
# performance-tracing API yet — there is nothing to apply it to until the SDK
# grows one.
# Collect FCM push tokens and map them in Segment. Fail-open, and note what it does
# NOT do: turning it off stops new tokens from being registered, it does not stop
# delivery to tokens already collected — that is a server-side decision.
const FLAG_PUSH_ENABLED := "push-enabled"
# Non-blocking shader/pipeline compilation (engine opt-in, Mobile renderer, Android/iOS only).
# Fail-closed, and it cannot be turned off once on. CLI `--async-shaders` / `--no-async-shaders`
# and the deeplink `async-shaders=true/false` win over the flag.
const FLAG_ASYNC_SHADER_COMPILATION := "async-shader-compilation"
const ASYNC_SHADERS_METHOD := &"enable_async_shader_compilation"
# The Day 1 welcome local notification. Fail-open kill switch: off stops scheduling it and
# disarms it if already armed. Event reminders and the OS permission prompt are not gated.
const FLAG_DAY1_NOTIFICATION := "day1-notification"
# Foundation parity, the same flag name Unity reads: scene glTF materials render with
# backfaces culled, which is what an inverted-hull outline (a `doubleSided` shell with
# flipped normals) expects. Read at material-fix time — see GltfContainer.fix_material.
# CLI `--force-backface-culling` / `--no-force-backface-culling` and the deeplink
# `force-backface-culling=true/false` win over the flag, so QA can see it without the
# bff serving it.
const FLAG_FORCE_BACKFACE_CULLING := "alfa-force-backface-culling"

var _flags: Dictionary = {}
var _loaded := false
var _async_shaders_enabled := false


func _ready() -> void:
	# A local opt-in applies right away, without waiting for the fetch.
	_update_async_shader_compilation()
	Global.deep_link_router.deep_link_received.connect(_update_async_shader_compilation)
	_async_load.call_deferred()


func is_loaded() -> bool:
	return _loaded


# `default_value` is returned while the flags aren't loaded yet or when the
# flag is absent from the payload.
func is_enabled(flag_name: String, default_value: bool = true) -> bool:
	return bool(_flags.get(flag_name, default_value))


# Numeric flags (e.g. sample rates). `default_value` is returned while the
# flags aren't loaded yet, when the flag is absent, or when it isn't a number.
func get_number(flag_name: String, default_value: float) -> float:
	var value = _flags.get(flag_name)
	if value is float or value is int:
		return float(value)
	return default_value


# Text flags. `default_value` is returned while the flags aren't loaded yet, when the flag is
# absent, or when it isn't a string.
func get_text(flag_name: String, default_value: String) -> String:
	var value = _flags.get(flag_name)
	if value is String:
		return value
	return default_value


# "true" / "false" when the run sets it locally, "" otherwise. The CLI wins over the
# deeplink, and both win over the remote flag they override.
func _local_override(arg_on: String, arg_off: String, param: String) -> String:
	if Global.cli.has_arg(arg_off):
		return "false"
	if Global.cli.has_arg(arg_on):
		return "true"
	if Global.deep_link_obj != null:
		var value := str(Global.deep_link_obj.params.get(param, "")).to_lower()
		if not value.is_empty():
			return "true" if value in ["true", "1", "yes"] else "false"
	return ""


# Foundation-parity cull mode for scene glTF materials: the remote flag, with the local
# opt-in on top (see FLAG_FORCE_BACKFACE_CULLING).
func is_force_backface_culling_enabled() -> bool:
	var local_override := _local_override(
		"--force-backface-culling", "--no-force-backface-culling", "force-backface-culling"
	)
	if not local_override.is_empty():
		return local_override == "true"
	return is_enabled(FLAG_FORCE_BACKFACE_CULLING, false)


# Extracts the flags dictionary from the mobile-bff response:
# `{"ok": true, "data": {"flags": {"archipielago": true, ...}}}`.
# Returns an empty dictionary on any shape mismatch (fail-open).
static func parse_response(json) -> Dictionary:
	if typeof(json) != TYPE_DICTIONARY or not json.get("ok", false):
		return {}
	var data = json.get("data", {})
	if typeof(data) != TYPE_DICTIONARY:
		return {}
	var flags = data.get("flags", {})
	if typeof(flags) != TYPE_DICTIONARY:
		return {}
	return flags


func _async_load() -> void:
	var url := String(DclUrls.feature_flags())
	var http_fn := func() -> Promise:
		return Global.http_requester.request_json(url, HTTPClient.METHOD_GET, "", {})
	var timeout_fn := func() -> Promise:
		var p := Promise.new()
		get_tree().create_timer(TIMEOUT_SECONDS).timeout.connect(
			func(): p.reject("feature_flags: timeout")
		)
		return p

	var result = await PromiseUtils.async_race([http_fn, timeout_fn])
	if result is PromiseError:
		# Expected on offline/slow cold starts — fail-open is the design, so this must
		# not be error-level (Sentry quota).
		push_warning("[FeatureFlags] fetch failed (fail-open defaults): " + str(result.get_error()))
	else:
		_flags = FeatureFlags.parse_response(result.get_string_response_as_json())
		print("[FeatureFlags] loaded: ", _flags)

	_loaded = true
	_apply_flags()
	flags_loaded.emit()


# Applies the comms-affecting flags. `archipielago=false` skips the archipelago
# connection while scene rooms (and Pulse) keep working — see
# CommunicationManager::set_archipelago_enabled on the Rust side.
# `pulse` is the fail-closed exception: the transport activates only when the
# payload explicitly enables it — a fetch failure or an absent flag reports
# `false` and Pulse stays off (LiveKit avatar sync covers everything). The flag
# only decides the default: explicit local opt-ins (deeplink `pulse=true` /
# `pulse-server=`, CLI `--pulse`) and opt-outs (`--no-pulse`, `pulse=false`)
# always win — see CommunicationManager::pulse_enabled on the Rust side.
# `dual-channel` is the rollout control for avatar sync: while it is true (the
# default, and today's behaviour) movement and emotes keep going over LiveKit
# even when Pulse is established. Flipping it false hands them to Pulse alone —
# only safe once the deployment's authoritative servers ingest Pulse as scene
# listeners, which is a property of the deployment, not of this client. Local
# `--livekit-movement` / `--no-livekit-movement` / `dual-channel=` still win.
func _apply_flags() -> void:
	Global.comms.set_archipelago_enabled(is_enabled(FLAG_ARCHIPELAGO, true))
	Global.comms.set_pulse_flag_enabled(is_enabled(FLAG_PULSE, false))
	Global.comms.set_dual_channel_flag_enabled(is_enabled(FLAG_DUAL_CHANNEL, true))

	# SentrySDK.init runs at process start (before this fetch resolves), so the
	# remote rate is enforced through the _before_send gate, not the init option.
	# The cast fails only outside a normal game run (editor tools/tests), where
	# there is no Sentry to throttle.
	var main_loop := get_tree() as ProjectMainLoop
	if main_loop != null:
		main_loop.set_sentry_sample_rate(
			get_number(FLAG_SENTRY_SAMPLE_RATE, ProjectMainLoop.DEFAULT_SENTRY_SAMPLE_RATE)
		)
		main_loop.set_sentry_error_events_enabled(is_enabled(FLAG_SENTRY_ERROR_EVENTS, false))

	# Push registration. This is the only place the flag is read, and Metrics holds the startup
	# identify until it arrives: the cached FCM token is already there in Metrics::ready(), so
	# the identify would otherwise always win the race and the switch could only ever suppress
	# token rotations. Reached on both paths above, so the hold ends even when the fetch fails.
	if Global.metrics != null:
		Global.metrics.set_push_enabled(is_enabled(FLAG_PUSH_ENABLED, true))

	NotificationsManager.set_day1_notification_enabled(is_enabled(FLAG_DAY1_NOTIFICATION, true))
	_update_async_shader_compilation()


func _update_async_shader_compilation() -> void:
	if _async_shaders_enabled:
		return
	var local_override := _local_override("--async-shaders", "--no-async-shaders", "async-shaders")
	var enabled: bool
	if not local_override.is_empty():
		enabled = local_override == "true"
	elif _loaded:
		enabled = is_enabled(FLAG_ASYNC_SHADER_COMPILATION, false)
	else:
		return
	if enabled:
		enabled = _enable_async_shader_compilation()
	if not enabled:
		print("[FeatureFlags] async shader compilation: disabled")
	if SentrySDK.is_enabled():
		SentrySDK.set_tag("async_shader_compilation", "on" if enabled else "off")


func _enable_async_shader_compilation() -> bool:
	var threads := 0
	if OS.has_feature("android"):
		threads = 3
	elif OS.has_feature("ios"):
		threads = 2
	if threads == 0 or not RenderingServer.has_method(ASYNC_SHADERS_METHOD):
		return false
	RenderingServer.call(ASYNC_SHADERS_METHOD, threads)
	_async_shaders_enabled = true
	print("[FeatureFlags] async shader compilation: enabled (%d threads)" % threads)
	return true
