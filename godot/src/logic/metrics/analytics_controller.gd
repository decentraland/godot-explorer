class_name AnalyticsController
extends RefCounted

## RefCounted controller for the GDScript side of the analytics pipeline. Held alive by Global's
## strong reference; signal connections do not own RefCounted targets on their own.
##
## Owns a transient Timer node only while polling for the player's first horizontal movement;
## the timer is freed as soon as `first_move_in_world` fires (or `loading_started` preempts it).
## No persistent scene-tree presence outside that one-shot polling window.
##
## Responsibilities:
##   • EULA consent gate (returning users in setup(), fresh accept via on_eula_accepted_locally).
##   • Firebase `login_success` on wallet_connected, suppressed for session recoveries.
##   • Firebase one-shot `first_move_in_world` after the player's first real movement.

# Horizontal velocity (m/s) above which we consider the player has "really moved" — filters
# physics drift and small camera corrections from triggering first_move_in_world.
const _MOVE_IN_WORLD_VELOCITY_THRESHOLD: float = 0.5

# 10 Hz polling is plenty for "first move" detection; cheaper than _process at 60 fps.
const _FIRST_MOVE_POLL_INTERVAL: float = 0.1

# Set to true while `try_recover_account` is restoring a previous session so the synthetic
# `wallet_connected` it emits doesn't get logged as a fresh `login_success` Firebase event.
var _wallet_connected_is_session_recovery: bool = false

# Lazily created when armed by loading_finished; queue_free'd when the event fires.
var _first_move_poll_timer: Timer = null


## Called by Global right after `AnalyticsController.new()`. RefCounted has no _ready, so this
## stand-in performs the one-time setup (EULA gate for returning users + signal wiring).
func setup() -> void:
	if Services.metrics == null:
		return

	# Returning user: EULA already accepted on a prior run. Open the consent gate at startup so
	# queued events ship. Segment ↔ Firebase link is handled automatically by Metrics (user_id /
	# session_id seeded as Firebase user properties; `Firebase Init` Segment event queued via the
	# plugin's app-instance-id signal).
	if Services.config.terms_and_conditions_version == Global.TERMS_AND_CONDITIONS_VERSION:
		Services.metrics.set_eula_accepted.call_deferred(true)

	# Connection order is load-bearing: this handler must run BEFORE lobby.gd's
	# `_on_wallet_connected` because `track_login` reads the wallet address from the signal arg
	# (lobby's handler is what calls `metrics.update_identity`, so `common.dcl_eth_address` is
	# stale at this point). We connect here in Global._ready, before the lobby scene loads.
	Services.player_identity.wallet_connected.connect(_on_wallet_connected_track_login)
	Global.loading_started.connect(_on_loading_started)
	Global.loading_finished.connect(_on_loading_finished)


## Called by lobby.gd when the user clicks "Accept" on the EULA. Opens the consent gate (which
## auto-flushes queued pre-consent events) and fires the Firebase `eula_accepted` Key Event.
func on_eula_accepted_locally() -> void:
	if Services.metrics == null:
		return
	Services.metrics.set_eula_accepted(true)
	Services.metrics.track_eula_accepted()


## Called by lobby.gd right before `try_recover_account`. Suppresses the next `wallet_connected`
## emission from being counted as a fresh `login_success`. The flag clear is scheduled for the
## start of the NEXT frame (via `process_frame` one-shot) — NOT via call_deferred, because that
## queue is FIFO and would clear before Rust's deferred emit_signal runs at end of this frame.
func mark_wallet_connected_as_recovery() -> void:
	_wallet_connected_is_session_recovery = true
	var tree := Engine.get_main_loop() as SceneTree
	tree.process_frame.connect(_clear_wallet_connected_session_recovery_flag, CONNECT_ONE_SHOT)


func _clear_wallet_connected_session_recovery_flag() -> void:
	_wallet_connected_is_session_recovery = false


func _on_wallet_connected_track_login(
	address: String, _chain_id: int, is_guest_value: bool
) -> void:
	if _wallet_connected_is_session_recovery:
		return
	if Services.metrics == null:
		return
	Services.metrics.track_login(address, is_guest_value)


func _on_loading_started() -> void:
	_stop_first_move_poller()


func _on_loading_finished() -> void:
	if Services.config.first_move_in_world_sent:
		return
	_start_first_move_poller()


## Creates the polling Timer under Global (so it shares Global's lifetime) and starts it.
## No-op if already running.
func _start_first_move_poller() -> void:
	if _first_move_poll_timer != null:
		return
	_first_move_poll_timer = Timer.new()
	_first_move_poll_timer.name = "FirstMovePoller"
	_first_move_poll_timer.wait_time = _FIRST_MOVE_POLL_INTERVAL
	_first_move_poll_timer.one_shot = false
	_first_move_poll_timer.autostart = true
	_first_move_poll_timer.timeout.connect(_on_first_move_poll_tick)
	Global.add_child.call_deferred(_first_move_poll_timer)


func _stop_first_move_poller() -> void:
	if _first_move_poll_timer == null:
		return
	_first_move_poll_timer.queue_free()
	_first_move_poll_timer = null


func _on_first_move_poll_tick() -> void:
	if Services.scene_runner == null:
		return
	var player_body = Services.scene_runner.player_body_node
	if player_body == null:
		return
	if not "actual_velocity_xz" in player_body:
		return
	if player_body.actual_velocity_xz < _MOVE_IN_WORLD_VELOCITY_THRESHOLD:
		return
	# Persist the one-shot flag ONLY when the Firebase event was actually logged (Android + EULA
	# accepted). On other platforms or pre-EULA we still stop polling for this session, but the
	# flag stays false so a future session retries.
	var fired := false
	if Services.metrics != null:
		fired = Services.metrics.track_first_move_in_world()
	if fired:
		var config := Services.config
		config.first_move_in_world_sent = true
		config.save_to_settings_file()
	_stop_first_move_poller()
