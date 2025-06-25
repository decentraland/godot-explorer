extends Control

@onready var panel_nearby_player_item: Panel = %Panel_NearbyPlayerItem
@onready var mic_enabled: MarginContainer = %MicEnabled
@onready var nickname: Label = %Nickname
@onready var hash: HBoxContainer = %Hash
@onready var tag: Label = %Tag
@onready var claimed_checkmark: MarginContainer = %ClaimedCheckmark
@onready var profile_picture: ProfilePicture = %ProfilePicture

func async_set_data(avatar):
	await profile_picture.async_update_profile_picture(avatar)
	var avatar_name = avatar.get_avatar_name()
	var splitted_nickname = avatar_name.split("#", false)
	if splitted_nickname.size() > 1:
		nickname.text = splitted_nickname[0]
		tag.text = splitted_nickname[1]
		tag.show()
		hash.show()
		claimed_checkmark.hide()
	else:
		if avatar_name == "":
			hide()
		else:
			nickname.text = avatar_name
			tag.text = ""
			tag.hide()
			hash.hide()
			claimed_checkmark.show()
	var nickname_color = avatar.get_nickname_color(avatar_name)
	nickname.add_theme_color_override("font_color", nickname_color)

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
