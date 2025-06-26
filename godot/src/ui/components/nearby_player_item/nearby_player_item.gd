extends Control

var avatar:DclAvatar = null

@onready var panel_nearby_player_item: Panel = %Panel_NearbyPlayerItem
@onready var mic_enabled: MarginContainer = %MicEnabled
@onready var nickname: Label = %Nickname
@onready var hash_container: HBoxContainer = %Hash
@onready var tag: Label = %Tag
@onready var claimed_checkmark: MarginContainer = %ClaimedCheckmark
@onready var profile_picture: ProfilePicture = %ProfilePicture

func async_set_data(avatar_param = null):
	if avatar_param != null:
		avatar = avatar_param
	elif avatar == null:
		return
	
	# Actualizar imagen de perfil solo si el avatar tiene datos de snapshots
	var avatar_data = avatar.get_avatar_data()
	if avatar_data != null:
		var face256_hash = avatar_data.get_snapshots_face_hash()
		var face256_url = avatar_data.get_snapshots_face_url()
		
		# Solo intentar cargar la imagen si tenemos datos válidos
		if not face256_hash.is_empty() and not face256_url.is_empty():
			profile_picture.async_update_profile_picture(avatar)
	
	var avatar_name = avatar.get_avatar_name()

	if avatar_name.is_empty():
		print("Avatar name is empty, hiding item temporarily")
		nickname.text = "Loading..."
		tag.text = ""
		tag.hide()
		hash_container.hide()
		claimed_checkmark.hide()
		return
	
	var splitted_nickname = avatar_name.split("#", false)
	if splitted_nickname.size() > 1:
		nickname.text = splitted_nickname[0]
		tag.text = splitted_nickname[1]
		tag.show()
		hash_container.show()
		claimed_checkmark.hide()
	else:
		nickname.text = avatar_name
		tag.text = ""
		tag.hide()
		hash_container.hide()
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
	var avatar_position = avatar.get_current_parcel_position()
	print(avatar_position)

func _on_button_report_pressed() -> void:
	var avatar_name = avatar.get_avatar_name()
	var splitted_nickname = avatar_name.split("#", false)
	print(splitted_nickname)
	async_set_data(avatar)
