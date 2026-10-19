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
## in the CheckBox nodes, which are not rebuilt on hide, so they survive F8.
##
## Created by the `DevIntent` autoload (dev_intent_overlay.gd), which owns the payload and
## the shortcut: a hidden card cannot receive the key that would bring it back.

const STEP_NAME_FORMAT := "CheckBox_Step%d"

@onready var label_title: Label = %Label_Title
@onready var vbox_steps: VBoxContainer = %VBoxContainer_Steps
@onready var button_close: Button = %Button_Close


func _ready() -> void:
	button_close.pressed.connect(_on_button_close_pressed)


## Rebuild the step rows. The overlay calls it with the resolved CLI intent; the headless
## test (src/test/ui/test_dev_intent_panel.gd) calls it directly, since it has no payload.
func apply_intent(title: String, steps: PackedStringArray) -> void:
	label_title.text = title
	for child in vbox_steps.get_children():
		vbox_steps.remove_child(child)
		child.queue_free()
	for i in steps.size():
		var checkbox := CheckBox.new()
		checkbox.name = STEP_NAME_FORMAT % (i + 1)
		checkbox.text = "%d. %s" % [i + 1, steps[i]]
		# The step text is the agent's own words, in whatever language it wrote them.
		checkbox.auto_translate_mode = Node.AUTO_TRANSLATE_MODE_DISABLED
		checkbox.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		# Explicit: every container above is IGNORE, the row itself must claim the tap.
		checkbox.mouse_filter = Control.MOUSE_FILTER_STOP
		vbox_steps.add_child(checkbox)


func _on_button_close_pressed() -> void:
	hide()
