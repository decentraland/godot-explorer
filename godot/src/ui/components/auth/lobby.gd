class_name Lobby
extends Control
## Lobby entry flow controller.
##
## Panel mapping (Figma screen -> Godot node):
##   EULA               -> %Eula                 (checkbox + accept)
##   VERSION_UPGRADE     -> %VersionUpgrade       (update / not now)
##   ACCOUNT_HOME        -> %Start                (create account / sign in)
##   AUTH_HOME           -> %SignIn step1          (auth provider buttons)
##   AUTH_BROWSER_OPEN   -> %SignIn step2          (spinner + cancel)
##   AVATAR_CREATE       -> %BackpackContainer     (backpack avatar editor)
##   AVATAR_NAMING       -> %RestoreAndChooseName  (choose-name mode)
##   COMEBACK            -> %RestoreAndChooseName  (restore mode: welcome back)
##   LOBBY_LOADING       -> %Loading               (spinner)
##   FTUE                -> $Main/FTUE              (first time user experience)
##
## Auth flow (Create Account / Sign In only changes the label):
##   EULA -> ACCOUNT_HOME -> AUTH_HOME -> (auth)
##     - profile exists  -> COMEBACK (Welcome Back) -> Explorer
##     - no profile      -> AVATAR_CREATE -> AVATAR_NAMING -> FTUE -> Explorer
##
## Returning user (has session):
##   EULA check -> Explorer (direct, no Welcome Back)

signal change_scene(new_scene_path: String)

const AUTH_TIMEOUT_SECONDS: float = 10.0
const FTUE_PLACE_ID: String = "780f04dd-eba1-41a8-b109-74896c87e98b"

var is_creating_account: bool = false
var auth_timeout_timer: Timer = null
var auth_waiting_for_browser: bool = false

var current_profile: DclUserProfile
var guest_account_created: bool = false

var waiting_for_new_wallet: bool = false
var ready_for_redirect_by_deep_link: bool = false

var loading_first_profile: bool = false
var current_screen_name: String = ""

var _skip_lobby: bool = false
var _skip_lobby_to_menu: bool = false
var _last_panel: Control = null
var _playing: String

@onready var control_main = %Main

@onready var dcl_line_edit: VBoxContainer = %DclLineEdit

@onready var control_loading = %Loading
@onready var control_eula = %Eula
@onready var control_version_upgrade = %VersionUpgrade
@onready var control_signin = %SignIn
@onready var control_auth_error = %AuthError
@onready var label_error_message: Label = %Label_ErrorMessage
@onready var control_start = %Start
@onready var control_backpack = %BackpackContainer
@onready var control_restore_and_choose_name: Control = %RestoreAndChooseName

@onready var container_sign_in_step1 = %VBoxContainer_SignInStep1
@onready var container_sign_in_step2 = %VBoxContainer_SignInStep2
@onready var sign_in_logo = %SignInLogo
@onready var sign_in_logo_sep = %SignInLogoSep
@onready var auth_spinner = %AuthSpinner
@onready var auth_error_label = %AuthErrorLabel
@onready var button_cancel = %Button_Cancel
@onready var button_cancel_icon = %ButtonCancelIcon
@onready var label_step2_title: Label = %VBoxContainer_SignInStep2/Label_Title

@onready var label_avatar_name = %Label_Name

@onready var avatar_preview: AvatarPreview = %AvatarPreview
@onready var button_next = %Button_Next

@onready var backpack = %Backpack

@onready var choose_name_head: VBoxContainer = %ChooseNameHead
@onready var choose_name_footer: VBoxContainer = %ChooseNameFooter
@onready var restore_name_footer: VBoxContainer = %RestoreNameFooter
@onready var label_name: Label = %Label_Name

@onready var button_enter_as_guest: Button = %Button_EnterAsGuest
@onready var button_back: Button = %Button_Back
@onready var sign_in_title: Label = %SignInTitle

@onready var checkbox_eula: CheckBox = %CheckBox_Eula
@onready var button_accept_eula: Button = %Button_AcceptEula

@onready var label_version = %Label_Version

@onready var ftue_screen: PlaceItem = $Main/FTUE


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
	button_back.hide()
	restore_name_footer.show()
	label_name.show()
	choose_name_head.hide()
	choose_name_footer.hide()
	show_panel(control_restore_and_choose_name)


func show_avatar_naming_screen():
	track_lobby_screen("AVATAR_NAMING")
	button_back.hide()
	restore_name_footer.hide()
	label_name.hide()
	choose_name_head.show()
	choose_name_footer.show()
	show_panel(control_restore_and_choose_name)


func show_loading_screen():
	current_screen_name = "LOBBY_LOADING"
	button_back.hide()
	show_panel(control_loading)


func show_eula_screen():
	track_lobby_screen("EULA")
	button_back.hide()
	show_panel(control_eula)


func show_version_upgrade_screen():
	track_lobby_screen("VERSION_UPGRADE")
	button_back.hide()
	show_panel(control_version_upgrade)


func show_account_home_screen():
	track_lobby_screen("ACCOUNT_HOME")
	button_back.hide()
	_request_notification_permission_if_needed()
	show_panel(control_start)


func _request_notification_permission_if_needed():
	if not Global.is_mobile() or Global.is_virtual_mobile():
		return
	if NotificationsManager.has_local_notification_permission():
		return
	NotificationsManager.request_local_notification_permission(current_screen_name)
	# Listen for the result (especially for iOS async flow)
	if not NotificationsManager.local_notification_permission_changed.is_connected(
		_on_notification_permission_result
	):
		NotificationsManager.local_notification_permission_changed.connect(
			_on_notification_permission_result
		)


func _on_notification_permission_result(_granted: bool):
	# Disconnect after first result
	if NotificationsManager.local_notification_permission_changed.is_connected(
		_on_notification_permission_result
	):
		NotificationsManager.local_notification_permission_changed.disconnect(
			_on_notification_permission_result
		)


func get_auth_home_screen_name():
	if Global.is_ios():
		return "AUTH_HOME_IOS"
	if Global.is_android():
		return "AUTH_HOME_ANDROID"

	return "AUTH_HOME_DESKTOP"


func show_auth_home_screen():
	track_lobby_screen(get_auth_home_screen_name())
	sign_in_logo.show()
	sign_in_logo_sep.show()
	container_sign_in_step1.show()
	container_sign_in_step2.hide()
	button_back.show()
	show_panel(control_signin)


func show_auth_browser_open_screen(
	message: String = "Opening Browser...", auth_method: String = ""
):
	current_screen_name = "AUTH_BROWSER_OPEN"
	var extra := JSON.stringify({"method": auth_method}) if not auth_method.is_empty() else ""
	Global.metrics.track_screen_viewed("AUTH_BROWSER_OPEN", extra)
	Global.metrics.flush.call_deferred()
	sign_in_logo.hide()
	sign_in_logo_sep.hide()
	container_sign_in_step1.hide()
	container_sign_in_step2.show()
	button_back.hide()
	show_panel(control_signin)

	label_step2_title.text = message
	label_step2_title.show()
	auth_error_label.hide()
	auth_spinner.show()
	button_cancel.show()
	button_cancel_icon.show()

	# Mark that we're waiting for browser auth
	auth_waiting_for_browser = true

	# Timer pauses on FOCUS_OUT and restarts on FOCUS_IN
	auth_timeout_timer.stop()


func show_ftue_screen():
	track_lobby_screen("DISCOVER_FTUE")
	button_back.hide()
	var nickname_label = ftue_screen.get_node_or_null("%Label_NickNameFTUE")
	if nickname_label and current_profile:
		nickname_label.text = current_profile.get_name()
	show_panel(ftue_screen)
	_async_fetch_ftue_place()


func _async_fetch_ftue_place() -> void:
	var response = await PlacesHelper.async_get_place_by_id(FTUE_PLACE_ID)
	if response is PromiseError:
		printerr("[Lobby] Failed to fetch FTUE place data: ", response.get_error())
		return
	if not is_instance_valid(ftue_screen):
		return
	var json: Dictionary = response.get_string_response_as_json()
	var place_data: Dictionary = json.get("data", json)
	if place_data.is_empty():
		return
	ftue_screen.set_data(place_data)


func show_avatar_create_screen():
	track_lobby_screen("AVATAR_CREATE")
	button_back.hide()
	show_panel(control_backpack)


# ADR-290: Snapshots no longer uploaded
func async_close_sign_in():
	Global.metrics.update_identity(
		Global.player_identity.get_address_str(), Global.player_identity.is_guest
	)

	# Auth Success metric
	Global.metrics.track_auth_success()

	if _should_go_to_explorer_from_deeplink():
		go_to_explorer()
		return

	if Global.is_xr():
		change_scene.emit("res://src/ui/components/menu/menu.tscn")
	else:
		get_tree().change_scene_to_file("res://src/ui/components/menu/menu.tscn")


# gdlint:ignore = async-function-name
func _ready():
	print("[Startup] lobby._ready start: %dms" % (Time.get_ticks_msec() - Global._startup_time))
	label_version.set_text(DclGlobal.get_version_with_env())
	button_enter_as_guest.visible = false

	Global.music_player.play.call_deferred("music_builder")
	control_restore_and_choose_name.hide()

	var login = %Login

	ready_for_redirect_by_deep_link = false
	Global.deep_link_received.connect(_on_deep_link_received)

	login.set_lobby(self)
	login.show()

	show_loading_screen()
	var startup_time_ms: int = Time.get_ticks_msec() - Global._startup_time
	print("[Startup] lobby.show_loading_screen: %dms" % startup_time_ms)

	# Track startup metric for analytics
	var startup_data := {"startup_time_ms": startup_time_ms, "platform": OS.get_name()}
	Global.metrics.track_screen_viewed("START", JSON.stringify(startup_data))

	# Run hardware benchmark AFTER loading screen is visible to avoid black screen
	# on iOS fresh install (Metal shader compilation can take 10-20s)
	if Global.should_run_first_launch_benchmark():
		print("[Startup] lobby: triggering first launch benchmark")
		Global.run_first_launch_benchmark()

	UiSounds.install_audio_recusirve(self)
	Global.dcl_tokio_rpc.need_open_url.connect(self._on_need_open_url)
	Global.player_identity.profile_changed.connect(self._async_on_profile_changed)
	Global.player_identity.wallet_connected.connect(self._on_wallet_connected)
	Global.player_identity.auth_error.connect(self._on_auth_error)

	# Create auth timeout timer
	auth_timeout_timer = Timer.new()
	auth_timeout_timer.one_shot = true
	auth_timeout_timer.timeout.connect(self._on_auth_timeout)
	add_child(auth_timeout_timer)

	Global.scene_runner.set_pause(true)

	if Global.cli.skip_lobby:
		_skip_lobby = true
	if Global.cli.skip_lobby_to_menu:
		_skip_lobby_to_menu = true

	# Preview deeplink: create guest and skip lobby for hot reload development
	if not Global.deep_link_obj.preview.is_empty():
		_skip_lobby = true

	var session_account: Dictionary = Global.get_config().session_account

	if Global.cli.guest_profile or not Global.deep_link_obj.preview.is_empty():
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
	elif _skip_lobby_to_menu:
		show_loading_screen()
		get_tree().change_scene_to_file.call_deferred("res://src/ui/components/menu/menu.tscn")
	else:
		var current_eula_version: int = Global.get_config().terms_and_conditions_version
		# Force show EULA when benchmarking (even if already accepted)
		if (
			Global.cli.benchmark_report
			or current_eula_version != Global.TERMS_AND_CONDITIONS_VERSION
		):
			if Global.cli.benchmark_report:
				print("✓ Forcing EULA for benchmark flow")
			show_eula_screen()
		else:
			show_account_home_screen()


func _notification(what: int) -> void:
	# Android back button / hardware back
	if what == NOTIFICATION_WM_GO_BACK_REQUEST:
		_handle_back_action()
		return

	# On mobile, pause/resume auth timeout when app loses/gains focus
	# This prevents timeout while user is in external browser for auth
	if not Global.is_mobile() or Global.is_virtual_mobile():
		return

	if not auth_waiting_for_browser:
		return

	if what == NOTIFICATION_APPLICATION_FOCUS_OUT:
		# App lost focus (browser opened) - stop the timer
		if auth_timeout_timer != null:
			auth_timeout_timer.stop()
	elif what == NOTIFICATION_APPLICATION_FOCUS_IN:
		# App regained focus (returned from browser) - update text and start timeout
		label_step2_title.text = "Signing in..."
		sign_in_logo.show()
		sign_in_logo_sep.show()
		if auth_timeout_timer != null:
			auth_timeout_timer.start(AUTH_TIMEOUT_SECONDS)


func go_to_explorer():
	if is_inside_tree():
		get_tree().change_scene_to_file("res://src/ui/explorer.tscn")


## Check if any deeplink parameter should redirect to explorer (preview, realm, or location)
func _should_go_to_explorer_from_deeplink() -> bool:
	return (
		Global.deep_link_obj.is_location_defined()
		or not Global.deep_link_obj.realm.is_empty()
		or not Global.deep_link_obj.preview.is_empty()
	)


func _async_on_profile_changed(new_profile: DclUserProfile):
	current_profile = new_profile
	await avatar_preview.avatar.async_update_avatar_from_profile(new_profile)

	if !new_profile.has_connected_web3():
		Global.get_config().guest_profile = new_profile.to_godot_dictionary()
		Global.get_config().save_to_settings_file()
		restore_name_footer.hide()
		label_name.hide()
		choose_name_head.show()
		choose_name_footer.show()

	if loading_first_profile:
		loading_first_profile = false
		if profile_has_name():
			# Session restored with existing profile — go directly to discover
			Global.metrics.update_identity(
				Global.player_identity.get_address_str(), Global.player_identity.is_guest
			)
			await async_close_sign_in()
			return
# gdlint: ignore=no-else-return
		else:
			show_account_home_screen()

	if _skip_lobby:
		go_to_explorer()
	elif _skip_lobby_to_menu:
		get_tree().change_scene_to_file("res://src/ui/components/menu/menu.tscn")

	if waiting_for_new_wallet:
		waiting_for_new_wallet = false
		if profile_has_name():
			# User has an existing profile: show Welcome Back screen
			label_avatar_name.set_text(new_profile.get_name())
			show_restore_screen()
			_show_avatar_preview()
			Global.metrics.update_identity(
				Global.player_identity.get_address_str(), Global.player_identity.is_guest
			)
		else:
			# No profile yet: go to avatar customization + naming
			create_guest_account_if_needed()
			_show_avatar_preview()
			show_avatar_create_screen()
	else:
		ready_for_redirect_by_deep_link = true
		if _should_go_to_explorer_from_deeplink():
			go_to_explorer()
			return


func _on_need_open_url(url: String, _description: String, use_webview: bool) -> void:
	Global.open_url(url, use_webview)


func _on_wallet_connected(_address: String, _chain_id: int, _is_guest: bool) -> void:
	_stop_auth_timeout()
	Global.get_config().session_account = {}

	var new_stored_account := {}
	if Global.player_identity.get_recover_account_to(new_stored_account):
		Global.get_config().session_account = new_stored_account

	Global.get_config().save_to_settings_file()

	# Note: Social service initialization moved to explorer.gd to ensure it completes
	# before the Friends panel is used (lobby scene transitions before it finishes)


func _on_check_box_eula_toggled(toggled_on: bool) -> void:
	button_accept_eula.disabled = !toggled_on


func _on_eula_check_area_gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		if event.pressed:
			checkbox_eula.button_pressed = !checkbox_eula.button_pressed


func _on_eula_meta_clicked(meta: Variant) -> void:
	Global.open_webview_url(meta)


func _on_button_accept_eula_pressed() -> void:
	Global.metrics.track_screen_viewed("ACCEPT_EULA", "")
	Global.metrics.track_click_button("accept", "ACCEPT_EULA", "")
	Global.metrics.flush()
	Global.get_config().terms_and_conditions_version = Global.TERMS_AND_CONDITIONS_VERSION
	Global.get_config().save_to_settings_file()
	show_account_home_screen()


func _on_button_update_pressed() -> void:
	Global.metrics.track_click_button("update", current_screen_name, "")
	# TODO: Open the appropriate app store URL based on platform
	if Global.is_ios():
		Global.open_webview_url("https://apps.apple.com/app/decentraland")
	elif Global.is_android():
		Global.open_webview_url(
			"https://play.google.com/store/apps/details?id=org.decentraland.explorer"
		)
	else:
		Global.open_webview_url("https://decentraland.org/download")


func _on_button_not_now_pressed() -> void:
	Global.metrics.track_click_button("not_now", current_screen_name, "")
	show_account_home_screen()


func _on_button_different_account_pressed():
	Global.metrics.update_identity("unauthenticated", false)
	Global.metrics.track_click_button("use_another_account", current_screen_name, "")
	Global.get_config().session_account = {}

	# Unsubscribe from block updates before clearing
	Global.social_service.unsubscribe_from_block_updates()

	# Clear the current social blacklist when switching accounts
	Global.social_blacklist.clear_blocked()
	Global.social_blacklist.clear_muted()

	Global.get_config().save_to_settings_file()
	show_account_home_screen()


func _on_button_continue_pressed():
	Global.metrics.track_click_button("next", current_screen_name, "")
	_async_on_profile_changed(Global.player_identity.get_mutable_profile())
	show_avatar_naming_screen()


func _on_button_start_pressed():
	Global.metrics.track_click_button("create_account", current_screen_name, "")
	button_enter_as_guest.visible = false
	sign_in_title.text = "Create your account"
	is_creating_account = true
	show_auth_method_screen()


# gdlint:ignore = async-function-name
func _on_button_next_pressed():
	Global.metrics.track_click_button("next", current_screen_name, "")
	if dcl_line_edit.line_edit.text.is_empty():
		return

	avatar_preview.hide()
	show_loading_screen()
	current_profile.set_name(dcl_line_edit.line_edit.text)
	current_profile.set_has_connected_web3(!Global.player_identity.is_guest)
	var avatar := current_profile.get_avatar()

	# ADR-290: Snapshots are no longer generated/uploaded by clients
	current_profile.set_avatar(avatar)

	# TODO: REMOVE THIS BEFORE MERGE, USEFUL FOR TESTING NEW ACCOUNT
	#var promise = ProfileService.async_deploy_profile(current_profile)
	#await PromiseUtils.async_awaiter(promise)
	#if promise.is_rejected():
	#printerr("[Lobby] Profile deploy failed: ", promise.get_reject_reason())

	show_ftue_screen()


func _on_button_random_name_pressed():
	dcl_line_edit.set_text_value(RandomGeneratorUtil.generate_unique_name())


func _on_button_go_to_sign_in_pressed():
	Global.metrics.track_click_button("sign_in", current_screen_name, "")
	button_enter_as_guest.hide()
	sign_in_title.text = "Sign in to Decentraland"
	is_creating_account = false
	show_auth_method_screen()


func _on_button_back_pressed():
	Global.metrics.track_click_button("back", current_screen_name, "")
	match current_screen_name:
		"ACCOUNT_HOME":
			show_eula_screen()
		"AVATAR_NAMING":
			show_avatar_create_screen()
		_:
			show_account_home_screen()


func _handle_back_action():
	match current_screen_name:
		"ACCOUNT_HOME":
			show_eula_screen()
		"AUTH_HOME_ANDROID", "AUTH_HOME_IOS", "AUTH_HOME_DESKTOP":
			show_account_home_screen()
		"AUTH_BROWSER_OPEN":
			_on_button_cancel_pressed()
		"AVATAR_NAMING":
			show_avatar_create_screen()


func _on_button_cancel_pressed():
	Global.metrics.track_click_button("cancel", current_screen_name, "")
	_stop_auth_timeout()
	Global.player_identity.abort_try_connect_account()
	show_auth_method_screen()


func show_auth_error_screen(error_message: String):
	track_lobby_screen("AUTH_ERROR")
	label_error_message.text = error_message
	button_back.hide()
	show_panel(control_auth_error)


func _on_auth_error(error_message: String):
	_stop_auth_timeout()
	show_auth_error_screen(error_message)


func _on_auth_timeout():
	Global.player_identity.abort_try_connect_account()
	show_auth_error_screen("Authentication timed out. Please try again.")


func _on_button_try_again_pressed():
	Global.metrics.track_click_button("try_again", current_screen_name, "")
	show_auth_method_screen()


func _stop_auth_timeout():
	auth_waiting_for_browser = false
	if auth_timeout_timer != null:
		auth_timeout_timer.stop()


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
	avatar_preview.avatar.emote_controller.async_play_emote("wave")


# gdlint:ignore = async-function-name
func _on_button_jump_in_pressed():
	Global.metrics.track_click_button("lets_go", current_screen_name, "")
	await async_close_sign_in()


func _on_avatar_preview_gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		if event.pressed:
			if not avatar_preview.avatar.emote_controller.is_playing():
				if dcl_line_edit.line_edit.text.contains("dancer"):
					avatar_preview.avatar.emote_controller.async_play_emote("dance")
				else:
					avatar_preview.avatar.emote_controller.async_play_emote("wave")


func _on_deep_link_received():
	if ready_for_redirect_by_deep_link:
		go_to_explorer.call_deferred()


func _on_dcl_line_edit_dcl_line_edit_changed() -> void:
	button_next.disabled = dcl_line_edit.error
	if dcl_line_edit.error:
		if not avatar_preview.avatar.emote_controller.is_playing() or _playing != "shrug":
			avatar_preview.avatar.emote_controller.async_play_emote("shrug")
			_playing = "shrug"
	else:
		if (
			!button_next.disabled and not avatar_preview.avatar.emote_controller.is_playing()
			or _playing != "fistpump"
		):
			avatar_preview.avatar.emote_controller.async_play_emote("fistpump")
			_playing = "fistpump"


func _on_ftue_ftue_completed() -> void:
	Global.get_config().discover_ftue_completed = true
	Global.get_config().save_to_settings_file()
	async_close_sign_in()


func _on_ftue_jump_in(parcel_position: Vector2i, realm_str: String) -> void:
	Global.get_config().discover_ftue_completed = true
	Global.get_config().save_to_settings_file()
	Global.teleport_to(parcel_position, realm_str)


func _on_ftue_jump_in_world(realm_str: String) -> void:
	Global.get_config().discover_ftue_completed = true
	Global.get_config().save_to_settings_file()
	Global.join_world(realm_str)
