extends Control

@onready var button_highlights = $ColorRect_Background/VBoxContainer/Hbox_Sections/Button_Highlights
@onready var button_places = $ColorRect_Background/VBoxContainer/Hbox_Sections/Button_Places
@onready var button_events = $ColorRect_Background/VBoxContainer/Hbox_Sections/Button_Events
@onready var button_favorites = $ColorRect_Background/VBoxContainer/Hbox_Sections/Button_Favorites
@onready var vbox_highlights = $ColorRect_Background/VBoxContainer/Control/Vbox_Highlights
@onready var vbox_places = $ColorRect_Background/VBoxContainer/Control/Vbox_Places
@onready var vbox_events = $ColorRect_Background/VBoxContainer/Control/Vbox_Events
@onready var vbox_favorites = $ColorRect_Background/VBoxContainer/Control/Vbox_Favorites


func _ready():
	button_highlights.button_pressed = true
	vbox_highlights.show()
	vbox_places.hide()
	vbox_events.hide()
	vbox_favorites.hide()


func hide_all():
	vbox_highlights.hide()
	vbox_places.hide()
	vbox_events.hide()
	vbox_favorites.hide()


func _on_button_favorites_pressed():
	hide_all()
	vbox_favorites.show()


func _on_button_events_pressed():
	hide_all()
	vbox_events.show()


func _on_button_places_pressed():
	hide_all()
	vbox_places.show()


func _on_button_highlights_pressed():
	hide_all()
	vbox_highlights.show()
