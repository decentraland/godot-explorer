class_name FeatureFlags
extends Node

## Remote feature flags fetched from the mobile-bff at app startup. The rules and the fetch live
## in the core (DclFeatureFlags); this node applies them to the rest of the app.
##
## The fetch is fire-and-forget: it starts when the node enters the tree
## (global.gd) and flags with runtime effects (comms) are
## applied as soon as the response arrives — normally long before the first
## comms connection. If a flag lands
## after comms already connected, the Rust-side runtime toggles reconcile
## gracefully (e.g. tearing archipelago down while keeping scene rooms alive).
##
## Fail-open: on timeout, network error or malformed payload every flag keeps
## its default and the feature stays enabled. Exception: `pulse` is fail-closed — it
## activates only when the payload explicitly enables it (see _apply_flags).

signal flags_loaded

# Flag names exactly as served by the mobile-bff payload.
const FLAG_ARCHIPELAGO := "archipielago"
const FLAG_PULSE := "pulse"
const FLAG_DUAL_CHANNEL := "dual-channel"
# Collect FCM push tokens and map them in Segment. Fail-open, and note what it does
# NOT do: turning it off stops new tokens from being registered, it does not stop
# delivery to tokens already collected — that is a server-side decision.
const FLAG_PUSH_ENABLED := "push-enabled"


func _ready() -> void:
	_async_load.call_deferred()


func is_loaded() -> bool:
	return DclFeatureFlags.is_loaded()


# `default_value` is returned while the flags aren't loaded yet or when the
# flag is absent from the payload.
func is_enabled(flag_name: String, default_value: bool = true) -> bool:
	return DclFeatureFlags.is_enabled(flag_name, default_value)


# Numeric flags (e.g. sample rates). `default_value` is returned while the
# flags aren't loaded yet, when the flag is absent, or when it isn't a number.
func get_number(flag_name: String, default_value: float) -> float:
	return DclFeatureFlags.get_number(flag_name, default_value)


# Extracts the flags dictionary from the mobile-bff response:
# `{"ok": true, "data": {"flags": {"archipielago": true, ...}}}`.
# Returns an empty dictionary on any shape mismatch (fail-open).
static func parse_response(json) -> Dictionary:
	return DclFeatureFlags.parse_response(json)


func _async_load() -> void:
	var result = await PromiseUtils.async_awaiter(DclFeatureFlags.async_load())
	if result is PromiseError:
		# Expected on offline/slow cold starts — fail-open is the design, so this must
		# not be error-level (Sentry quota).
		push_warning("[FeatureFlags] fetch failed (fail-open defaults): " + str(result.get_error()))
	else:
		print("[FeatureFlags] loaded: ", DclFeatureFlags.flags())

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

	# Push registration. This is the only place the flag is read, and Metrics holds the startup
	# identify until it arrives: the cached FCM token is already there in Metrics::ready(), so
	# the identify would otherwise always win the race and the switch could only ever suppress
	# token rotations. Reached on both paths above, so the hold ends even when the fetch fails.
	if Global.metrics != null:
		Global.metrics.set_push_enabled(is_enabled(FLAG_PUSH_ENABLED, true))
