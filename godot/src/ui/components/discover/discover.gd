class_name Discover
extends Control

var search_text: String = ""

@onready var jump_in = %JumpIn
@onready var event_details: EventDetailWrapper = %EventDetails

@onready var button_search_bar: Button = %Button_SearchBar
@onready var line_edit_search_bar: LineEdit = %LineEdit_SearchBar
@onready var button_clear_filter: Button = %Button_ClearFilter
@onready var timer_search_debounce: Timer = %Timer_SearchDebounce

@onready var last_visited: VBoxContainer = %LastVisited
@onready var places_featured: VBoxContainer = %PlacesFeatured
@onready var places_most_active: VBoxContainer = %PlacesMostActive
@onready var places_worlds: VBoxContainer = %PlacesWorlds
@onready var events: VBoxContainer = %Events


func _ready():
	UiSounds.install_audio_recusirve(self)
	jump_in.hide()
	event_details.hide()
	button_search_bar.show()
	button_clear_filter.hide()
	line_edit_search_bar.hide()


func on_item_pressed(data):
	jump_in.set_data(data)
	jump_in.show_animation()


func on_event_pressed(data):
	event_details.set_data(data)
	event_details.show_animation()


func _on_jump_in_jump_in(parcel_position: Vector2i, realm: String):
	jump_in.hide()
	Global.teleport_to(parcel_position, realm)


func _on_visibility_changed():
	if is_node_ready() and is_inside_tree() and is_visible_in_tree():
		%LastVisitGenerator.request_last_places()


func _on_line_edit_search_bar_focus_exited() -> void:
	button_search_bar.show()
	line_edit_search_bar.hide()


func _on_button_search_bar_pressed() -> void:
	button_search_bar.hide()
	line_edit_search_bar.show()
	line_edit_search_bar.grab_focus()


func _on_button_clear_filter_pressed() -> void:
	search_text = ""
	set_search_filter_text("")
	line_edit_search_bar.text = ""
	timer_search_debounce.stop()


func set_search_filter_text(new_text: String) -> void:
	button_clear_filter.visible = !new_text.is_empty()

	if new_text.is_empty():
		last_visited.show()
		places_featured.show()
	else:
		last_visited.hide()
		places_featured.hide()
	places_most_active.set_search_param(new_text)
	places_worlds.set_search_param(new_text)
	events.set_search_param(new_text)


func _on_line_edit_search_bar_text_changed(new_text: String) -> void:
	search_text = new_text
	timer_search_debounce.start()


func _on_timer_search_debounce_timeout() -> void:
	set_search_filter_text(search_text)


func _on_event_details_jump_in(parcel_position: Vector2i, realm: String) -> void:
	event_details.hide()
	Global.teleport_to(parcel_position, realm)
