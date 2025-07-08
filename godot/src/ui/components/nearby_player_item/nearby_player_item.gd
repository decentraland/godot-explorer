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



func async_set_data(avatar_param = null):
	if avatar_param != null:
		avatar = avatar_param
		_update_buttons()

	elif avatar == null:
		return
	
	# Verificar que el avatar es válido antes de acceder a él
	if not is_instance_valid(avatar):
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


	var position = avatar_name.find("#")
	if position != -1:
		nickname.text = avatar_name.left(position)
		texture_rect_claimed_checkmark.hide()
	else:
		nickname.text = avatar_name
		texture_rect_claimed_checkmark.show()

	tag.text = avatar.avatar_id.right(4)
	

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


func _on_button_report_pressed() -> void:
	print("Report ", avatar.avatar_id, " (", avatar.get_avatar_name(), ")")


func _on_button_block_user_toggled(toggled_on: bool) -> void:
	var profile: DclUserProfile = Global.player_identity.get_profile_or_null()
	if toggled_on:
		profile.add_blocked(avatar.avatar_id)
	else:
		profile.remove_blocked(avatar.avatar_id)
	_update_buttons()

func _on_button_mute_user_toggled(toggled_on: bool) -> void:
	var profile: DclUserProfile = Global.player_identity.get_profile_or_null()
	if toggled_on:
		profile.add_muted(avatar.avatar_id)
	else:
		profile.remove_muted(avatar.avatar_id)
	_update_buttons()
	

func _update_buttons() -> void:
	var profile: DclUserProfile = Global.player_identity.get_profile_or_null()
	
	var is_blocked = profile.is_blocked(avatar.avatar_id)
	button_block_user.set_pressed_no_signal(is_blocked)
	if is_blocked:
		button_block_user.icon = BLOCK
	else:
		button_block_user.icon = UNBLOCK
	
	var is_muted = profile.is_muted(avatar.avatar_id)
	button_mute_user.set_pressed_no_signal(is_muted)
	if is_muted:
		button_mute_user.icon = MUTE
	else:
		button_mute_user.icon = UNMUTE
	
	
func _exit_tree() -> void:
	stop_marquee_effect()
