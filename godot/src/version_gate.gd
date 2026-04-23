extends Node

const RETRY_SECONDS := 15.0
const OVERLAY_SCENE := preload("res://src/ui/components/update_available/update_available.tscn")
const OVERLAY_CANVAS_LAYER := 100

var _current_version_int: int = -1
var _retry_timer: Timer


func _ready() -> void:
	_current_version_int = _parse_current_minor()
	if _current_version_int < 0:
		push_warning("version_gate: could not parse current version; skipping check")
		queue_free()
		return

	_retry_timer = Timer.new()
	_retry_timer.one_shot = true
	_retry_timer.wait_time = RETRY_SECONDS
	_retry_timer.timeout.connect(_async_check)
	add_child(_retry_timer)
	_async_check.call_deferred()


func _parse_current_minor() -> int:
	# DclGlobal.get_version() -> "0.64.0-abc1234-dev" (or "0.64.0-t{ts}-dev" fallback)
	var full := String(DclGlobal.get_version())
	var semver_parts := full.split("-", true, 1)
	if semver_parts.is_empty():
		return -1
	var parts := semver_parts[0].split(".")
	if parts.size() < 2:
		return -1
	return int(parts[1])


func _async_check() -> void:
	var url := String(DclUrls.app_versions())
	var promise: Promise = Global.http_requester.request_json(url, HTTPClient.METHOD_GET, "", {})
	var result = await PromiseUtils.async_awaiter(promise)
	if result is PromiseError:
		push_warning(
			"version_gate: %s - retrying in %ds" % [result.get_error(), int(RETRY_SECONDS)]
		)
		_retry_timer.start()
		return

	var json = result.get_string_response_as_json()
	if typeof(json) != TYPE_DICTIONARY or not json.get("ok", false):
		push_warning("version_gate: malformed response - retrying")
		_retry_timer.start()
		return

	var data: Dictionary = json.get("data", {})
	var platform_key := "ios" if Global.is_ios() else "android"
	var p: Dictionary = data.get(platform_key, {})
	var minimal := int(p.get("minimalRequiredVersionNumber", 0))
	var recommended := int(p.get("recommendedVersionNumber", 0))

	if _current_version_int < minimal:
		_show_overlay(false)
	elif _current_version_int < recommended:
		_show_overlay(true)

	queue_free()


func _show_overlay(allow_later: bool) -> void:
	var layer := CanvasLayer.new()
	layer.layer = OVERLAY_CANVAS_LAYER
	var overlay := OVERLAY_SCENE.instantiate()
	overlay.setup(allow_later)
	layer.add_child(overlay)
	get_tree().root.add_child(layer)
