class_name Lobby
extends Control

signal change_scene(new_scene_path: String)

# Debug flag for social service testing
const DEBUG_SOCIAL_SERVICE: bool = true

var is_creating_account: bool = false

var current_profile: DclUserProfile
var guest_account_created: bool = false

var waiting_for_new_wallet: bool = false

var loading_first_profile: bool = false
var current_screen_name: String = ""

var _skip_lobby: bool = false
var _last_panel: Control = null
var _playing: String

@onready var control_main = %Main

@onready var control_loading = %Loading
@onready var control_signin = %SignIn
@onready var control_start = %Start
@onready var control_backpack = %BackpackContainer
@onready var control_restore_and_choose_name: Control = %RestoreAndChooseName
@onready var label_length: Label = %Label_Length
@onready var label_error: RichTextLabel = %Label_Error
@onready var label_advise: Label = %Label_Advise

@onready var panel_container_error_border: PanelContainer = %PanelContainer_ErrorBorder

@onready var line_edit_choose_name: LineEdit = %LineEdit_ChooseName

@onready var container_sign_in_step1 = %VBoxContainer_SignInStep1
@onready var container_sign_in_step2 = %VBoxContainer_SignInStep2

@onready var label_avatar_name = %Label_Name

@onready var avatar_preview: AvatarPreview = %AvatarPreview
@onready var button_next = %Button_Next

@onready var backpack = %Backpack

@onready var choose_name_head: VBoxContainer = %ChooseNameHead
@onready var restore_name_head: VBoxContainer = %RestoreNameHead
@onready var choose_name_footer: VBoxContainer = %ChooseNameFooter
@onready var restore_name_footer: VBoxContainer = %RestoreNameFooter
@onready var label_name: Label = %Label_Name

@onready var button_enter_as_guest: Button = %Button_EnterAsGuest
@onready var sign_in_title: Label = %SignInTitle

@onready var label_version = %Label_Version


func show_panel(child_node: Control, subpanel: Control = null):
	for child in control_main.get_children():
		child.hide()

	child_node.show()

	if _last_panel != null:
		_last_panel.hide()
		_last_panel = null

	if subpanel != null:
		subpanel.show()
		_last_panel = subpanel


func track_lobby_screen(screen_name: String):
	current_screen_name = screen_name
	Global.metrics.track_screen_viewed(screen_name, "")
	Global.metrics.flush.call_deferred()


func show_restore_screen():
	track_lobby_screen("COMEBACK")
	restore_name_head.show()
	restore_name_footer.show()
	label_name.show()
	choose_name_head.hide()
	choose_name_footer.hide()
	show_panel(control_restore_and_choose_name)


func show_avatar_naming_screen():
	track_lobby_screen("AVATAR_NAMING")
	restore_name_head.hide()
	restore_name_footer.hide()
	label_name.hide()
	choose_name_head.show()
	choose_name_footer.show()
	show_panel(control_restore_and_choose_name)
	_check_button_finish()


func show_loading_screen():
	current_screen_name = "LOBBY_LOADING"
	show_panel(control_loading)


func show_account_home_screen():
	track_lobby_screen("ACCOUNT_HOME")
	show_panel(control_start)


func get_auth_home_screen_name():
	if Global.is_ios():
		return "AUTH_HOME_IOS"
	if Global.is_android():
		return "AUTH_HOME_ANDROID"

	return "AUTH_HOME_DESKTOP"


func show_auth_home_screen():
	track_lobby_screen(get_auth_home_screen_name())
	container_sign_in_step1.show()
	container_sign_in_step2.hide()
	show_panel(control_signin)


func show_auth_browser_open_screen():
	track_lobby_screen("AUTH_BROWSER_OPEN")
	container_sign_in_step1.hide()
	container_sign_in_step2.show()
	show_panel(control_signin)


func show_avatar_create_screen():
	track_lobby_screen("AVATAR_CREATE")
	show_panel(control_backpack)


func async_close_sign_in(generate_snapshots: bool = true):
	if generate_snapshots:
		var avatar := current_profile.get_avatar()
		await backpack.async_prepare_snapshots(avatar, current_profile)

	Global.metrics.update_identity(
		Global.player_identity.get_address_str(), Global.player_identity.is_guest
	)

	if Global.deep_link_obj.is_location_defined() or not Global.deep_link_obj.realm.is_empty():
		go_to_explorer()
		return

	if Global.is_xr():
		change_scene.emit("res://src/ui/components/menu/menu.tscn")
	else:
		get_tree().change_scene_to_file("res://src/ui/components/menu/menu.tscn")


# gdlint:ignore = async-function-name
func _ready():
	# Set version label
	label_version.set_text("v" + Global.get_version())

	Global.music_player.play("music_builder")
	control_restore_and_choose_name.hide()

	var login = %Login

	login.set_lobby(self)
	login.show()

	show_loading_screen()

	UiSounds.install_audio_recusirve(self)
	Global.dcl_tokio_rpc.need_open_url.connect(self._on_need_open_url)
	Global.player_identity.profile_changed.connect(self._async_on_profile_changed)
	Global.player_identity.wallet_connected.connect(self._on_wallet_connected)

	Global.scene_runner.set_pause(true)

	if Global.cli.skip_lobby:
		_skip_lobby = true

	var session_account: Dictionary = Global.get_config().session_account

	if Global.cli.guest_profile:
		session_account.clear()
		Global.get_config().save_to_settings_file()
		Global.player_identity.create_guest_account()
		Global.player_identity.set_random_profile()
		var random_profile = Global.player_identity.get_profile_or_null()
		if random_profile != null:
			Global.get_config().guest_profile = random_profile.to_godot_dictionary()

	if Global.player_identity.try_recover_account(session_account):
		loading_first_profile = true
		show_loading_screen()
	elif _skip_lobby:
		show_loading_screen()
		go_to_explorer.call_deferred()
	else:
		show_account_home_screen()


func go_to_explorer():
	if is_inside_tree():
		get_tree().change_scene_to_file("res://src/ui/explorer.tscn")


func _async_on_profile_changed(new_profile: DclUserProfile):
	current_profile = new_profile
	await avatar_preview.avatar.async_update_avatar_from_profile(new_profile)

	if !new_profile.has_connected_web3():
		Global.get_config().guest_profile = new_profile.to_godot_dictionary()
		Global.get_config().save_to_settings_file()
		restore_name_head.hide()
		restore_name_footer.hide()
		label_name.hide()
		choose_name_head.show()
		choose_name_footer.show()

	if loading_first_profile:
		loading_first_profile = false
		if profile_has_name():
			label_avatar_name.set_text(new_profile.get_name())
			show_restore_screen()
			_show_avatar_preview()
			Global.metrics.update_identity(
				Global.player_identity.get_address_str(), Global.player_identity.is_guest
			)
			if _skip_lobby:
				go_to_explorer.call_deferred()
		else:
			show_account_home_screen()

	if _skip_lobby:
		go_to_explorer()

	if waiting_for_new_wallet:
		waiting_for_new_wallet = false
		await async_close_sign_in()
	else:
		if Global.deep_link_obj.is_location_defined() or not Global.deep_link_obj.realm.is_empty():
			go_to_explorer()
			return


func _on_need_open_url(url: String, _description: String, use_webview: bool) -> void:
	Global.open_url(url, use_webview)


func _on_wallet_connected(_address: String, _chain_id: int, _is_guest: bool) -> void:
	Global.get_config().session_account = {}

	var new_stored_account := {}
	if Global.player_identity.get_recover_account_to(new_stored_account):
		Global.get_config().session_account = new_stored_account

	Global.get_config().save_to_settings_file()

	# Initialize social service for non-guest accounts only
	if not _is_guest:
		_async_initialize_social_service()


func _on_button_different_account_pressed():
	Global.metrics.update_identity("unauthenticated", false)
	Global.metrics.track_click_button("use_another_account", current_screen_name, "")
	Global.get_config().session_account = {}

	# Clear the current social blacklist when switching accounts
	Global.social_blacklist.clear_blocked()
	Global.social_blacklist.clear_muted()

	Global.get_config().save_to_settings_file()
	show_account_home_screen()


func _on_button_continue_pressed():
	Global.metrics.track_click_button("next", current_screen_name, "")
	_async_on_profile_changed(backpack.mutable_profile)
	show_avatar_naming_screen()


func _on_button_start_pressed():
	Global.metrics.track_click_button("create_account", current_screen_name, "")
	button_enter_as_guest.show()
	sign_in_title.text = "Create Your Account"
	create_guest_account_if_needed()
	is_creating_account = true
	show_avatar_create_screen()


# gdlint:ignore = async-function-name
func _on_button_next_pressed():
	Global.metrics.track_click_button("next", current_screen_name, "")
	if line_edit_choose_name.text.is_empty():
		return

	avatar_preview.hide()
	show_loading_screen()
	current_profile.set_name(line_edit_choose_name.text)
	current_profile.set_has_connected_web3(!Global.player_identity.is_guest)
	var avatar := current_profile.get_avatar()

	await backpack.async_prepare_snapshots(avatar, current_profile)

	current_profile.set_avatar(avatar)

	await ProfileService.async_deploy_profile(current_profile, true)

	show_auth_home_screen()


func _on_button_random_name_pressed():
	line_edit_choose_name.set_text_value(RandomGeneratorUtil.generate_unique_name())
	_check_button_finish()


func _on_button_go_to_sign_in_pressed():
	Global.metrics.track_click_button("sign_in", current_screen_name, "")
	button_enter_as_guest.hide()
	sign_in_title.text = "Sign In to Decentraland"
	show_auth_home_screen()


func _on_button_cancel_pressed():
	Global.metrics.track_click_button("cancel", current_screen_name, "")
	Global.player_identity.abort_try_connect_account()
	show_auth_home_screen()


func create_guest_account_if_needed():
	if not guest_account_created:
		Global.get_config().guest_profile = {}
		Global.get_config().save_to_settings_file()
		Global.player_identity.create_guest_account()
		if is_creating_account:
			Global.player_identity.set_profile(current_profile)
		else:
			Global.player_identity.set_default_profile()
		guest_account_created = true


func profile_has_name():
	var profile = Global.player_identity.get_profile_or_null()
	return profile != null and not profile.get_name().is_empty()


# gdlint:ignore = async-function-name
func _on_button_enter_as_guest_pressed():
	Global.metrics.track_click_button("enter_as_guest", current_screen_name, "")
	create_guest_account_if_needed()
	await async_close_sign_in()


func _show_avatar_preview():
	avatar_preview.show()
	avatar_preview.avatar.emote_controller.play_emote("raiseHand")


# gdlint:ignore = async-function-name
func _on_button_jump_in_pressed():
	Global.metrics.track_click_button("lets_go", current_screen_name, "")
	await async_close_sign_in()


func _on_check_box_terms_and_privacy_toggled(_toggled_on):
	_check_button_finish()


func _on_line_edit_choose_name_text_changed(_new_text):
	_check_button_finish()


func _check_button_finish():
	var color: Color = Color.WHITE
	label_length.text = (
		str(line_edit_choose_name.text.length()) + "/" + str(line_edit_choose_name.character_limit)
	)
	if line_edit_choose_name.text.length() > line_edit_choose_name.character_limit:
		color = Color.RED
	else:
		color = Color.WHITE
	label_length.label_settings.font_color = color

	if line_edit_choose_name.error:
		label_error.show()
		label_advise.hide()
		label_error.text = line_edit_choose_name.error_message
		button_next.disabled = true
		if not avatar_preview.avatar.emote_controller.is_playing() or _playing != "shrug":
			avatar_preview.avatar.emote_controller.play_emote("shrug")
			_playing = "shrug"
		panel_container_error_border.self_modulate = Color.RED
	else:
		label_error.hide()
		label_advise.show()
		button_next.disabled = line_edit_choose_name.text.is_empty()
		if (
			!button_next.disabled and not avatar_preview.avatar.emote_controller.is_playing()
			or _playing != "clap"
		):
			avatar_preview.avatar.emote_controller.play_emote("clap")
			_playing = "clap"
		panel_container_error_border.self_modulate = Color.TRANSPARENT


func _on_avatar_preview_gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		if event.pressed:
			if not avatar_preview.avatar.emote_controller.is_playing():
				if line_edit_choose_name.text.contains("dancer"):
					avatar_preview.avatar.emote_controller.play_emote("dance")
				else:
					avatar_preview.avatar.emote_controller.play_emote("wave")


func _on_line_edit_choose_name_dcl_line_edit_changed() -> void:
	_check_button_finish()


func _async_initialize_social_service() -> void:
	print("[Lobby] Initializing Social Service for authenticated user")
	Global.social_service.initialize_from_player_identity(Global.player_identity)

	var promise = Global.social_service.subscribe_to_updates()
	await PromiseUtils.async_awaiter(promise)

	if promise.is_rejected():
		var error = promise.get_data()
		push_error("[Lobby] Failed to initialize Social Service: " + str(error.get_error()))
		return

	print("[Lobby] Social Service initialized successfully")

	# Run debug tests if enabled
	if DEBUG_SOCIAL_SERVICE:
		var SocialServiceDebug = preload("res://src/ui/components/auth/social_service_debug.gd")
		await SocialServiceDebug.async_debug_social_service()
