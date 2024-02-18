extends Control

var current_profile: DclUserProfile
var guest_account_created: bool = false

var waiting_for_new_wallet: bool = false

var loading_first_profile: bool = false

@onready var control_main = %Main

@onready var control_restore = %Restore
@onready var control_signin = %SignIn
@onready var control_start = %Start
@onready var control_backpack = %Backpack
@onready var control_choose_name = %ChooseName

@onready var container_sign_in_step1 = %VBoxContainer_SignInStep1
@onready var container_sign_in_step2 = %VBoxContainer_SignInStep2

@onready var label_name = %Label_Name

@onready var avatar_preview = %AvatarPreview

@onready var lineedit_choose_name = %LineEdit_ChooseName

@onready var restore_panel = %VBoxContainer_RestorePanel
@onready var restore_loading = %TextureProgressBar_RestoreLoading

# TODO: Change screen orientation for Mobile
#func set_portrait():
##DisplayServer.screen_set_orientation(DisplayServer.SCREEN_PORTRAIT)
#DisplayServer.window_set_size(Vector2i(720, 1280))
##get_tree().root.get_viewport().set_size(Vector2i(720, 1280))
##ProjectSettings.set_setting("display/window/size/viewport_width", 720)
##ProjectSettings.set_setting("display/window/size/viewport_height", 1280)
#
#
#func set_landscape():
##DisplayServer.screen_set_orientation(DisplayServer.SCREEN_LANDSCAPE)
#DisplayServer.window_set_size(Vector2i(1280, 720))
##get_tree().root.get_viewport().set_size(Vector2i(1280, 720))


func show_panel(child_node: Control):
	for child in control_main.get_children():
		child.hide()

	child_node.show()


func close_sign_in():
	get_tree().change_scene_to_file("res://src/ui/components/discover/discover.tscn")


func _ready():
	Global.player_identity.need_open_url.connect(self._on_need_open_url)
	Global.player_identity.profile_changed.connect(self._async_on_profile_changed)
	Global.player_identity.wallet_connected.connect(self._on_wallet_connected)

	Global.scene_runner.set_pause(true)
	if Global.player_identity.try_recover_account(Global.config.session_account):
		loading_first_profile = true
		restore_panel.hide()
		restore_loading.hide()
		show_panel(control_restore)
		_async_show_avatar_preview()
	else:
		show_panel(control_start)


func _async_on_profile_changed(new_profile: DclUserProfile):
	current_profile = new_profile

	if loading_first_profile:
		loading_first_profile = false
		label_name.text = "Welcome back " + new_profile.get_name()
		label_name.show()

		restore_loading.hide()
		restore_panel.show()

	await avatar_preview.avatar.async_update_avatar_from_profile(new_profile)

	if waiting_for_new_wallet:
		waiting_for_new_wallet = false
		if profile_has_name():
			close_sign_in()
		else:
			show_panel(control_choose_name)
			_async_show_avatar_preview()


func show_connect():
	show_panel(control_signin)


func _on_need_open_url(url: String, _description: String) -> void:
	Global.open_url(url)


func _on_wallet_connected(_address: String, _chain_id: int, _is_guest: bool) -> void:
	Global.config.session_account = {}

	var new_stored_account := {}
	if Global.player_identity.get_recover_account_to(new_stored_account):
		Global.config.session_account = new_stored_account

	Global.config.save_to_settings_file()


func _on_button_different_account_pressed():
	Global.config.session_account = {}
	Global.config.save_to_settings_file()
	show_connect()
	avatar_preview.hide()


func _on_button_continue_pressed():
	show_connect()


func _on_avatar_preview_gui_input(event):
	if event is InputEventScreenTouch:
		if event.pressed:
			if not avatar_preview.avatar.emote_controller.is_playing():
				avatar_preview.avatar.emote_controller.play_emote("wave")


func _on_button_start_pressed():
	create_guest_account_if_needed()

	show_panel(control_backpack)


# gdlint:ignore = async-function-name
func _on_button_next_pressed():
	if lineedit_choose_name.text.is_empty():
		return  # TODO: Add error message

	current_profile.set_name(lineedit_choose_name.text)
	current_profile.set_has_connected_web3(!Global.player_identity.is_guest)
	var avatar := current_profile.get_avatar()
	avatar.set_name(current_profile.get_name())
	current_profile.set_avatar(avatar)

	# Forget, it's going to be lock until a realm is set
	Global.player_identity.async_deploy_profile(current_profile, true)

	close_sign_in()


func _on_button_random_name_pressed():
	lineedit_choose_name.text = RandomGeneratorUtil.generate_unique_name()


func _on_button_open_browser_pressed():
	Global.player_identity.try_connect_account()
	container_sign_in_step1.hide()
	container_sign_in_step2.show()
	waiting_for_new_wallet = true


func _on_button_go_to_sign_in_pressed():
	show_connect()


func _on_button_cancel_pressed():
	Global.player_identity.abort_try_connect_account()
	show_panel(control_signin)
	container_sign_in_step1.show()
	container_sign_in_step2.hide()


func create_guest_account_if_needed():
	if not guest_account_created:
		Global.config.guest_profile = {}
		Global.config.save_to_settings_file()
		Global.player_identity.set_default_profile()
		Global.player_identity.create_guest_account()
		guest_account_created = true


func profile_has_name():
	var profile = Global.player_identity.get_profile_or_null()
	return profile != null and not profile.get_name().is_empty()


func _on_button_enter_as_guest_pressed():
	create_guest_account_if_needed()

	show_panel(control_choose_name)
	_async_show_avatar_preview()


func _async_show_avatar_preview():
	await get_tree().create_timer(1.0).timeout
	avatar_preview.show()
	avatar_preview.avatar.emote_controller.play_emote("raiseHand")


func _on_button_jump_in_pressed():
	close_sign_in()
