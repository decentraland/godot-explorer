extends Node

const TIMEOUT_SECONDS := 3.0
const SNOOZE_SECONDS := 86400  # 24h
const OVERLAY_SCENE := preload("res://src/ui/components/update_available/update_available.tscn")
const OVERLAY_CANVAS_LAYER := 99  # modal_manager uses 100, so this sits just below

const RESULT_PROCEED := "proceed"
const RESULT_SOFT := "soft"
const RESULT_HARD := "hard"


# Convert semver "major.minor.patch" (ignoring any "-suffix") into a monotonic
# integer: major*100000 + minor*100 + patch. Minor/patch are clamped to 99.
# Returns -1 on parse failure.
# Examples: "0.64.3-abc1234-dev" -> 6403, "1.0.0" -> 100000
func _parse_version_number(full: String) -> int:
	var semver_parts := full.split("-", true, 1)
	if semver_parts.is_empty():
		return -1
	var parts := semver_parts[0].split(".")
	if parts.size() < 2:
		return -1
	var major := int(parts[0])
	var minor := clampi(int(parts[1]), 0, 99)
	var patch := clampi(int(parts[2]) if parts.size() >= 3 else 0, 0, 99)
	return major * 100000 + minor * 100 + patch


# Runs the check against the mobile-bff. Races the HTTP call against a 3s
# timeout; on timeout, error or malformed response returns RESULT_PROCEED
# silently (no retry — the check only runs once per boot).
# Server must encode minimalRequiredVersionNumber and recommendedVersionNumber
# using the same monotonic scheme (major*100000 + minor*100 + patch).
func async_check() -> String:
	var current := _parse_version_number(String(DclGlobal.get_version()))
	if current < 0:
		return RESULT_PROCEED

	var url := String(DclUrls.app_versions())
	var http_fn := func() -> Promise:
		return Global.http_requester.request_json(url, HTTPClient.METHOD_GET, "", {})
	var timeout_fn := func() -> Promise:
		var p := Promise.new()
		get_tree().create_timer(TIMEOUT_SECONDS).timeout.connect(
			func(): p.reject("version_gate: timeout")
		)
		return p

	var result = await PromiseUtils.async_race([http_fn, timeout_fn])
	if result is PromiseError:
		return RESULT_PROCEED

	var json = result.get_string_response_as_json()
	if typeof(json) != TYPE_DICTIONARY or not json.get("ok", false):
		return RESULT_PROCEED

	var data: Dictionary = json.get("data", {})
	var platform_key := "ios" if Global.is_ios() else "android"
	var p: Dictionary = data.get(platform_key, {})
	var minimal := int(p.get("minimalRequiredVersionNumber", 0))
	var recommended := int(p.get("recommendedVersionNumber", 0))

	if current < minimal:
		return RESULT_HARD
	if current < recommended:
		var now := int(Time.get_unix_time_from_system())
		if now < Global.get_config().version_gate_snooze_until:
			return RESULT_PROCEED
		return RESULT_SOFT
	return RESULT_PROCEED


func show_overlay(allow_later: bool) -> void:
	var layer := CanvasLayer.new()
	layer.layer = OVERLAY_CANVAS_LAYER
	var overlay := OVERLAY_SCENE.instantiate()
	overlay.setup(allow_later)
	layer.add_child(overlay)
	get_tree().root.add_child(layer)
