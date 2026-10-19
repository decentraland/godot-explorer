extends RefCounted

# Regression test for the --dev-intent overlay (docs/dev-intent.md): the card must render
# one tickable row per expected step, and every container except those rows must stay
# click-through, or the card eats clicks meant for the scene it was opened to explain
# (issue #2578; lib/src/hud_input_lint.rs checks the same in the .tscn).

const PANEL_SCENE := preload(
	"res://src/ui/components/organisms/dev_intent_panel/dev_intent_panel.tscn"
)
const OVERLAY_SCRIPT := preload(
	"res://src/ui/components/organisms/dev_intent_panel/dev_intent_overlay.gd"
)

var suite_name := "dev_intent_panel"
var method_name := "test_steps_and_click_through"
var errors: Array[String] = []
var execution_time_seconds := 0.0


func run() -> bool:
	var start := Time.get_ticks_usec()
	var ok := true

	# Registered in project.godot, so the card sits above the whole UI, not only the scene.
	ok = _expect(root_has_autoload(), true, "DevIntent autoload is registered") and ok

	var panel := PANEL_SCENE.instantiate()
	Engine.get_main_loop().root.add_child(panel)
	panel.apply_intent(
		"Head IK fix", PackedStringArray(["Open two clients", "Look down", "Neck follows"])
	)

	var rows: Array[Node] = panel.get_node("%VBoxContainer_Steps").get_children()
	ok = _expect(rows.size(), 3, "one row per step") and ok
	var checkboxes := 0
	for row in rows:
		if row is CheckBox:
			checkboxes += 1
	ok = _expect(checkboxes, 3, "every row is a CheckBox") and ok
	ok = _expect(panel.get_node("%Label_Title").text, "Head IK fix", "title is shown") and ok

	ok = (
		_expect(panel.mouse_filter, Control.MOUSE_FILTER_IGNORE, "card background is click-through")
		and ok
	)
	ok = (
		_expect(
			panel.get_node("%VBoxContainer_Steps").mouse_filter,
			Control.MOUSE_FILTER_IGNORE,
			"steps container is click-through"
		)
		and ok
	)
	ok = (
		_expect(rows[0].mouse_filter, Control.MOUSE_FILTER_STOP, "the step row claims the tap")
		and ok
	)

	panel.queue_free()

	# The close button hid the card, and F8 is the only way back to it: a card that cannot be
	# reopened after X is worse than no card.
	var overlay := OVERLAY_SCRIPT.new()
	var probe := PANEL_SCENE.instantiate()
	Engine.get_main_loop().root.add_child(overlay)
	Engine.get_main_loop().root.add_child(probe)
	probe.get_node("%Button_Close").pressed.emit()
	ok = _expect(probe.visible, false, "close button hides the card") and ok
	# Same slot the overlay fills from `Global.cli`; the test has no CLI payload.
	overlay._card = probe
	overlay.toggle()
	ok = _expect(probe.visible, true, "F8 shows the card again after the close button") and ok
	overlay.toggle()
	ok = _expect(probe.visible, false, "F8 hides it too") and ok
	overlay.queue_free()
	probe.queue_free()

	execution_time_seconds = (Time.get_ticks_usec() - start) / 1_000_000.0
	return ok


func root_has_autoload() -> bool:
	var root: Window = Engine.get_main_loop().root
	return root.has_node("DevIntent")


func _expect(actual: Variant, expected: Variant, label: String) -> bool:
	if actual != expected:
		errors.append("%s: expected %s, got %s" % [label, expected, actual])
		return false
	return true
