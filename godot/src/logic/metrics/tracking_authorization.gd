class_name TrackingAuthorization
extends RefCounted
## iOS App Tracking Transparency prompt (#3183). Asked once per install, right after the splash
## and before login or any other system prompt. iOS remembers the answer, so every later launch
## (and Android, desktop, or a plugin build without ATT) settles immediately.

## Watchdog for an answer lost in the native glue, which would otherwise keep the player on the
## splash spinner on every launch. Generous, so a player reading the prompt is never cut short.
const ANSWER_TIMEOUT_MS := 60000


static func async_request_if_needed() -> void:
	var plugin = _ios_plugin()
	if plugin == null or plugin.get_tracking_authorization_status() != "not_determined":
		_settle()
		return

	if Global.metrics != null:
		Global.metrics.track_att_prompt("shown")
	# A one-shot connection instead of a bare await, so an answer that comes in after the
	# watchdog is still recorded. `answered` is mutated, never reassigned: lambdas capture by value.
	var answered: Array = []
	var on_answer := func(status) -> void:
		answered.append(status)
		if Global.metrics != null:
			Global.metrics.track_att_prompt("allowed" if status == "authorized" else "denied")
		_settle()
	plugin.tracking_authorization_result.connect(on_answer, CONNECT_ONE_SHOT)
	plugin.request_tracking_authorization()

	var tree := Engine.get_main_loop() as SceneTree
	var started_ms := Time.get_ticks_msec()
	while answered.is_empty() and Time.get_ticks_msec() - started_ms < ANSWER_TIMEOUT_MS:
		await tree.process_frame
	if answered.is_empty():
		push_warning("TrackingAuthorization: no answer within %ds" % (ANSWER_TIMEOUT_MS / 1000))
		_settle()


static func _ios_plugin():
	if not Global.is_ios() or not Engine.has_singleton("DclGodotiOS"):
		return null
	var plugin = Engine.get_singleton("DclGodotiOS")
	if plugin == null or not plugin.has_method("request_tracking_authorization"):
		return null
	return plugin


## Lets Metrics start Tenjin: its first session must wait for the answer, or it goes out
## without the IDFA the player may be about to allow.
static func _settle() -> void:
	if Global.metrics != null:
		Global.metrics.set_tracking_authorization_settled()
