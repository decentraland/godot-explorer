extends RefCounted

# Regression test for the --dev-intent overlay (docs/dev-intent.md): the card must render
# one tickable row per expected step, and every container except those rows must stay
# click-through, or the card eats clicks meant for the scene it was opened to explain
# (issue #2578; lib/src/hud_input_lint.rs checks the same in the .tscn).

const PANEL_SCENE := preload(
	"res://src/ui/components/organisms/dev_intent_panel/dev_intent_panel.tscn"
)

var suite_name := "dev_intent_panel"
var method_name := "test_steps_and_click_through"
var errors: Array[String] = []
var execution_time_seconds := 0.0


func run() -> bool:
	var start := Time.get_ticks_usec()
	var ok := true

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
	execution_time_seconds = (Time.get_ticks_usec() - start) / 1_000_000.0
	return ok


func _expect(actual: Variant, expected: Variant, label: String) -> bool:
	if actual != expected:
		errors.append("%s: expected %s, got %s" % [label, expected, actual])
		return false
	return true
