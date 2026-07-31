class_name FeatureFlags
extends Node

## Remote feature flags fetched from the mobile-bff at app startup.
##
## The fetch is fire-and-forget: it starts when the node enters the tree
## (global.gd) and comms-affecting flags are applied as soon as the response
## arrives — normally long before the first comms connection. If a flag lands
## after comms already connected, the Rust-side runtime toggles reconcile
## gracefully (e.g. tearing archipelago down while keeping scene rooms alive).
##
## Fail-open: on timeout, network error or malformed payload every flag keeps
## its default and the feature stays enabled.

signal flags_loaded

const TIMEOUT_SECONDS := 5.0

# Flag names exactly as served by the mobile-bff payload.
const FLAG_ARCHIPELAGO := "archipielago"
const FLAG_PULSE := "pulse"

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
# `pulse=false` is a fleet-wide kill switch for the Pulse transport: it tears the
# room down (or prevents its creation) and LiveKit avatar sync takes over. It is
# disable-only on purpose — a server flag must never force-enable Pulse over a
# local `--no-pulse` / `pulse=false` test run.
func _apply_flags() -> void:
	Global.comms.set_archipelago_enabled(is_enabled(FLAG_ARCHIPELAGO, true))
	if not is_enabled(FLAG_PULSE, true):
		Global.comms.set_pulse_enabled(false)
