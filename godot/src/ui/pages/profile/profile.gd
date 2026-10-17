extends Control

signal close_profile

const NICK_MAX_LENGTH: int = 15
const MUTE = preload("res://assets/ui/audio_off.svg")
const UNMUTE = preload("res://assets/ui/audio_on.svg")
const BLOCK = preload("res://assets/ui/block.svg")

@export var rounded: bool = false
@export var closable: bool = false

var url_to_visit: String = ""
var is_own_passport: bool = false
var is_blocked_user: bool = false
var is_muted_user: bool = false
var current_profile: DclUserProfile = null
var address: String = ""
var player_profile = Global.player_identity.get_profile_or_null()

var _deploy_waiting: bool = false
var _deploy_timeout_timer: Timer

@onready var scroll_container: ScrollContainer = %ScrollContainer
@onready var avatar_preview: AvatarPreview = %AvatarPreview
@onready var profile_about: VBoxContainer = %ProfileAbout
@onready var profile_equipped: VBoxContainer = %ProfileEquipped
@onready var profile_links: VBoxContainer = %ProfileLinks
@onready var label_nickname: Label = %Label_Nickname
@onready var label_address: Label = %Label_Address
@onready var texture_rect_claimed_checkmark: TextureRect = %TextureRect_ClaimedCheckmark
@onready var label_tag: Label = %Label_Tag
@onready var button_block_user: Button = %Button_BlockUser

@onready var v_box_container_content: VBoxContainer = %VBoxContainer_Content
@onready var h_box_container_data: HBoxContainer = %HBoxContainer_Data
@onready var h_box_container_placeholder: HBoxContainer = %HBoxContainer_PlaceHolder
@onready var button_mute_user: Button = %Button_MuteUser
@onready var button_edit_profile: Button = %Button_EditProfile
@onready var button_close_profile: Button = %Button_CloseProfile
@onready var button_menu: Button = %Button_Menu
@onready var button_unmute_user: Button = %Button_UnmuteUser
@onready var button_unblock_user: Button = %Button_UnblockUser
@onready var menu: ColorRect = %Menu
@onready var profile_header: VBoxContainer = %ProfileHeader
@onready var control_own_and_landscape: Control = %Control_OwnAndLandscape
@onready var control_own_and_landscape_menu: VBoxContainer = %Control_OwnAndLandscapeMenu
@onready var margin_container_tag: MarginContainer = %MarginContainer_Tag


func _ready() -> void:
	call_deferred("_set_about_compact_mode")
	scroll_container.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	Global.player_identity.profile_changed.connect(self._on_global_profile_changed)
	button_menu.button_pressed = false
	menu.hide()

	if rounded:
		var current_style = get_theme_stylebox("panel")
		if current_style is StyleBoxFlat:
			var style_box = current_style.duplicate()
			style_box.corner_radius_top_left = 15
			style_box.corner_radius_top_right = 15
			style_box.corner_radius_bottom_right = 15
			style_box.corner_radius_bottom_left = 15
			add_theme_stylebox_override("panel", style_box)

	if closable:
		button_close_profile.show()
		control_own_and_landscape.hide()
		control_own_and_landscape_menu.hide()
	else:
		button_close_profile.hide()
		control_own_and_landscape.show()
		control_own_and_landscape_menu.show()

	_deploy_timeout_timer = Timer.new()
	_deploy_timeout_timer.wait_time = 15.0
	_deploy_timeout_timer.one_shot = true
	_deploy_timeout_timer.timeout.connect(self._async_on_deploy_timeout)
	add_child(_deploy_timeout_timer)

	_update_elements_visibility()
	add_to_group("blacklist_ui_sync")

	profile_equipped.emote_pressed.connect(_on_emote_pressed)
	profile_equipped.stop_emote.connect(_on_stop_emote)
	button_edit_profile.pressed.connect(_on_button_edit_profile_pressed)

	if not button_block_user.pressed.is_connected(_on_button_block_user_pressed):
		button_block_user.pressed.connect(_on_button_block_user_pressed)
	if not button_unblock_user.pressed.is_connected(_on_button_unblock_user_pressed):
		button_unblock_user.pressed.connect(_on_button_unblock_user_pressed)
	if not button_mute_user.pressed.is_connected(_on_button_mute_user_pressed):
		button_mute_user.pressed.connect(_on_button_mute_user_pressed)
	if not button_unmute_user.pressed.is_connected(_on_button_unmute_user_pressed):
		button_unmute_user.pressed.connect(_on_button_unmute_user_pressed)


func _update_elements_visibility() -> void:
	menu.hide()
	if is_own_passport:
		button_block_user.hide()
		button_mute_user.hide()
		button_menu.hide()
		button_edit_profile.show()
	else:
		button_block_user.show()
		button_mute_user.show()
		button_menu.show()
		button_edit_profile.hide()

	if current_profile != null:
		if current_profile.has_claimed_name():
			texture_rect_claimed_checkmark.show()
			label_tag.text = ""
			margin_container_tag.hide()
		else:
			texture_rect_claimed_checkmark.hide()
			margin_container_tag.show()
			label_tag.text = "#" + address.substr(address.length() - 4, 4)


func _show_placeholder() -> void:
	h_box_container_placeholder.show()
	h_box_container_data.hide()


func _show_data() -> void:
	h_box_container_placeholder.hide()
	h_box_container_data.show()
	_on_stop_emote()
	if not avatar_preview.avatar.emote_controller.is_playing():
		avatar_preview.avatar.emote_controller.async_play_emote("wave")
	_update_elements_visibility()
	_update_buttons()


func async_show_profile(profile: DclUserProfile) -> void:
	scroll_container.scroll_vertical = 0
	profile_about.hide()
	current_profile = profile

	if player_profile != null:
		is_own_passport = profile.get_ethereum_address() == player_profile.get_ethereum_address()
	else:
		is_own_passport = false

	# Show placeholder skeleton while data loads
	_show_placeholder()
	show()
	await avatar_preview.avatar.async_update_avatar_from_profile(current_profile)

	profile_about.refresh(current_profile)
	profile_links.refresh(current_profile)
	_refresh_name_and_address()
	await profile_equipped.async_refresh(current_profile)

	# All data ready, swap placeholder for real content
	_show_data()

	if is_own_passport:
		var mutable: DclUserProfile = Global.player_identity.get_mutable_profile()
		if mutable != null and profile.get_profile_version() < mutable.get_profile_version():
			if not _deploy_waiting:
				_deploy_waiting = true
				_show_placeholder()
				_deploy_timeout_timer.start()

	UiSounds.play_sound("mainmenu_widget_open")


func _on_emote_pressed(urn: String) -> void:
	avatar_preview.reset_avatar_rotation()
	avatar_preview.avatar.emote_controller.stop_emote()
	if not avatar_preview.avatar.emote_controller.is_playing():
		avatar_preview.avatar.emote_controller.async_play_emote(urn)


func _on_stop_emote() -> void:
	avatar_preview.avatar.emote_controller.stop_emote()


func _on_reset_avatars_rotation() -> void:
	avatar_preview.reset_avatar_rotation()


func close() -> void:
	hide()
	_show_placeholder()
	_on_stop_emote()
	_on_reset_avatars_rotation()
	if closable:
		close_profile.emit()


func _set_about_compact_mode() -> void:
	profile_about.is_portrait = false


func _refresh_name_and_address() -> void:
	address = current_profile.get_ethereum_address()
	label_address.text = Global.shorten_address(address)

	label_nickname.text = current_profile.get_name()
	var nickname_color = DclAvatar.get_nickname_color(current_profile.get_name())
	label_nickname.add_theme_color_override("font_color", nickname_color)


func _async_on_change_nick_popup_update_name_on_profile(nickname: String) -> void:
	label_nickname.text = nickname
	Global.player_identity.get_mutable_profile().set_name(nickname)
	await Global.player_identity.async_save_profile_metadata()


func _copy_name_and_tag() -> void:
	DisplayServer.clipboard_set(label_nickname.text + label_tag.text)
	NotificationsManager.show_system_toast(tr("TOAST_COPIED"), tr("TOAST_NAME_COPIED"))


func _copy_address() -> void:
	DisplayServer.clipboard_set(address)
	NotificationsManager.show_system_toast(tr("TOAST_COPIED"), tr("TOAST_ADDRESS_COPIED"))


func _on_label_nickname_gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		if event.pressed:
			_copy_name_and_tag()


func _on_label_tag_gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		if event.pressed:
			_copy_name_and_tag()


func _on_label_address_gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		if event.pressed:
			_copy_address()


func _on_global_profile_changed(new_profile: DclUserProfile) -> void:
	if new_profile == null:
		return
	var new_addr = new_profile.get_ethereum_address()
	if not is_own_passport and new_addr != address:
		return
	current_profile = new_profile
	profile_links.refresh(current_profile)
	profile_about.refresh(current_profile)
	_refresh_name_and_address()
	if _deploy_waiting:
		_deploy_waiting = false
		_show_data()
	if _deploy_timeout_timer != null and _deploy_timeout_timer.is_stopped() == false:
		_deploy_timeout_timer.stop()


func _async_on_deploy_timeout() -> void:
	if not _deploy_waiting:
		return
	var addr = Global.player_identity.get_address_str()
	var lambda_url = Global.realm.get_lambda_server_base_url()
	await Global.player_identity.async_fetch_profile(addr, lambda_url)
	if _deploy_waiting:
		_deploy_waiting = false
		_show_data()


func _on_button_mute_user_toggled(toggled_on: bool) -> void:
	if toggled_on:
		Global.social_blacklist.add_muted(avatar_preview.avatar.avatar_id)
	else:
		Global.social_blacklist.remove_muted(avatar_preview.avatar.avatar_id)
	_update_buttons()

	_notify_other_components_of_change()


func _check_block_and_mute_status() -> void:
	var current_avatar = avatar_preview.avatar
	is_blocked_user = Global.social_blacklist.is_blocked(current_avatar.avatar_id)
	is_muted_user = Global.social_blacklist.is_muted(current_avatar.avatar_id)

	if is_blocked_user:
		button_block_user.hide()
		button_mute_user.hide()
	elif is_muted_user:
		button_block_user.show()
		button_mute_user.button_pressed = true


func _update_buttons() -> void:
	if is_own_passport:
		return
	var current_avatar = avatar_preview.avatar
	is_blocked_user = Global.social_blacklist.is_blocked(current_avatar.avatar_id)
	is_muted_user = Global.social_blacklist.is_muted(current_avatar.avatar_id)
	if is_blocked_user:
		button_block_user.hide()
		button_unblock_user.show()
		button_mute_user.hide()
		button_unmute_user.hide()
	else:
		button_block_user.show()
		button_unblock_user.hide()
		if is_muted_user:
			button_mute_user.hide()
			button_unmute_user.show()
		else:
			button_mute_user.show()
			button_unmute_user.hide()


func _block_user(user_address: String) -> void:
	button_block_user.disabled = false
	Global.metrics.track_block_user(user_address, false)
	Global.social_blacklist.add_blocked(user_address)
	_update_buttons()
	_notify_other_components_of_change()


func _unblock_user(user_address: String) -> void:
	button_unblock_user.disabled = false
	Global.social_blacklist.remove_blocked(user_address)
	_update_buttons()
	_notify_other_components_of_change()


func _notify_other_components_of_change() -> void:
	if avatar_preview.avatar != null:
		Global.get_tree().call_group(
			"blacklist_ui_sync", "_sync_blacklist_ui", avatar_preview.avatar.avatar_id
		)


func _sync_blacklist_ui(changed_avatar_id: String) -> void:
	if (
		not is_own_passport
		and current_profile != null
		and avatar_preview.avatar != null
		and avatar_preview.avatar.avatar_id == changed_avatar_id
	):
		call_deferred("_update_buttons")


func _on_button_close_profile_button_up() -> void:
	close()


func _on_visibility_changed() -> void:
	if visible:
		grab_focus()


func _on_button_menu_toggled(toggled_on: bool) -> void:
	if toggled_on:
		menu.show()
	else:
		menu.hide()


func _on_button_edit_profile_pressed() -> void:
	NamesRequest.pre_fetch()
	close()
	Global.set_orientation_portrait()
	Global.open_profile_editor.emit()


func _on_copy_nick_pressed() -> void:
	_copy_name_and_tag()


func _on_copy_address_pressed() -> void:
	_copy_address()


func _on_button_block_user_pressed() -> void:
	var current_avatar = avatar_preview.avatar
	button_block_user.disabled = true
	Global.metrics.track_click_button("user_block", "PROFILE", "")
	_block_user(current_avatar.avatar_id)


func _on_button_unblock_user_pressed() -> void:
	var current_avatar = avatar_preview.avatar
	button_unblock_user.disabled = true
	Global.metrics.track_click_button("user_unblock", "PROFILE", "")
	_unblock_user(current_avatar.avatar_id)


func _on_button_mute_user_pressed() -> void:
	Global.social_blacklist.add_muted(avatar_preview.avatar.avatar_id)
	call_deferred("_update_buttons")
	_notify_other_components_of_change()


func _on_button_unmute_user_pressed() -> void:
	Global.social_blacklist.remove_muted(avatar_preview.avatar.avatar_id)
	call_deferred("_update_buttons")
	_notify_other_components_of_change()
