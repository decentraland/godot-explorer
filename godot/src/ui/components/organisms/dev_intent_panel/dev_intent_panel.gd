extends PanelContainer

## Card that tells the human what the agent that launched this client expects them to test
## (`--dev-intent` / `--dev-intent-file`, see docs/dev-intent.md): the title, one tickable
## row per expected step and a reminder of the shortcut. It exists because the prompt
## usually arrives in a chat while the game window has focus, and it is gone from sight by
## the time the human is looking at the bug. Any tool can read the same payload from
## `Global.cli` (get_dev_intent / get_dev_intent_title / get_dev_intent_steps); Global
## itself has no room left under the gdlint public-method cap.
##
## Everything but the checkboxes and the close button is MOUSE_FILTER_IGNORE, so the card
## never steals clicks from the scene (issue #2578; lib/src/hud_input_lint.rs). Ticks live
## in the CheckBox nodes, which are not rebuilt on hide, so F8 keeps them.

const STEP_NAME_FORMAT := "CheckBox_Step%d"

@onready var label_title: Label = %Label_Title
@onready var vbox_steps: VBoxContainer = %VBoxContainer_Steps
@onready var button_close: Button = %Button_Close


func _ready() -> void:
	button_close.pressed.connect(_on_button_close_pressed)
	var title := Global.cli.get_dev_intent_title()
	var steps := Global.cli.get_dev_intent_steps()
	apply_intent(title, steps)
	# Nothing to say when the client was opened without an intent.
	visible = not title.is_empty() or not steps.is_empty()


## F8 hides/shows the card. No other HUD shortcut uses it.
func _unhandled_key_input(event: InputEvent) -> void:
	if event is InputEventKey and event.pressed and not event.echo and event.keycode == KEY_F8:
		visible = not visible
		get_viewport().set_input_as_handled()


## Rebuild the step rows. `_ready` calls it with the resolved CLI intent; the headless test
## (src/test/ui/test_dev_intent_panel.gd) calls it directly, since it has no CLI payload.
func apply_intent(title: String, steps: PackedStringArray) -> void:
	label_title.text = title
	for child in vbox_steps.get_children():
		vbox_steps.remove_child(child)
		child.queue_free()
	for i in steps.size():
		var checkbox := CheckBox.new()
		checkbox.name = STEP_NAME_FORMAT % (i + 1)
		checkbox.text = "%d. %s" % [i + 1, steps[i]]
		checkbox.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		# Explicit: every container above is IGNORE, the row itself must claim the tap.
		checkbox.mouse_filter = Control.MOUSE_FILTER_STOP
		vbox_steps.add_child(checkbox)


func _on_button_close_pressed() -> void:
	hide()
