class_name Lobby
extends Control

signal change_scene(new_scene_path: String)

var current_profile: DclUserProfile
var guest_account_created: bool = false

var waiting_for_new_wallet: bool = false

var loading_first_profile: bool = false

var _skip_lobby: bool = false
var _last_panel: Control = null

@onready var control_main = %Main

@onready var control_loading = %Loading
@onready var control_signin = %SignIn
@onready var control_start = %Start
@onready var control_backpack = %BackpackContainer
@onready var control_restore_and_choose_name: Control = %RestoreAndChooseName

@onready var container_sign_in_step1 = %VBoxContainer_SignInStep1
@onready var container_sign_in_step2 = %VBoxContainer_SignInStep2

@onready var label_avatar_name = %Label_Name

@onready var avatar_preview = %AvatarPreview

@onready var lineedit_choose_name = %LineEdit_ChooseName

@onready var restore_panel: VBoxContainer = %VBoxContainer_RestorePanel
@onready var choose_name: VBoxContainer = %VBoxContainer_ChooseName

@onready var checkbox_terms_and_privacy = %CheckBox_TermsAndPrivacy
@onready var button_next = %Button_Next

@onready var backpack = %Backpack

@onready var button_open_browser = %Button_OpenBrowser

@onready var background_1: Control = %Background1
@onready var background_2: Control = %Background2


func show_panel(child_node: Control, subpanel: Control = null):
	for child in control_main.get_children():
		child.hide()

	child_node.show()

	match child_node:
		control_loading, control_backpack:
			_show_background1()
		control_start:
			_show_background2()

	if _last_panel != null:
		_last_panel.hide()
		_last_panel = null

	if subpanel != null:
		subpanel.show()
		_last_panel = subpanel


func async_close_sign_in(generate_snapshots: bool = true):
	if generate_snapshots:
		var avatar := current_profile.get_avatar()
		await backpack.async_prepare_snapshots(avatar, current_profile)

	if Global.is_xr():
		change_scene.emit("res://src/ui/components/menu/menu.tscn")
	else:
		get_tree().change_scene_to_file("res://src/ui/components/menu/menu.tscn")


# gdlint:ignore = async-function-name
func _ready():
	Global.music_player.play("music_builder")
	restore_panel.hide()
	choose_name.hide()

	var android_login = %AndroidLogin
	if is_instance_valid(android_login) and android_login.is_platform_supported():
		android_login.set_lobby(self)
		android_login.show()
		button_open_browser.hide()
	else:
		button_open_browser.text = "OPEN BROWSER"
		android_login.hide()

	show_panel(control_loading)

	UiSounds.install_audio_recusirve(self)
	Global.dcl_tokio_rpc.need_open_url.connect(self._on_need_open_url)
	Global.player_identity.profile_changed.connect(self._async_on_profile_changed)
	Global.player_identity.wallet_connected.connect(self._on_wallet_connected)

	Global.scene_runner.set_pause(true)

	var args = OS.get_cmdline_args()
	if args.has("--skip-lobby"):
		_skip_lobby = true

	var session_account: Dictionary = Global.get_config().session_account

	if Global.player_identity.try_recover_account(session_account):
		loading_first_profile = true
		show_panel(control_loading)

	elif _skip_lobby:
		show_panel(control_loading)
		create_guest_account_if_needed()
		go_to_explorer.call_deferred()
	else:
		show_panel(control_start)


func go_to_explorer():
	if is_inside_tree():
		get_tree().change_scene_to_file("res://src/ui/explorer.tscn")


func _async_on_profile_changed(new_profile: DclUserProfile):
	current_profile = new_profile
	await avatar_preview.avatar.async_update_avatar_from_profile(new_profile)

	if !new_profile.has_connected_web3():
		Global.get_config().guest_profile = new_profile.to_godot_dictionary()
		Global.get_config().save_to_settings_file()

	if loading_first_profile:
		loading_first_profile = false
		if profile_has_name():
			label_avatar_name.set_text("Welcome back " + new_profile.get_name())

			restore_panel.show()
			show_panel(control_restore_and_choose_name, restore_panel)
			_show_avatar_preview()
			if _skip_lobby:
				go_to_explorer.call_deferred()
		else:
			show_panel(control_start)

	if _skip_lobby:
		go_to_explorer()

	if waiting_for_new_wallet:
		waiting_for_new_wallet = false
		if profile_has_name():
			await async_close_sign_in()
		else:
			show_panel(control_restore_and_choose_name, choose_name)
			_show_avatar_preview()


func show_connect():
	show_panel(control_signin)


func _on_need_open_url(url: String, _description: String, use_webview: bool) -> void:
	Global.open_url(url, use_webview)


func _on_wallet_connected(_address: String, _chain_id: int, _is_guest: bool) -> void:
	Global.get_config().session_account = {}

	var new_stored_account := {}
	if Global.player_identity.get_recover_account_to(new_stored_account):
		Global.get_config().session_account = new_stored_account

	Global.get_config().save_to_settings_file()


func _on_button_different_account_pressed():
	Global.get_config().session_account = {}
	Global.get_config().save_to_settings_file()
	show_connect()
	avatar_preview.hide()


func _on_button_continue_pressed():
	_async_on_profile_changed(backpack.mutable_profile)
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
	if lineedit_choose_name.text.is_empty() or checkbox_terms_and_privacy.button_pressed == false:
		return

	avatar_preview.hide()
	show_panel(control_loading)
	current_profile.set_name(lineedit_choose_name.text)
	current_profile.set_has_connected_web3(!Global.player_identity.is_guest)
	var avatar := current_profile.get_avatar()

	await backpack.async_prepare_snapshots(avatar, current_profile)

	current_profile.set_avatar(avatar)

	await Global.player_identity.async_deploy_profile(current_profile, true)

	await async_close_sign_in(false)


func _on_button_random_name_pressed():
	lineedit_choose_name.set_text(RandomGeneratorUtil.generate_unique_name())
	_check_button_finish()


func _on_button_open_browser_pressed():
	Global.player_identity.try_connect_account("")
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
		Global.get_config().guest_profile = {}
		Global.get_config().save_to_settings_file()
		Global.player_identity.create_guest_account()
		Global.player_identity.set_default_profile()
		guest_account_created = true


func profile_has_name():
	var profile = Global.player_identity.get_profile_or_null()
	return profile != null and not profile.get_name().is_empty()


func _on_button_enter_as_guest_pressed():
	create_guest_account_if_needed()

	show_panel(control_restore_and_choose_name, choose_name)
	_show_avatar_preview()


func _show_avatar_preview():
	avatar_preview.show()
	avatar_preview.avatar.emote_controller.play_emote("raiseHand")


# gdlint:ignore = async-function-name
func _on_button_jump_in_pressed():
	await async_close_sign_in()


func toggle_terms_and_privacy_checkbox():
	checkbox_terms_and_privacy.set_pressed(not checkbox_terms_and_privacy.button_pressed)


func _on_rich_text_label_gui_input(event):
	if event is InputEventScreenTouch:
		if !event.pressed:
			toggle_terms_and_privacy_checkbox()


func _on_rich_text_label_meta_clicked(meta):
	Global.open_url(meta)
	# we're going to toggle in the rich text box gui input
	# so here we toggle it again compensate, to let as it is
	# not the best solution.
	toggle_terms_and_privacy_checkbox()


func _on_check_box_terms_and_privacy_toggled(_toggled_on):
	_check_button_finish()


func _on_line_edit_choose_name_text_changed(_new_text):
	_check_button_finish()


func _check_button_finish():
	var disabled = (
		lineedit_choose_name.text.is_empty() or not checkbox_terms_and_privacy.button_pressed
	)
	if button_next.disabled != disabled:
		avatar_preview.avatar.emote_controller.play_emote("shrug" if disabled else "clap")
	button_next.disabled = disabled


func _show_background1():
	background_1.show()
	background_2.hide()


func _show_background2():
	background_1.hide()
	background_2.show()
