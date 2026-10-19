extends CanvasLayer

## Tells the human what the agent that launched this client expects them to test
## (`--dev-intent` / `--dev-intent-file`, docs/dev-intent.md). Registered as the `DevIntent`
## autoload, so the card sits above the whole UI (scene HUD, menus, lobby, loading screens)
## instead of only over the scene, and it outlives scene changes.
##
## F8 lives here and not in the card: a hidden node stops receiving input, and the close
## button would otherwise be a one-way door.

const CARD_SCENE := preload(
	"res://src/ui/components/organisms/dev_intent_panel/dev_intent_panel.tscn"
)
## Above the scene/UI CanvasLayers (small numbers), below TouchFeedback (128).
const OVERLAY_LAYER := 127

var _card: Control = null


func _ready() -> void:
	# Same rule as TouchFeedback: a store build cannot be told to show this.
	if Global.is_production():
		return
	layer = OVERLAY_LAYER
	# F8 has to work while a menu or a modal has the tree paused.
	process_mode = Node.PROCESS_MODE_ALWAYS
	set_process_unhandled_key_input(true)
	attach_card(Global.cli.get_dev_intent_title(), Global.cli.get_dev_intent_steps())


## Build the card for this payload and echo it to stdout, where a log can be grepped.
## The headless test (src/test/ui/test_dev_intent_panel.gd) passes its own text, since a test
## run has no CLI payload.
func attach_card(title: String, steps: PackedStringArray) -> void:
	var file := Global.cli.dev_intent_file
	if not file.is_empty():
		print("[DEV-INTENT] source: --dev-intent-file ", file)
	if title.is_empty() and steps.is_empty():
		return
	_card = CARD_SCENE.instantiate()
	add_child(_card)
	_card.apply_intent(title, steps)
	if not title.is_empty():
		print("[DEV-INTENT] ", title)
	for i in steps.size():
		print("[DEV-INTENT] %d. %s" % [i + 1, steps[i]])


func card() -> Control:
	return _card


## Hide/show the card. No other HUD shortcut uses F8.
func toggle() -> void:
	if is_instance_valid(_card):
		_card.visible = not _card.visible


func _unhandled_key_input(event: InputEvent) -> void:
	if event is InputEventKey and event.pressed and not event.echo and event.keycode == KEY_F8:
		toggle()
		if is_instance_valid(_card):
			get_viewport().set_input_as_handled()
