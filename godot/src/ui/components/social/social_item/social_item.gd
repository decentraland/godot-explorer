extends Control

enum SocialType { ONLINE, OFFLINE, REQUEST, NEARBY, BLOCKED }

@export var item_type: SocialType

var mute_icon = load("res://assets/ui/audio_off.svg")
var unmute_icon = load("res://assets/ui/audio_on.svg")
var social_data: SocialItemData

@onready var h_box_container_online: HBoxContainer = %HBoxContainer_Online
@onready var h_box_container_nearby: HBoxContainer = %HBoxContainer_Nearby
@onready var h_box_container_request: HBoxContainer = %HBoxContainer_Request
@onready var h_box_container_blocked: HBoxContainer = %HBoxContainer_Blocked
@onready var panel_nearby_player_item: Panel = %Panel_NearbyPlayerItem
@onready var mic_enabled: MarginContainer = %MicEnabled
@onready var nickname: Label = %Nickname
@onready var label_place: Label = %Label_Place
@onready var profile_picture: ProfilePicture = %ProfilePicture
@onready var v_box_container_nickname: VBoxContainer = %VBoxContainer_Nickname
@onready var texture_rect_claimed_checkmark: TextureRect = %TextureRect_ClaimedCheckmark
@onready var button_add_friend: Button = %Button_AddFriend
@onready var button_mute: Button = %Button_Mute


func _ready():
	add_to_group("blacklist_ui_sync")
	_update_elements_visibility()


func set_data(data: SocialItemData) -> void:
	social_data = data
	var tag_position = data.name.find("#")
	if tag_position != -1:
		data.name = data.name.left(tag_position)
		texture_rect_claimed_checkmark.hide()
	else:
		texture_rect_claimed_checkmark.show()

	if data.name.length() > 15:
		data.name = data.name.left(15) + "..."
	nickname.text = data.name

	#var nickname_color = avatar.get_nickname_color(data.name)
	#nickname.add_theme_color_override("font_color", nickname_color)
	#if data.has_claimed_name:
	#texture_rect_claimed_checkmark.show()
	#else:
	#texture_rect_claimed_checkmark.hide()
	profile_picture.async_update_profile_picture(data.name, data.profile_picture_url)


func set_data_from_avatar(avatar_param: DclAvatar):
	social_data = SocialItemData.new()
	social_data.name = avatar_param.get_avatar_name()
	social_data.address = avatar_param.avatar_id
	social_data.profile_picture_url = (
		avatar_param.get_avatar_data().to_godot_dictionary()["snapshots"]["face256"]
	)
	social_data.has_claimed_name = false
	set_data(social_data)
	print(social_data.profile_picture_url, "FACE256")


func _on_mouse_entered() -> void:
	panel_nearby_player_item.self_modulate = "#ffffff"


func _on_mouse_exited() -> void:
	panel_nearby_player_item.self_modulate = "#ffffff00"


func _on_button_mute_toggled(toggled_on: bool) -> void:
	if toggled_on:
		Global.social_blacklist.add_muted(social_data.address)
	else:
		Global.social_blacklist.remove_muted(social_data.address)
	_update_buttons()
	_notify_other_components_of_change()


func _update_buttons() -> void:
	var is_muted = Global.social_blacklist.is_muted(social_data.address)
	button_mute.set_pressed_no_signal(is_muted)
	if is_muted:
		button_mute.icon = mute_icon
	else:
		button_mute.icon = unmute_icon


func _notify_other_components_of_change() -> void:
	if social_data.address:
		Global.get_tree().call_group("blacklist_ui_sync", "_sync_blacklist_ui", social_data.address)


func _sync_blacklist_ui(changed_avatar_id: String) -> void:
	if social_data.address == changed_avatar_id:
		call_deferred("_update_buttons")


func _tap_to_open_profile(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		print(social_data)
		if event.pressed:
			Global.open_profile_by_address.emit(social_data.address)


func _update_elements_visibility() -> void:
	_hide_all_buttons()
	match item_type:
		SocialType.NEARBY:
			h_box_container_nearby.show()
		SocialType.ONLINE:
			h_box_container_online.show()
			label_place.show()
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
	label_place.hide()


func set_type(type: SocialType) -> void:
	item_type = type
	_update_elements_visibility()


func _on_button_add_friend_pressed() -> void:
	print(
		"TODO: Emit signal to friends manager to send friend request to the avatar: ",
		social_data.address
	)
	button_add_friend.hide()


func _on_button_jump_in_pressed() -> void:
	#Global.teleport_to(avatar.get_current_parcel_position(), Realm.MAIN_REALM)
	pass


func update_location() -> void:
	#var pos = avatar.get_current_parcel_position()
	#var url: String = "https://places.decentraland.org/api/places?limit=1"
	#url += "&positions=%d,%d" % [pos.x, pos.y]
#
	#var headers = {"Content-Type": "application/json"}
	#var promise: Promise = Global.http_requester.request_json(
	#url, HTTPClient.METHOD_GET, "", headers
	#)
	#var result = await PromiseUtils.async_awaiter(promise)
#
	#if result is PromiseError:
	#printerr("Error request places jump in", result.get_error())
	#return
#
	#var json: Dictionary = result.get_string_response_as_json()
#
	#if json.data.is_empty():
	#label_place.text = "Unknown place"
	#else:
	#label_place.text = json.data[0].get("title", "Unknown place")
	pass


func _on_button_unblock_pressed() -> void:
	Global.social_blacklist.remove_blocked(social_data.address)
	# Actualizar la lista contenedora
	var parent_list = get_parent()
	if parent_list != null and parent_list.has_method("async_update_list"):
		parent_list.async_update_list()
	_notify_other_components_of_change()
