extends Control

var avatar:DclAvatar = null

const MUTE = preload("res://assets/ui/audio_off.svg")
const UNMUTE = preload("res://assets/ui/audio_on.svg")
const BLOCK = preload("res://assets/ui/block.svg")
const UNBLOCK = preload("res://assets/ui/unblock.svg")

@onready var panel_nearby_player_item: Panel = %Panel_NearbyPlayerItem
@onready var mic_enabled: MarginContainer = %MicEnabled
@onready var nickname: Label = %Nickname
@onready var hash_container: HBoxContainer = %Hash
@onready var tag: Label = %Tag
@onready var profile_picture: ProfilePicture = %ProfilePicture
@onready var button_block_user: Button = %Button_BlockUser
@onready var button_mute_user: Button = %Button_MuteUser

func async_set_data(avatar_param = null):
	if avatar_param != null:
		avatar = avatar_param
	elif avatar == null:
		return
	var avatar_data = avatar.get_avatar_data()
	if avatar_data != null:
		profile_picture.async_update_profile_picture(avatar)
	else:
		printerr("NO AVATAR DATA")
	
	#TODO: I think this will be redundant when client receive depured avatar list.
	var avatar_name = avatar.get_avatar_name()
	if avatar_name.is_empty():
		print("Deleting element because name is empty")
		queue_free()
	
	var splitted_nickname = avatar_name.split("#", false)
	if splitted_nickname.size() > 1:
		nickname.text = splitted_nickname[0]
		tag.text = splitted_nickname[1]
		tag.show()
		hash_container.show()
	else:
		nickname.text = avatar_name
		tag.text = ""
		tag.hide()
		hash_container.hide()
	
	var nickname_color = avatar.get_nickname_color(avatar_name)
	nickname.add_theme_color_override("font_color", nickname_color)

func _on_mouse_entered() -> void:
	panel_nearby_player_item.self_modulate = "#ffffff"


func _on_mouse_exited() -> void:
	panel_nearby_player_item.self_modulate = "#ffffff00"


func _on_button_report_pressed() -> void:
	print("Report ", avatar.avatar_id, " (", avatar.get_avatar_name(), ")")


func _on_button_block_user_toggled(toggled_on: bool) -> void:
	if toggled_on:
		print("Block ", avatar.avatar_id, " (", avatar.get_avatar_name(), ")")
		button_block_user.icon = BLOCK
	else:
		print("Unblock ", avatar.avatar_id, " (", avatar.get_avatar_name(), ")")
		button_block_user.icon = UNBLOCK

func _on_button_mute_user_toggled(toggled_on: bool) -> void:
	if toggled_on:
		print("Mute ", avatar.avatar_id, " (", avatar.get_avatar_name(), ")")
		button_mute_user.icon = MUTE
	else:
		print("Unmute ", avatar.avatar_id, " (", avatar.get_avatar_name(), ")")
		button_mute_user.icon = UNMUTE
