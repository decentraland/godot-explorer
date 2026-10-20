extends Node

# Unit tests for DeepLinkRouter.consume_navigation_target() (#2569, #3088): once read, a
# deeplink's realm/location must not survive to a later explorer boot, while preview must.
# Runs as a scene so the Global autoload exists.
#
# Run headless:
#   .bin/godot/godot4_bin --headless --path godot \
#     res://src/test/deep_link/test_deep_link_consume.tscn --quit

const PREVIEW := "http://127.0.0.1:8000"

var failures := 0


func _ready() -> void:
	var saved_obj: DclParseDeepLink = Global.deep_link_obj
	var saved_url: String = Global.deep_link_url

	_test_location_target()
	_test_realm_target()

	Global.deep_link_obj = saved_obj
	Global.deep_link_url = saved_url

	if failures == 0:
		print("[test_deep_link_consume] PASS")
	else:
		printerr("[test_deep_link_consume] %d FAILURE(S)" % failures)
	get_tree().quit(1 if failures > 0 else 0)


func _test_location_target() -> void:
	_arm("decentraland://open?position=-31,19", "", Vector2i(-31, 19))
	DeepLinkRouter.new().consume_navigation_target()
	_assert_consumed("location target")


func _test_realm_target() -> void:
	_arm("decentraland://open?realm=pumpkinball.dcl.eth", "pumpkinball.dcl.eth", Vector2i.MAX)
	DeepLinkRouter.new().consume_navigation_target()
	_assert_consumed("realm target")


func _arm(url: String, realm: String, location: Vector2i) -> void:
	var obj := DclParseDeepLink.new()
	obj.realm = realm
	obj.location = location
	obj.preview = PREVIEW
	Global.deep_link_obj = obj
	Global.deep_link_url = url


func _assert_consumed(label: String) -> void:
	_expect(label + ": realm cleared", Global.deep_link_obj.realm.is_empty())
	_expect(label + ": location cleared", not Global.deep_link_obj.is_location_defined())
	_expect(label + ": url cleared", Global.deep_link_url.is_empty())
	_expect(label + ": preview kept", Global.deep_link_obj.preview == PREVIEW)


func _expect(label: String, ok: bool) -> void:
	if not ok:
		failures += 1
		printerr("FAIL: ", label)
