extends Control

@onready var panel_nearby_player_item: Panel = %Panel_NearbyPlayerItem

func _on_mouse_entered() -> void:
	panel_nearby_player_item.self_modulate = "#ffffff"


func _on_mouse_exited() -> void:
	panel_nearby_player_item.self_modulate = "#ffffff00"


func _on_button_block_user_pressed() -> void:
	print('block')


func _on_button_mute_pressed() -> void:
	print('mute')


func _on_button_report_pressed() -> void:
	print('report')
