@tool
extends VBoxContainer

signal link_clicked(url: String)

const PROFILE_LINK_BUTTON = preload("res://src/ui/components/profile/profile_link_button.tscn")

@onready var h_flow_container_links: HFlowContainer = %HFlowContainer_Links


func refresh(profile: DclUserProfile) -> void:
	if Engine.is_editor_hint():
		return
	if profile == null:
		return
	var children_to_remove = []
	for child in h_flow_container_links.get_children():
		if child.is_in_group("profile_link_buttons"):
			children_to_remove.append(child)
	for child in children_to_remove:
		h_flow_container_links.remove_child(child)
		child.queue_free()
	var links = profile.get_links()
	for link in links:
		_instantiate_link_button(link.title, link.url)
	visible = links.size() > 0


func _instantiate_link_button(title: String, url: String) -> void:
	var new_link_button = PROFILE_LINK_BUTTON.instantiate()
	h_flow_container_links.add_child(new_link_button)
	new_link_button.try_open_link.connect(func(link_url): link_clicked.emit(link_url))
	new_link_button.text = title
	new_link_button.url = url
	new_link_button.emit_signal("change_editing", false)
