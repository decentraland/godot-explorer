extends Node

# Regression test for --dev-intent (docs/dev-intent.md): one tickable row per step, the card
# click-through everywhere but those rows, and F8 able to bring it back after X — a hidden
# node stops receiving input, which is exactly how the card shipped unopenable once.
#
# Runs as a scene, not --script: the card and the overlay reach Global.cli, and a --script run
# compiles before the autoloads exist, so every call silently becomes a no-op (same reason as
# test_destination). _checks is the guard against that.

const PANEL_SCENE := preload(
	"res://src/ui/components/organisms/dev_intent_panel/dev_intent_panel.tscn"
)
const OVERLAY_SCRIPT := preload(
	"res://src/ui/components/organisms/dev_intent_panel/dev_intent_overlay.gd"
)
const STEPS := ["Open two clients", "Look down", "Neck follows"]

## Assertions that actually evaluated. A script that fails to compile leaves _failures empty
## and would otherwise report a green suite that never ran.
const EXPECTED_CHECKS := 12

var _failures: Array[String] = []
var _checks: int = 0


func _ready() -> void:
	_test_card_rows()
	_test_overlay_toggle()
	_finish()


func _test_card_rows() -> void:
	# Registered in project.godot, so the card sits above the whole UI, not only the scene.
	_expect(get_tree().root.has_node("DevIntent"), "the DevIntent autoload is registered")

	var panel := PANEL_SCENE.instantiate()
	add_child(panel)
	panel.apply_intent("Head IK fix", PackedStringArray(STEPS))

	var rows: Array[Node] = panel.get_node("%VBoxContainer_Steps").get_children()
	_expect(rows.size() == 3, "one row per step, got %d" % rows.size())
	var checkboxes := 0
	for row in rows:
		if row is CheckBox:
			checkboxes += 1
	_expect(checkboxes == 3, "every row is a CheckBox, got %d" % checkboxes)
	_expect(panel.get_node("%Label_Title").text == "Head IK fix", "the title is shown")
	_expect(
		panel.mouse_filter == Control.MOUSE_FILTER_IGNORE, "the card background is click-through"
	)
	_expect(
		panel.get_node("%VBoxContainer_Steps").mouse_filter == Control.MOUSE_FILTER_IGNORE,
		"the steps container is click-through"
	)
	_expect(
		rows[0].mouse_filter == Control.MOUSE_FILTER_STOP,
		"the step row claims the tap, or it cannot be ticked"
	)
	panel.queue_free()


func _test_overlay_toggle() -> void:
	var overlay: CanvasLayer = OVERLAY_SCRIPT.new()
	add_child(overlay)
	# The payload the autoload resolves from Global.cli; this run passed no flag.
	overlay.attach_card("Head IK fix", PackedStringArray(STEPS))

	var card: Control = overlay.card()
	_expect(card != null, "attach_card builds the card")
	if card == null:
		overlay.queue_free()
		return

	card.get_node("%Button_Close").pressed.emit()
	_expect(not card.visible, "the close button hides the card")
	overlay.toggle()
	_expect(card.visible, "F8 shows the card again after the close button")
	overlay.toggle()
	_expect(not card.visible, "F8 hides it too")

	var row: CheckBox = card.get_node("%VBoxContainer_Steps").get_child(0)
	overlay.toggle()
	row.button_pressed = true
	overlay.toggle()
	overlay.toggle()
	_expect(row.button_pressed, "the tick survives hiding and showing")
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
		print("[test_dev_intent_panel] PASS")
		get_tree().quit(0)
		return
	for failure in _failures:
		printerr(failure)
	printerr("[test_dev_intent_panel] FAIL: %d case(s)" % _failures.size())
	get_tree().quit(1)
