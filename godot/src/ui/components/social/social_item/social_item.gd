extends Control

var mute_icon = load("res://assets/ui/audio_off.svg")
var unmute_icon = load("res://assets/ui/audio_on.svg")
var block_icon = load("res://assets/ui/block.svg")
var unblock_icon = load("res://assets/ui/unblock.svg")


@export var item_type: SocialType

enum SocialType { ONLINE, OFFLINE, REQUEST, NEARBY, BLOCKED }

var avatar: DclAvatar = null

@onready var h_box_container_online: HBoxContainer = %HBoxContainer_Online
@onready var h_box_container_nearby: HBoxContainer = %HBoxContainer_Nearby
@onready var h_box_container_request: HBoxContainer = %HBoxContainer_Request
@onready var h_box_container_blocked: HBoxContainer = %HBoxContainer_Blocked

@onready var panel_nearby_player_item: Panel = %Panel_NearbyPlayerItem
@onready var mic_enabled: MarginContainer = %MicEnabled
@onready var nickname: Label = %Nickname
@onready var hash_container: HBoxContainer = %Hash
@onready var tag: Label = %Tag
@onready var profile_picture: ProfilePicture = %ProfilePicture
#@onready var button_block_user: Button = %Button_BlockUser
#@onready var button_mute_user: Button = %Button_MuteUser
@onready var v_box_container_nickname: VBoxContainer = %VBoxContainer_Nickname
@onready var texture_rect_claimed_checkmark: TextureRect = %TextureRect_ClaimedCheckmark
@onready var texture_rect_status: TextureRect = %TextureRect_Status


func _ready():
	add_to_group("blacklist_ui_sync")
	_update_elements_visibility()


func async_set_data(avatar_param = null):
	if avatar_param != null:
		avatar = avatar_param

	elif avatar == null:
		return

	if not is_instance_valid(avatar):
		return

	var avatar_data = avatar.get_avatar_data()
	if avatar_data != null:
		profile_picture.async_update_profile_picture(avatar)
	else:
		printerr("NO AVATAR DATA")

	if !avatar.finish_loading:
		hide()
	else:
		show()
		var avatar_name = avatar.get_avatar_name()
		var tag_position = avatar_name.find("#")
		if tag_position != -1:
			nickname.text = avatar_name.left(tag_position)
			texture_rect_claimed_checkmark.hide()
		else:
			nickname.text = avatar_name
			texture_rect_claimed_checkmark.show()

		tag.text = avatar.avatar_id.right(4)
		_update_buttons()

		var nickname_color = avatar.get_nickname_color(avatar_name)
		nickname.add_theme_color_override("font_color", nickname_color)


func _on_mouse_entered() -> void:
	panel_nearby_player_item.self_modulate = "#ffffff"


func _on_mouse_exited() -> void:
	panel_nearby_player_item.self_modulate = "#ffffff00"


func _on_button_mute_user_toggled(toggled_on: bool) -> void:
	if toggled_on:
		Global.social_blacklist.add_muted(avatar.avatar_id)
	else:
		Global.social_blacklist.remove_muted(avatar.avatar_id)
	_update_buttons()
	_notify_other_components_of_change()


func _update_buttons() -> void:
	pass
	#var is_blocked = Global.social_blacklist.is_blocked(avatar.avatar_id)
	#button_block_user.set_pressed_no_signal(is_blocked)
	#if is_blocked:
		#button_block_user.icon = null
		#button_block_user.text = "UNBLOCK"
		#button_mute_user.hide()
	#else:
		#button_block_user.icon = BLOCK
		#button_block_user.text = ""
		#button_mute_user.show()
#
	#var is_muted = Global.social_blacklist.is_muted(avatar.avatar_id)
	#button_mute_user.set_pressed_no_signal(is_muted)
	#if is_muted:
		#button_mute_user.icon = MUTE
	#else:
		#button_mute_user.icon = UNMUTE


func _on_button_block_user_pressed() -> void:
	var is_blocked = Global.social_blacklist.is_blocked(avatar.avatar_id)
	if is_blocked:
		Global.social_blacklist.remove_blocked(avatar.avatar_id)
	else:
		Global.social_blacklist.add_blocked(avatar.avatar_id)
	_update_buttons()
	_notify_other_components_of_change()


func _notify_other_components_of_change() -> void:
	if avatar != null:
		Global.get_tree().call_group("blacklist_ui_sync", "_sync_blacklist_ui", avatar.avatar_id)


func _sync_blacklist_ui(changed_avatar_id: String) -> void:
	if avatar != null and avatar.avatar_id == changed_avatar_id:
		call_deferred("_update_buttons")


func _tap_to_open_profile(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		if event.pressed:
			if avatar != null and is_instance_valid(avatar):
				var explorer = Global.get_explorer()
				if avatar.avatar_id == Global.player_identity.get_address_str():
					explorer.control_menu.show_own_profile()
				else:
					Global.open_profile.emit(avatar)


func _on_button_report_pressed() -> void:
	pass  # Replace with function body.


func _update_elements_visibility() -> void:
	_hide_all_buttons()
	match item_type:
		SocialType.NEARBY:
			h_box_container_nearby.show()
		SocialType.ONLINE:
			h_box_container_online.show()
			profile_picture.set_online()
		SocialType.REQUEST:
			h_box_container_request.show()
		SocialType.BLOCKED:
			h_box_container_blocked.show()
		_:
			profile_picture.set_offline()
		

func _hide_all_buttons() -> void:
	h_box_container_online.hide()
	h_box_container_nearby.hide()
	h_box_container_request.hide()
	h_box_container_blocked.hide()
	profile_picture.hide_status()
	
func set_type(type:SocialType) -> void:
	item_type = type
	_update_elements_visibility()
