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
const EXPECTED_CHECKS := 24

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
	_expect(chat_slot.get_child_count() == 2, "enter docks one chat under the tap shield")
	# The settle timer carries the layout epoch it was armed for: a stale one must not reveal.
	var chat: Control = chat_slot.get_child(0)
	var shield: Control = overlay.get_node("%Control_ChatShield")
	overlay._scroll_chat_to_bottom(overlay._chat_reveal_epoch - 1)
	_expect(
		chat.modulate == Color.TRANSPARENT and shield.visible,
		"a stale settle timer leaves the chat hidden, its taps shielded"
	)
	overlay._scroll_chat_to_bottom(overlay._chat_reveal_epoch)
	_expect(
		chat.modulate == Color.WHITE and not shield.visible,
		"the current settle timer reveals the chat and drops the shield"
	)

	overlay._process(0.0)
	var texture_rect: TextureRect = overlay.get_node("%TextureRect_Video")
	_expect(texture_rect.texture == player.texture, "the player's texture reaches the overlay")

	var layout: BoxContainer = overlay.get_node("%BoxContainer_Layout")
	var chat_column: Control = overlay.get_node("%PanelContainer_Chat")
	var collapse: Button = overlay.get_node("%Button_CollapseChat")
	var expand: Button = overlay.get_node("%Button_ExpandChat")
	var button_exit: Button = overlay.get_node("%Button_Exit")
	Global.orientation_changed.emit(true)
	_expect(layout.vertical, "portrait stacks the chat under the video")
	var exit_box := button_exit.get_theme_stylebox("normal") as StyleBoxFlat
	_expect(
		(
			button_exit.custom_minimum_size == Vector2(96, 96)
			and exit_box.corner_radius_top_left == 48
			and texture_rect.offset_top > 0.0
		),
		"portrait shows a 96 px exit disc and the video under the exit/rotate row"
	)
	collapse.pressed.emit()
	_expect(
		chat_column.visible and not chat_slot.visible,
		"portrait collapse keeps only the chat header"
	)
	Global.orientation_changed.emit(false)
	_expect(not layout.vertical, "landscape puts the chat beside the video")
	_expect(
		(
			button_exit.custom_minimum_size == Vector2(66, 66)
			and exit_box.corner_radius_top_left == 33
			and texture_rect.offset_top == 0.0
		),
		"landscape shows a 66 px exit disc and the row floating over the video"
	)
	_expect(
		not chat_column.visible and expand.visible,
		"landscape keeps the collapse: no chat column, edge tab shown"
	)
	expand.pressed.emit()
	_expect(chat_column.visible and not expand.visible, "the edge tab brings the chat back")

	# Typing in landscape: the input stretches over the video and the header steps aside.
	var header: PanelContainer = overlay.get_node("%PanelContainer_Header")
	var line_edit_area: MarginContainer = chat._line_edit_safe_area
	chat.on_enter_write_mode.emit()
	var column_left := chat.global_position.x - overlay.global_position.x
	_expect(
		(
			line_edit_area.get_theme_constant("margin_left")
			== -roundi(column_left - overlay._inset_left)
		),
		"landscape write mode moves the input left to the screen edge"
	)
	Global.change_virtual_keyboard.emit(400)
	_expect(not header.visible, "landscape hides the NEARBY header while the keyboard is up")
	Global.change_virtual_keyboard.emit(0)
	_expect(header.visible, "the header is back once the keyboard goes")
	chat.on_exit_write_mode.emit()
	_expect(
		line_edit_area.get_theme_constant("margin_left") == 0,
		"leaving write mode returns the input to its column"
	)

	# Side insets are arithmetic over the device insets, so they are driven here without one.
	var controls: MarginContainer = overlay.get_node("%MarginContainer_Controls")
	var header_box := header.get_theme_stylebox("panel") as StyleBoxFlat
	overlay._layout_side_insets(44, 30)
	_expect(
		(
			controls.get_theme_constant("margin_left") == 48 + 44
			and controls.get_theme_constant("margin_right") == 48
			and header_box.content_margin_left == 16
			and header_box.content_margin_right == 16 + 30
			and chat.offset_left == 0
			and chat.offset_right == -30
			and expand.offset_right == -(24 + 30)
		),
		"landscape: the left inset on the controls, the right one on the chat side only"
	)
	collapse.pressed.emit()
	overlay._layout_side_insets(44, 30)
	_expect(
		controls.get_theme_constant("margin_right") == 48 + 30,
		"landscape collapsed: the video reaches the right edge, so the controls take that inset"
	)
	expand.pressed.emit()
	Global.orientation_changed.emit(true)
	overlay._layout_side_insets(44, 30)
	_expect(
		(
			controls.get_theme_constant("margin_left") == 40 + 44
			and controls.get_theme_constant("margin_right") == 40 + 30
			and header_box.content_margin_left == 32 + 44
			and header_box.content_margin_right == 32 + 30
			and chat.offset_left == 44
			and chat.offset_right == -30
		),
		"portrait: both insets on the controls, the header and the chat"
	)
	Global.change_virtual_keyboard.emit(400)
	_expect(header.visible, "portrait keeps the NEARBY header while typing")
	Global.change_virtual_keyboard.emit(0)

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
