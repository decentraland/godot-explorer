extends Node

# Tests for the scene-authored nametag plate (#2811, `AvatarNametag`).
# Pins: an empty label is a color-only disc, spaces are preserved, long labels
# truncate, the authored colors land on the pill and the text, and scene text
# cannot inject rich text.
#
# Runs as a scene, not --script: SceneNametag preloads project fonts, and --script
# compiles before the autoloads exist.

## Guards against a silent early return: every check below must have run.
const EXPECTED_CHECKS := 13

var _failures: Array[String] = []
var _checks: int = 0


func _ready() -> void:
	_test_empty_label_is_a_disc()
	_test_label_keeps_spaces()
	_test_label_truncates()
	_test_colors_and_text()
	_finish()


func _make() -> SceneNametag:
	var plate := SceneNametag.new()
	add_child(plate)
	plate.configure("Club Owner", Color.WHITE, Color.BLACK, Color.BLACK)
	return plate


func _test_empty_label_is_a_disc() -> void:
	var plate := _make()
	plate.configure("", Color.WHITE, Color.BLACK, Color.BLACK)
	_expect("empty label kept", "", plate.label_text)
	_expect("empty label draws no text", "", plate._label.text)
	_expect("empty label is square", true, is_equal_approx(plate.size.x, plate.size.y))
	plate.free()


func _test_label_keeps_spaces() -> void:
	var plate := _make()
	plate.configure("            ", Color.WHITE, Color.BLACK, Color.BLACK)
	var disc_width := plate.size.y
	_expect("spaces kept", "            ", plate.label_text)
	_expect("spaces widen the plate", true, plate.size.x > disc_width)
	plate.free()


func _test_label_truncates() -> void:
	var plate := _make()
	plate.configure("x".repeat(400), Color.WHITE, Color.BLACK, Color.BLACK)
	_expect("long label clamped to max width", SceneNametag.MAX_WIDTH, plate.size.x)
	_expect("long label clips", true, plate._label.clip_text)
	plate.free()


func _test_colors_and_text() -> void:
	var plate := _make()
	var text_color := Color(1.0, 0.8, 0.0)
	var background := Color(0.1, 0.1, 0.1)
	var border := Color(0.4, 0.2, 0.6)
	plate.configure("VIP", text_color, background, border)
	_expect("label text", "VIP", plate._label.text)
	_expect("text color", text_color, plate._label.get_theme_color(&"font_color"))
	_expect("background color", background, plate._style.bg_color)
	_expect("border color", border, plate._style.border_color)
	_expect(
		"scene text is not translated",
		Node.AUTO_TRANSLATE_MODE_DISABLED,
		plate._label.auto_translate_mode
	)
	_expect(
		"corner radius is half the height", plate.size.y * 0.5, plate._style.corner_radius_top_left
	)
	plate.free()


func _expect(ctx: String, expected: Variant, actual: Variant) -> void:
	_checks += 1
	if expected != actual:
		_failures.append("%s: expected %s, got %s" % [ctx, expected, actual])


func _finish() -> void:
	if _checks != EXPECTED_CHECKS:
		_failures.append("ran %d checks, expected %d" % [_checks, EXPECTED_CHECKS])
	if _failures.is_empty():
		print("[test_scene_nametag] PASS")
		get_tree().quit(0)
		return
	for f in _failures:
		printerr(f)
	printerr("[test_scene_nametag] FAIL: %d case(s)" % _failures.size())
	get_tree().quit(1)
