extends Node

# Tests for the scene-authored nametag plate (#2811, `AvatarNametag`).
# Pins: an empty label is a color-only disc, spaces are preserved, long labels
# truncate, the authored colors land on the pill and the text, scene text cannot
# inject rich text, and where NameplateLayer draws the plate — one gap above the
# nametag, or in the nametag's slot when there is none.
#
# Runs as a scene, not --script: SceneNametag preloads project fonts, and --script
# compiles before the autoloads exist.

## Guards against a silent early return: every check below must have run.
const EXPECTED_CHECKS := 25

var _failures: Array[String] = []
var _checks: int = 0


func _ready() -> void:
	_test_empty_label_is_a_disc()
	_test_label_keeps_spaces()
	_test_label_truncates()
	_test_colors_and_text()
	_test_plate_gap_is_drawn_px()
	_test_tag_floats_above_the_nametag()
	_test_tag_takes_the_name_slot_without_a_nametag()
	# Layout needs frames, and gdlint forbids an `await` in `_ready`.
	_async_test_real_plate_slot.call_deferred()


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
	_expect("scene text is drawn in a plain Label", true, plate._label is Label)
	_expect(
		"corner radius is half the height", plate.size.y * 0.5, plate._style.corner_radius_top_left
	)
	plate.free()


## The design's 6 px is what the player sees on screen, and the layer draws the tag
## scaled by SCALE — so the constant has to be 6.0 / SCALE nickname_ui units.
func _test_plate_gap_is_drawn_px() -> void:
	_expect_near("6 px gap as drawn", 6.0, NameplateLayer.PLATE_GAP * NameplateLayer.SCALE)


func _test_tag_floats_above_the_nametag() -> void:
	var pill := Rect2(100.0, 200.0, 220.0, 44.0)
	var tag_size := Vector2(120.0, 22.0)
	var pos := NameplateLayer._scene_tag_position(pill, tag_size, NameplateLayer.SCALE, true)
	_expect_near(
		"tag stands one gap above the pill",
		pill.position.y - NameplateLayer.PLATE_GAP * NameplateLayer.SCALE,
		pos.y + tag_size.y
	)
	_expect_near("tag is centered on the pill", pill.get_center().x, pos.x + tag_size.x * 0.5)


func _test_tag_takes_the_name_slot_without_a_nametag() -> void:
	var pill := Rect2(100.0, 200.0, 220.0, 44.0)
	var tag_size := Vector2(120.0, 22.0)
	var pos := NameplateLayer._scene_tag_position(pill, tag_size, NameplateLayer.SCALE, false)
	_expect_near(
		"nameless tag keeps the name's vertical position",
		pill.get_center().y,
		pos.y + tag_size.y * 0.5
	)
	_expect(
		"nameless tag is not parked on the pill's bottom", false, pos.y + tag_size.y >= pill.end.y
	)
	_expect_near(
		"nameless tag is centered on the pill", pill.get_center().x, pos.x + tag_size.x * 0.5
	)


## The placement rules read the plate's real rects: the pill is what the design counts
## (the plate's own rect starts above it, at a transparent margin) and it stays
## measurable while the plate is hidden — the own-nametag-off case, where the scene tag
## is the only thing drawn. One `await` only: the suite runs with `--quit`, which ends
## the process after the first frame, and containers sort their children on that frame.
func _async_test_real_plate_slot() -> void:
	var ui = load("res://src/decentraland_components/nickname_ui.tscn").instantiate()
	add_child(ui)
	ui.nickname = "Hanked"
	ui.tag = "1234"
	ui.size = ui.get_combined_minimum_size()
	ui.scale = Vector2(NameplateLayer.SCALE, NameplateLayer.SCALE)
	await get_tree().process_frame

	var pill: Rect2 = ui.panel.get_global_rect()
	_expect("plate has a pill", true, pill.size.x > 0.0 and pill.size.y > 0.0)
	_expect("pill top is below the plate's rect", true, pill.position.y > ui.position.y)
	_expect(
		"the name's slot is the pill's centre",
		true,
		absf(pill.get_center().y - ui.nickname_tag.get_global_rect().get_center().y) <= 1.0
	)

	var tag := _make()
	var tag_size: Vector2 = tag.size * NameplateLayer.SCALE
	ui.hide()
	var hidden_pill: Rect2 = ui.panel.get_global_rect()
	_expect("a hidden plate still measures", pill.size, hidden_pill.size)
	var pos := NameplateLayer._scene_tag_position(
		hidden_pill, tag_size, NameplateLayer.SCALE, false
	)
	_expect_near(
		"own nametag off: tag sits in the name's slot",
		hidden_pill.get_center().y,
		pos.y + tag_size.y * 0.5
	)
	tag.free()
	ui.free()
	_finish()


func _expect(ctx: String, expected: Variant, actual: Variant) -> void:
	_checks += 1
	if expected != actual:
		_failures.append("%s: expected %s, got %s" % [ctx, expected, actual])


func _expect_near(ctx: String, expected: float, actual: float) -> void:
	_checks += 1
	if not is_equal_approx(expected, actual):
		_failures.append("%s: expected ~%f, got %f" % [ctx, expected, actual])


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
