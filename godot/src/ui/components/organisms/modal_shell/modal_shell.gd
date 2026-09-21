class_name ModalShell
extends ColorRect

## Reusable modal chrome: dim backdrop, blur, rounded textured panel, safe
## padding and a content slot.
##
## Every modal in this project had been re-declaring the same subtree
## (`ColorRect` + blurred `ColorRect` + `ResponsiveContainer` panel +
## `PanelContainer_Background` + `MarginContainer` + `VBoxContainer`), each with
## its own inline copy of the six-line screen-blur shader. This centralises it.
##
## Use it as an INHERITED scene: make your modal's scene an instance of
## `modal_shell.tscn`, give it a script extending `ModalShell`, and parent your
## own nodes under `%ContentContainer`. See `bug_report_modal.tscn`.
##
## Existing modals (input_modal, code_modal, modal, reward_modal, upgrade_modal)
## deliberately have NOT been migrated — that is a follow-up, kept separate so a
## UI change to one feature can't regress five other screens.

## Emitted when the user dismisses via the backdrop. Buttons inside the content
## should emit their own signals; this only covers tap-outside.
signal dismissed

## When false, tapping the backdrop does nothing — for flows that must not be
## abandoned by a stray tap (e.g. while a request is in flight).
@export var dismissable: bool = true

@onready var content_container: VBoxContainer = %ContentContainer
@onready var panel: PanelContainer = %Panel


func _ready() -> void:
	Global.change_virtual_keyboard.connect(_on_virtual_keyboard_changed)


func _on_gui_input(event: InputEvent) -> void:
	if not dismissable:
		return
	if event is InputEventScreenTouch and event.pressed:
		dismissed.emit()


## Lifts the panel clear of the on-screen keyboard. Mirrors InputModal, which
## solved this first; the maths is deliberately identical.
func _on_virtual_keyboard_changed(keyboard_height: int) -> void:
	if not is_instance_valid(panel):
		return
	if keyboard_height == 0:
		panel.vertical_offset = 0.0
		return
	var viewport_size := get_viewport().get_visible_rect().size
	var window_size := Vector2(DisplayServer.window_get_size())
	var y_factor := viewport_size.y / window_size.y
	panel.vertical_offset = -keyboard_height * y_factor * 0.5


func _on_visibility_changed() -> void:
	if not visible and is_instance_valid(panel):
		panel.vertical_offset = 0.0
