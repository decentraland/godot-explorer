extends CanvasLayer

## Tells the human what the agent that launched this client expects them to test
## (`--dev-intent` / `--dev-intent-file`, docs/dev-intent.md). Registered as the `DevIntent`
## autoload, so the card sits above the whole UI (scene HUD, menus, lobby, loading screens)
## instead of only over the scene, and it outlives scene changes.
##
## F8 lives here and not in the card: a hidden node stops receiving input, and the close
## button is one click away from a card the human could never get back.

const CARD_SCENE := preload(
	"res://src/ui/components/organisms/dev_intent_panel/dev_intent_panel.tscn"
)
## Above the scene/UI CanvasLayers (small numbers), below TouchFeedback (128).
const OVERLAY_LAYER := 127

var _card: Control = null


func _ready() -> void:
	layer = OVERLAY_LAYER
	# F8 has to work while a menu or a modal has the tree paused.
	process_mode = Node.PROCESS_MODE_ALWAYS
	set_process_unhandled_key_input(true)

	var title := Global.cli.get_dev_intent_title()
	var steps := Global.cli.get_dev_intent_steps()
	# Nothing to say when the client was opened without an intent.
	if title.is_empty() and steps.is_empty():
		return

	_card = CARD_SCENE.instantiate()
	add_child(_card)
	_card.apply_intent(title, steps)


## Hide/show the card. No other HUD shortcut uses F8.
func toggle() -> void:
	if is_instance_valid(_card):
		_card.visible = not _card.visible


func _unhandled_key_input(event: InputEvent) -> void:
	if event is InputEventKey and event.pressed and not event.echo and event.keycode == KEY_F8:
		toggle()
		if is_instance_valid(_card):
			get_viewport().set_input_as_handled()
