extends Control

const MUTE = preload("res://assets/ui/audio_off.svg")
const UNMUTE = preload("res://assets/ui/audio_on.svg")
const BLOCK = preload("res://assets/ui/block.svg")
const UNBLOCK = preload("res://assets/ui/unblock.svg")

var avatar: DclAvatar = null
var is_marquee_active: bool = false
var marquee_tween: Tween
var marquee_speed: float = 60.0
var pause_duration: float = 2

@onready var panel_nearby_player_item: Panel = %Panel_NearbyPlayerItem
@onready var mic_enabled: MarginContainer = %MicEnabled
@onready var nickname: Label = %Nickname
@onready var scroll_container_nickname: ScrollContainer = %ScrollContainer_Nickname
@onready var hash_container: HBoxContainer = %Hash
@onready var tag: Label = %Tag
@onready var profile_picture: ProfilePicture = %ProfilePicture
@onready var button_block_user: Button = %Button_BlockUser
@onready var button_mute_user: Button = %Button_MuteUser
@onready var v_box_container_nickname: VBoxContainer = %VBoxContainer_Nickname
@onready var texture_rect_claimed_checkmark: TextureRect = %TextureRect_ClaimedCheckmark


func _ready():
	profile_picture.gui_input.connect(_on_profile_picture_gui_input)
	add_to_group("blacklist_ui_sync")


func _on_profile_picture_gui_input(event: InputEvent):
	if event is InputEventMouseButton:
		#if event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
		if avatar != null and is_instance_valid(avatar):
			Global.open_profile.emit(avatar)


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

	call_deferred("check_and_start_marquee")


func _on_mouse_entered() -> void:
	panel_nearby_player_item.self_modulate = "#ffffff"


func _on_mouse_exited() -> void:
	panel_nearby_player_item.self_modulate = "#ffffff00"


func is_text_overflowing() -> bool:
	return nickname.size.x > scroll_container_nickname.size.x


func start_marquee_effect() -> void:
	if is_marquee_active:
		return

	is_marquee_active = true

	var max_scroll_distance = nickname.size.x - scroll_container_nickname.size.x
	if max_scroll_distance <= 0:
		return

	var scroll_duration = max_scroll_distance / marquee_speed

	if marquee_tween:
		marquee_tween.kill()

	nickname.position.x = 0

	marquee_tween = create_tween()
	marquee_tween.set_loops()
	marquee_tween.set_trans(Tween.TRANS_LINEAR)
	marquee_tween.set_ease(Tween.EASE_IN_OUT)

	marquee_tween.tween_interval(pause_duration)
	marquee_tween.tween_property(nickname, "position:x", -max_scroll_distance, scroll_duration)
	marquee_tween.tween_interval(pause_duration)
	marquee_tween.tween_property(nickname, "position:x", 0, scroll_duration)


func check_and_start_marquee() -> void:
	if is_text_overflowing():
		start_marquee_effect()
	else:
		nickname.position.x = 0


func stop_marquee_effect() -> void:
	if not is_marquee_active:
		return

	is_marquee_active = false

	if marquee_tween:
		marquee_tween.kill()
		marquee_tween = null

	nickname.position.x = 0


func _on_button_mute_user_toggled(toggled_on: bool) -> void:
	if toggled_on:
		Global.social_blacklist.add_muted(avatar.avatar_id)
	else:
		Global.social_blacklist.remove_muted(avatar.avatar_id)
	_update_buttons()
	_notify_other_components_of_change()


func _update_buttons() -> void:
	var is_blocked = Global.social_blacklist.is_blocked(avatar.avatar_id)
	button_block_user.set_pressed_no_signal(is_blocked)
	if is_blocked:
		button_block_user.icon = null
		button_block_user.text = "UNBLOCK"
		button_mute_user.hide()
	else:
		button_block_user.icon = BLOCK
		button_block_user.text = ""
		button_mute_user.show()

	var is_muted = Global.social_blacklist.is_muted(avatar.avatar_id)
	button_mute_user.set_pressed_no_signal(is_muted)
	if is_muted:
		button_mute_user.icon = MUTE
	else:
		button_mute_user.icon = UNMUTE


func _exit_tree() -> void:
	stop_marquee_effect()


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
