extends PanelContainer

## On-screen QA harness for the in-app review prompt (issue #2739).
##
## Why this exists: Play silently suppresses its review card once a user is over the
## (undocumented, roughly monthly) quota — nothing renders, no error, the callback still fires.
## So "the card appeared" is not a signal a tester can rely on, and its absence proves nothing.
## This panel reports the coordinator's own decisions instead, and in dry-run mode the card is
## never requested at all, which also makes the whole state machine testable on desktop.
##
## It reads and writes the REAL `review_*` config keys rather than a parallel set, so what you
## exercise here is the shipping cadence, not a simulation of it.
##
## Dev builds only. Mounted by Global when `review_debug_enabled` is set (see the
## `review-debug=true` deeplink); every consumer is gated on `not Global.is_production()`.

const DAY_SECONDS := 86400

var collapsed := true

var _wired := false

@onready var status_label: RichTextLabel = %StatusLabel
@onready var log_label: RichTextLabel = %LogLabel
@onready var collapse_button: Button = %CollapseButton
@onready var controls: VBoxContainer = %Controls
@onready var timer: Timer = %Timer


func _ready() -> void:
	collapse_button.pressed.connect(_on_collapse_button_pressed)
	timer.timeout.connect(_refresh)

	%ButtonReset.pressed.connect(_on_reset_pressed)
	%ButtonBack7d.pressed.connect(_on_back_pressed.bind(7 * DAY_SECONDS, "7 days"))
	%ButtonBack14d.pressed.connect(_on_back_pressed.bind(14 * DAY_SECONDS, "14 days"))
	%ButtonBack1y.pressed.connect(_on_back_pressed.bind(365 * DAY_SECONDS, "1 year"))

	_ensure_wired()
	_apply_collapsed()
	_refresh()


# The panel can be mounted either at boot (deeplink already persisted, before the coordinator is
# in the tree) or live from a deeplink while running (coordinator already there). Rather than
# depend on that ordering, wire lazily and retry on every refresh tick until it takes.
func _ensure_wired() -> void:
	if _wired:
		return
	var coordinator := _coordinator()
	if coordinator == null:
		return
	_wired = true
	# Only the wearable claim gets a button: the other three triggers (friend, favorite, upvote)
	# are all reachable through normal UI in a few taps, and testing them for real is the point.
	# A claim notification is server-pushed and cannot be produced on demand.
	%ButtonWearable.pressed.connect(_fire.bind(coordinator.TRIGGER_WEARABLE_CLAIMED))
	coordinator.debug_event.connect(_on_debug_event)


func _coordinator() -> Node:
	return Global.review_prompt_coordinator


func _on_collapse_button_pressed() -> void:
	collapsed = not collapsed
	_apply_collapsed()


func _apply_collapsed() -> void:
	controls.visible = not collapsed
	log_label.visible = not collapsed
	collapse_button.text = "▸" if collapsed else "▾"
	if not collapsed:
		_refresh()
	# A Control never shrinks below its grown size on its own — snap the panel back to its
	# (now header-only) minimum, or the collapsed panel keeps the expanded height.
	_snap_height.call_deferred()


# Height only. reset_size() would also collapse the scene's authored width, because the
# autowrapping labels report a ~0 minimum width.
func _snap_height() -> void:
	size = Vector2(size.x, 0.0)


# --- status ---------------------------------------------------------------------------------


func _refresh() -> void:
	_ensure_wired()
	var coordinator := _coordinator()
	if coordinator == null:
		status_label.text = "[color=yellow]waiting for coordinator…[/color]"
		return

	var config: ConfigData = Global.get_config()
	var shots: int = config.review_shots_fired
	var max_shots: int = coordinator.MAX_SHOTS
	var lines := PackedStringArray()

	var capped := shots >= max_shots
	lines.append(
		(
			"[b]Review prompt[/b]   [color=%s]%d of %d asks used[/color]"
			% ["red" if capped else "white", shots, max_shots]
		)
	)

	if capped:
		lines.append("[color=red]Finished — it will never ask again[/color]")
	elif coordinator._is_due(config):
		lines.append("[color=green]Ready — the next action will ask[/color]")
	else:
		lines.append("[color=yellow]Waiting — %s[/color]" % _waiting_text(coordinator, config))

	var rail: String = coordinator._blocking_rail()
	if rail.is_empty():
		lines.append("[color=gray]Nothing blocking it[/color]")
	else:
		lines.append(
			"[color=orange]Held back: %s[/color]" % coordinator.RAIL_REASONS.get(rail, rail)
		)

	if coordinator.is_debug_enabled():
		lines.append("[color=green]Test mode — the real prompt is never shown[/color]")

	status_label.text = "\n".join(lines)
	_snap_height.call_deferred()


# What the next ask is still waiting on, as a phrase that completes "Waiting — ...".
func _waiting_text(coordinator: Node, config: ConfigData) -> String:
	if config.review_shots_fired == 0:
		return "needs %d sessions, has %d" % [coordinator.SESSION_GATE, config.review_session_count]
	var floor_seconds: int = (
		coordinator.SHOT_2_FLOOR_SECONDS
		if config.review_shots_fired == 1
		else coordinator.SHOT_3_FLOOR_SECONDS
	)
	var elapsed: int = int(Time.get_unix_time_from_system()) - config.review_last_shot_unix
	var remaining: int = floor_seconds - elapsed
	if remaining <= 0:
		return "ready on the next action"
	return "unlocks in %s" % _duration_text(remaining)


func _duration_text(seconds: int) -> String:
	if seconds >= DAY_SECONDS:
		var days: int = seconds / DAY_SECONDS
		var hours: int = (seconds % DAY_SECONDS) / 3600
		return "%dd %dh" % [days, hours]
	if seconds >= 3600:
		return "%dh %dm" % [seconds / 3600, (seconds % 3600) / 60]
	if seconds >= 60:
		return "%dm %ds" % [seconds / 60, seconds % 60]
	return "%ds" % seconds


# --- event log ------------------------------------------------------------------------------


# The panel shows the last event itself rather than relying only on the toast: show_system_toast
# queues behind any real notification, so a toast can arrive seconds late or out of order. This
# line is synchronous and authoritative.
func _on_debug_event(kind: String, text: String) -> void:
	var color := "green" if kind == "fired" else "yellow"
	var stamp := Time.get_time_string_from_system()
	log_label.text = "[color=%s]%s  %s[/color]" % [color, stamp, text]
	if kind == "fired":
		# Stand in for the Play card at exactly the moment it would have appeared. A toast would
		# under-sell it: the real thing is modal and takes over the screen.
		Global.modal_manager.async_show_review_prompt_debug_modal(
			"This is the moment Google Play's rating card would appear.\n\n" + text
		)
	else:
		NotificationsManager.show_system_toast("Review prompt", text, "system", "alert")
	_refresh()


# --- controls -------------------------------------------------------------------------------


func _on_reset_pressed() -> void:
	var coordinator := _coordinator()
	var config: ConfigData = Global.get_config()
	config.review_shots_fired = 0
	config.review_last_shot_unix = 0
	# Armed, not merely cleared: the session gate would otherwise need 5 real app launches.
	config.review_session_count = coordinator.SESSION_GATE
	config.save_to_settings_file()
	_note("Started over — ready to ask on the next action")


# Moves the last shot further into the past so the next floor opens. Wound back far enough
# (-1y) it also demonstrates that the lifetime cap is checked before any elapsed-time floor.
func _on_back_pressed(seconds: int, label: String) -> void:
	var config: ConfigData = Global.get_config()
	if config.review_last_shot_unix == 0:
		_note("%s does nothing yet — it has not asked once" % label)
		return
	config.review_last_shot_unix -= seconds
	config.save_to_settings_file()
	_note("Jumped %s ahead in time" % label)


func _fire(trigger_id: String) -> void:
	_coordinator().on_trigger(trigger_id)


func _note(text: String) -> void:
	print("[ReviewDebug] " + text)
	_on_debug_event("note", text)
