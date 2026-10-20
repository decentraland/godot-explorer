extends Node

# Smoke test for the video focus mode (issue #1381): it opens on a player, docks a chat, shows
# the player's texture, follows orientation and tears down on exit or when the player goes
# away. Runs as a scene, not --script: the overlay reaches Global (scene_runner, orientation),
# and a --script run compiles before the autoloads exist, turning every call into a no-op.
# The runner passes --quit, so everything happens inside _ready: the overlay's per-frame step
# is driven by hand instead of awaiting frames.

const OVERLAY_SCENE := preload(
	"res://src/ui/components/organisms/video_focus_mode/video_focus_mode.tscn"
)

## Assertions that actually evaluated; guards against a suite that never ran.
const EXPECTED_CHECKS := 9

var _failures: Array[String] = []
var _checks: int = 0


## Stands in for DclVideoPlayer: the overlay only needs get_backend_texture().
class FakeVideoPlayer:
	extends Node
	var texture: ImageTexture

	func _init() -> void:
		var image := Image.create(16, 9, false, Image.FORMAT_RGBA8)
		texture = ImageTexture.create_from_image(image)

	func get_backend_texture() -> Texture2D:
		return texture


func _ready() -> void:
	_test_lifecycle()
	_finish()


func _test_lifecycle() -> void:
	var overlay: Control = OVERLAY_SCENE.instantiate()
	add_child(overlay)
	_expect(not overlay.visible, "hidden until a video screen is tapped")

	var player := FakeVideoPlayer.new()
	add_child(player)
	overlay.enter(player)
	_expect(overlay.visible, "enter shows the overlay")
	var chat_slot: Control = overlay.get_node("%Control_ChatSlot")
	_expect(chat_slot.get_child_count() == 1, "enter docks one chat")

	overlay._process(0.0)
	var texture_rect: TextureRect = overlay.get_node("%TextureRect_Video")
	_expect(texture_rect.texture == player.texture, "the player's texture reaches the overlay")

	var layout: BoxContainer = overlay.get_node("%BoxContainer_Layout")
	Global.orientation_changed.emit(true)
	_expect(layout.vertical, "portrait stacks the chat under the video")
	Global.orientation_changed.emit(false)
	_expect(not layout.vertical, "landscape puts the chat beside the video")

	overlay.get_node("%Button_Exit").pressed.emit()
	_expect(not overlay.visible, "the exit button hides the overlay")
	_expect(chat_slot.get_child(0).is_queued_for_deletion(), "exit frees the docked chat")

	overlay.enter(player)
	player.free()
	overlay._process(0.0)
	_expect(not overlay.visible, "a freed player (scene unloaded) closes the overlay")
	overlay.queue_free()


func _expect(condition: bool, ctx: String) -> void:
	_checks += 1
	if not condition:
		_failures.append(ctx)


func _finish() -> void:
	if _checks != EXPECTED_CHECKS:
		_failures.append(
			"ran %d checks, expected %d -- did the scripts compile?" % [_checks, EXPECTED_CHECKS]
		)
	if _failures.is_empty():
		print("[test_video_focus_mode] PASS")
		get_tree().quit(0)
		return
	for failure in _failures:
		printerr(failure)
	printerr("[test_video_focus_mode] FAIL: %d case(s)" % _failures.size())
	get_tree().quit(1)
