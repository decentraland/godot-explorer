extends Control
@onready var h_box_container_about_1: HBoxContainer = %HBoxContainer_About1
@onready var label_no_links: Label = %Label_NoLinks
@onready var label_editing_links: Label = %Label_EditingLinks
@onready var scroll_container: ScrollContainer = %ScrollContainer
@onready var h_flow_container_about: HFlowContainer = %HFlowContainer_About


func _ready() -> void:
	scroll_container.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	_on_button_edit_about_toggled(false)
	_on_button_edit_links_toggled(false) 


func _on_button_edit_about_toggled(toggled_on: bool) -> void:
	for child in h_box_container_about_1.get_children():
		child.emit_signal('change_editing', toggled_on)
	for child in h_flow_container_about.get_children():
		child.emit_signal('change_editing', toggled_on)


func _on_button_edit_links_toggled(toggled_on: bool) -> void:
	if toggled_on:
		label_editing_links.show()
	else:
		label_editing_links.hide()
