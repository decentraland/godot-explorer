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
## its default and the feature stays enabled. Exceptions: `pulse` and
## `sentry-error-events` are fail-closed — they activate only when the payload
## explicitly enables them (see _apply_flags).

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

var _flags: Dictionary = {}
var _loaded := false


func _ready() -> void:
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
