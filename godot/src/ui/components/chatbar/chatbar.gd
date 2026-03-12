extends Control

signal share_place


func _ready() -> void:
	Global.change_parcel.connect(_on_change_parcel)


func _on_change_parcel(coordinates: Vector2i) -> void:
	%Label_Coordinates.text = "%d,%d" % [coordinates.x, coordinates.y]


func _on_hud_button_discover_pressed() -> void:
	Global.open_discover.emit()
	Global.send_haptic_feedback()


func _on_hud_button_share_pressed() -> void:
	share_place.emit()
	Global.send_haptic_feedback()


func _on_button_chat_pressed() -> void:
	Global.open_chat.emit()
	Global.send_haptic_feedback()
