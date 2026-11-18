class_name TrackingAuthorization
extends RefCounted
## iOS App Tracking Transparency prompt (#3183). Asked once per install, right after the splash
## and before login or any other system prompt. iOS remembers the answer, so every later launch
## (and Android, desktop, or a plugin build without ATT) settles immediately.


static func async_request_if_needed() -> void:
	var plugin = _ios_plugin()
	if plugin == null or plugin.get_tracking_authorization_status() != "not_determined":
		_settle()
		return

	if Global.metrics != null:
		Global.metrics.track_att_prompt("shown")
	plugin.request_tracking_authorization()
	var status: String = await plugin.tracking_authorization_result
	if Global.metrics != null:
		Global.metrics.track_att_prompt("allowed" if status == "authorized" else "denied")
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
