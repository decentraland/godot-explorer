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

const FTUE_PLACE_ID: String = "780f04dd-eba1-41a8-b109-74896c87e98b"
const LOGO_TAP_TIMEOUT: float = 0.5  # seconds to reset tap count

var is_creating_account: bool = false

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
var _logo_tap_count: int = 0
var _logo_tap_timer: float = 0.0

@onready var control_main = %Main

@onready var dcl_line_edit: VBoxContainer = %DclLineEdit

@onready var control_loading = %Loading
@onready var control_eula = %Eula
@onready var control_version_upgrade = %VersionUpgrade
@onready var control_signin = %SignIn
@onready var control_start = %Start
@onready var control_backpack = %BackpackContainer
@onready var control_choose_name: Control = %ChooseName
@onready var control_comeback: Control = %Comeback

@onready var loading_solid_bg: ColorRect = %LoadingSolidBg
@onready var default_bg: TextureRect = %DefaultBg
@onready var discover_bg: TextureRect = $DiscoverBg

@onready var container_sign_in_step1 = %VBoxContainer_SignInStep1
@onready var container_sign_in_step2 = %VBoxContainer_SignInStep2
@onready var auth_spinner_container = %VBoxContainer_AuthSpinner
@onready var auth_error_container = %VBoxContainer_AuthError
@onready var auth_error_label_main = %AuthErrorLabel
@onready var auth_error_label_code = %AuthErrorCodeLabel
@onready var button_cancel = %Button_Cancel
@onready var label_step2_title: Label = %VBoxContainer_SignInStep2/Label_Title

@onready
var button_try_again: Button = $Main/SignIn/MarginContainer/VBoxFixed/VBoxContainer/VBoxContainer_SignInStep2/Button_TryAgain

@onready var avatar_preview_container_comeback: Control = %AvatarPreviewContainer_Comeback
@onready var avatar_preview_container_choose_name: Control = %AvatarPreviewContainer_ChooseName
@onready var avatar_preview: AvatarPreview = %AvatarPreview
@onready var button_next = %Button_Next

@onready var backpack = %Backpack
@onready
var label_signed_as_name: Label = $Main/Comeback/MarginContainer/VBoxContainer/RestoreNameHead/Label_SignedAsName

@onready var button_continue_as_guest: Button = %Button_ContinueAsGuest
@onready var button_enter_as_disposable_account: Button = %Button_EnterAsDisposableAccount
@onready var button_back: Button = %Button_Back
@onready var sign_in_title: Label = %SignInTitle
@onready var sign_in_logo: TextureRect = %SignInLogo

@onready var checkbox_eula: CheckBox = %CheckBox_Eula
@onready var button_accept_eula: Button = %Button_AcceptEula

@onready var label_version = %Label_Version

@onready var control_ftue = %FTUE
@onready var ftue_screen = %FTUE/FTUE

@onready var backgrounds = [loading_solid_bg, default_bg, discover_bg]
@onready var control_with_discover_bg = [
	control_ftue, control_comeback, control_choose_name, control_backpack
]


func show_panel(child_node: Control, subpanel: Control = null):
	for bg in backgrounds:
		bg.hide()

	if child_node == control_loading:
		loading_solid_bg.show()
	elif control_with_discover_bg.has(child_node):
		discover_bg.show()
	else:
		default_bg.show()

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


func show_comeback_screen():
	track_lobby_screen("COMEBACK")
	button_back.hide()
	show_panel(control_comeback)
	avatar_preview.reparent(avatar_preview_container_comeback)


func show_avatar_naming_screen():
	track_lobby_screen("AVATAR_NAMING")
	button_back.hide()
	show_panel(control_choose_name)
	avatar_preview.reparent(avatar_preview_container_choose_name)


func show_loading_screen():
	current_screen_name = "LOBBY_LOADING"
	button_back.hide()
	show_panel(control_loading)
	loading_solid_bg.show()
	default_bg.hide()


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
	container_sign_in_step1.show()
	container_sign_in_step2.hide()
	button_back.show()
	show_panel(control_signin)


func show_auth_browser_open_screen(
	message: String = "Opening browser...", auth_method: String = ""
):
	current_screen_name = "AUTH_BROWSER_OPEN"
	var extra := JSON.stringify({"method": auth_method}) if not auth_method.is_empty() else ""
	Global.metrics.track_screen_viewed("AUTH_BROWSER_OPEN", extra)
	Global.metrics.flush.call_deferred()
	container_sign_in_step1.hide()
	container_sign_in_step2.show()
	button_back.hide()
	show_panel(control_signin)

	label_step2_title.text = message
	label_step2_title.show()
	auth_error_container.hide()
	auth_spinner_container.show()
	button_cancel.show()
	button_try_again.hide()


func show_control_ftue():
	current_screen_name = "DISCOVER_FTUE"
	button_back.hide()
	if current_profile:
		ftue_screen.set_username(current_profile.get_name())
	show_panel(control_ftue)
	ftue_screen.load_places()


func show_avatar_create_screen():
	track_lobby_screen("AVATAR_CREATE")
	button_back.hide()
	show_panel(control_backpack)


# ADR-290: Snapshots no longer uploaded
func async_close_sign_in():
	if _should_go_to_explorer_from_deeplink():
		go_to_explorer()
		return

	if Global.is_xr():
		change_scene.emit("res://src/ui/components/organisms/menu/menu.tscn")
	else:
		get_tree().change_scene_to_file("res://src/ui/components/organisms/menu/menu.tscn")


# gdlint:ignore = async-function-name
func _ready():
	print("[Startup] lobby._ready start: %dms" % (Time.get_ticks_msec() - Global._startup_time))
	label_version.set_text(DclGlobal.get_version_with_env())
	button_enter_as_disposable_account.visible = false

	# The backpack ships with top_node_margin pointing at its own navbar (hidden
	# via hide_navbar=true here), which leaves the preview without a real top
	# reference. Wire the embedded preview to the lobby's "Create your avatar"
	# label so the camera-fit overlap math has a properly-positioned anchor,
	# and snap the avatar to the viewport top so it shows full-height behind
	# the editor overlay instead of being sized into the uncovered slice.
	var create_avatar_label: Label = $Main/BackpackContainer/MarginContainer/VBoxContainer/Label_Name
	backpack.avatar_preview.snap_top_to_viewport = true
	backpack.avatar_preview.preview_margin_top = 30
	backpack.avatar_preview.set_top_margin_node(create_avatar_label)

	# Secret guest mode: double-tap logo when not in prod
	sign_in_logo.gui_input.connect(_on_sign_in_logo_gui_input)

	# Lobby fires onboarding/auth events one at a time — ship them on a snappy 2s cadence.
	# Menu/Explorer restore the default 10s when leaving the lobby.
	Global.metrics.set_flush_interval(2.0)

	Global.music_player.play.call_deferred("music_builder")

	var login = %Login

	ready_for_redirect_by_deep_link = false
	Global.deep_link_router.deep_link_received.connect(_on_deep_link_received)

	login.set_lobby(self)
	login.show()

	show_loading_screen()
	var startup_time_ms: int = Time.get_ticks_msec() - Global._startup_time
	print("[Startup] lobby.show_loading_screen: %dms" % startup_time_ms)

	if Global.is_mobile():
		var gate_decision := await _async_run_version_gate()
		if gate_decision == "hard":
			# Overlay blocks interaction; loading screen stays behind it.
			return

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

	Global.scene_runner.set_pause(true)

	if Global.cli.skip_lobby:
		_skip_lobby = true
	if Global.cli.skip_lobby_to_menu:
		_skip_lobby_to_menu = true
	if Global.is_gp_benchmark():
		_skip_lobby = true

	# Preview deeplink: create guest and skip lobby for hot reload development
	if not Global.deep_link_obj.preview.is_empty():
		_skip_lobby = true

	var session_account: Dictionary = Global.get_config().session_account

	if (
		Global.cli.guest_profile
		or Global.is_gp_benchmark()
		or not Global.deep_link_obj.preview.is_empty()
	):
		# Mark session as ephemeral so guest data is never persisted to disk,
		# preserving any previously saved wallet session.
		Global.get_config().session_is_ephemeral = true
		# Use assignment instead of clear() to avoid mutating the dictionary in-place.
		# clear() would also corrupt the reference inside settings_file, causing the
		# copy loop in save_to_settings_file() to lose the saved wallet session.
		session_account = {}
		Global.player_identity.create_disposable_account()
		if Global.is_gp_benchmark():
			var fixed_profile := DclUserProfile.randomized_with_seed(1862)
			fixed_profile.set_ethereum_address(Global.player_identity.get_address_str())
			Global.player_identity.set_profile(fixed_profile)
		else:
			Global.player_identity.set_random_profile()
		var random_profile = Global.player_identity.get_profile_or_null()
		if random_profile != null:
			Global.get_config().guest_profile = random_profile.to_godot_dictionary()

	# Flag the wallet_connected emission produced by try_recover_account so the analytics
	# controller skips emitting a Firebase `login_success` for it. Safe to call unconditionally:
	# the clear runs deferred and just unblocks the next legitimate fresh login.
	if Global.analytics_controller != null:
		Global.analytics_controller.mark_wallet_connected_as_recovery()
	var recovered := Global.player_identity.try_recover_account(session_account)
	if recovered:
		loading_first_profile = true
		show_loading_screen()
	elif _skip_lobby:
		show_loading_screen()
		go_to_explorer.call_deferred()
	elif _skip_lobby_to_menu:
		show_loading_screen()
		get_tree().change_scene_to_file.call_deferred(
			"res://src/ui/components/organisms/menu/menu.tscn"
		)
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

	if what == NOTIFICATION_APPLICATION_FOCUS_OUT:
		if current_screen_name == "AUTH_BROWSER_OPEN":
			label_step2_title.text = label_step2_title.text.replace("Opening", "Waiting")


func _process(delta: float) -> void:
	# Reset logo tap count after timeout
	if _logo_tap_count > 0:
		_logo_tap_timer += delta
		if _logo_tap_timer >= LOGO_TAP_TIMEOUT:
			_logo_tap_count = 0
			_logo_tap_timer = 0.0


func _on_sign_in_logo_gui_input(event: InputEvent) -> void:
	# Secret guest mode: double-tap logo when not in prod
	if DclGlobal.is_production():
		return

	if event is InputEventScreenTouch and event.pressed:
		_logo_tap_timer = 0.0
		_logo_tap_count += 1

		if _logo_tap_count >= 2:
			_logo_tap_count = 0
			button_enter_as_disposable_account.visible = true


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

	if loading_first_profile:
		loading_first_profile = false
		if profile_has_name():
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
		get_tree().change_scene_to_file("res://src/ui/components/organisms/menu/menu.tscn")

	if waiting_for_new_wallet:
		waiting_for_new_wallet = false
		if profile_has_name():
			# User has an existing profile: show Welcome Back screen
			label_signed_as_name.set_text("You're signed in as\n%s." % [new_profile.get_name()])
			show_comeback_screen()
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


func _on_wallet_connected(address: String, _chain_id: int, is_guest: bool) -> void:
	Global.metrics.update_identity(address, is_guest)
	Global.metrics.track_screen_viewed("AUTH_SUCCESS", "")
	Global.metrics.flush.call_deferred()

	Global.get_config().session_account = {}

	var new_stored_account := {}
	if Global.player_identity.get_recover_account_to(new_stored_account):
		Global.get_config().session_account = new_stored_account
	else:
		push_error("[recovery] get_recover_account_to returned false for address=%s" % address)

	Global.get_config().save_to_settings_file()

	# Initialize social service early so Discover can show friends before entering explorer
	if not is_guest:
		Global.social_service.initialize_from_player_identity(Global.player_identity)


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
	# Opens the consent gate (auto-flushes queued pre-consent events) and fires Firebase
	# `eula_accepted` Key Event. All Firebase/Segment orchestration lives in the controller.
	if Global.analytics_controller != null:
		Global.analytics_controller.on_eula_accepted_locally()
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

	# Unsubscribe from all social service updates before clearing
	Global.social_service.unsubscribe_from_updates()
	Global.social_service.unsubscribe_from_connectivity_updates()
	Global.social_service.unsubscribe_from_block_updates()
	# Drop the gRPC manager so the previous identity's streams don't leak into
	# the next account. initialize_from_player_identity rebuilds it on login.
	Global.social_service.disconnect()

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
	sign_in_title.text = "Create your account"
	is_creating_account = true
	show_auth_home_screen()


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

	var promise = ProfileService.async_deploy_profile(current_profile)
	await PromiseUtils.async_awaiter(promise)
	if promise.is_rejected():
		var error: PromiseError = promise.get_data()
		printerr("[Lobby] Profile deploy failed: ", error.get_error())
	else:
		Global.metrics.track_screen_viewed("AUTH_DEPLOY_SUCCESS", "")
		Global.metrics.flush.call_deferred()

	show_control_ftue()


func _on_button_random_name_pressed():
	dcl_line_edit.set_text_value(RandomGeneratorUtil.generate_unique_name())


func _on_button_go_to_sign_in_pressed():
	Global.metrics.track_click_button("sign_in", current_screen_name, "")
	sign_in_title.text = "Sign in to Decentraland"
	is_creating_account = false
	show_auth_home_screen()


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

	Global.player_identity.abort_try_connect_account()
	show_auth_home_screen()


func _on_button_try_again_pressed():
	Global.metrics.track_click_button("try_again", current_screen_name, "")

	show_auth_home_screen()


func _show_auth_error(error_message: String):
	track_lobby_screen("AUTH_ERROR")
	auth_spinner_container.hide()
	label_step2_title.text = "Authentication failed"
	auth_error_label_main.text = error_message
	auth_error_label_code.text = ""
	auth_error_container.show()
	button_cancel.hide()
	button_try_again.show()


func _on_auth_error(error_message: String):
	_show_auth_error(error_message)


func create_guest_account_if_needed():
	if not guest_account_created:
		# Don't create a guest account if the user already has a web3 wallet connected
		# (e.g., MetaMask via WalletConnect). Creating a guest account would overwrite
		# the web3 wallet with a random local wallet.
		if (
			not Global.player_identity.is_guest
			and not Global.player_identity.get_address_str().is_empty()
		):
			guest_account_created = true
			return

		Global.get_config().guest_profile = {}
		Global.get_config().save_to_settings_file()
		Global.player_identity.create_disposable_account()
		if is_creating_account:
			Global.player_identity.set_profile(current_profile)
		else:
			Global.player_identity.set_default_profile()
		guest_account_created = true


func profile_has_name():
	var profile = Global.player_identity.get_profile_or_null()
	return profile != null and not profile.get_name().is_empty()


func _on_button_enter_as_disposable_account_pressed():
	Global.metrics.track_click_button("enter_as_disposable_account", current_screen_name, "")
	Global.get_config().guest_profile = {}
	Global.get_config().save_to_settings_file()
	guest_account_created = false
	Global.player_identity.create_disposable_account()
	Global.player_identity.set_default_profile()
	guest_account_created = true
	_show_avatar_preview()
	show_avatar_create_screen()


# Resolves the platform-native device anchor (SSAID on Android, Keychain UUID
# on iOS) used to derive a deterministic thirdweb guest wallet. Empty string
# means "no native anchor available" — Rust falls back to a UUID stored in
# user:// so desktop still works.
#
# Android note: `has_method()` always returns false for JNISingleton methods
# (Object.has_method consults ClassDB, the Android plugin method_map is
# separate). Don't guard the call with has_method or it silently no-ops.
# See: https://github.com/godotengine/godot/issues/106436
func _get_device_anchor_id() -> String:
	if Global.is_android():
		var plugin = Engine.get_singleton("dcl-godot-android")
		if plugin != null:
			return plugin.getDeviceAnchorId()
	elif Global.is_ios():
		var plugin = Engine.get_singleton("DclGodotiOS")
		if plugin != null and plugin.has_method("get_device_anchor_id"):
			return plugin.get_device_anchor_id()
	return ""


# gdlint:ignore = async-function-name
func _on_button_continue_as_guest_pressed():
	Global.metrics.track_click_button("continue_as_guest", current_screen_name, "")
	button_continue_as_guest.disabled = true

	# Mirror the social-login flow: flag `waiting_for_new_wallet` so the
	# wallet_connected → async_fetch_profile → profile_changed chain routes
	# us to comeback screen (existing profile) or avatar create (new wallet)
	# instead of forcing one or the other manually.
	waiting_for_new_wallet = true

	var anchor: String = _get_device_anchor_id()
	var promise: Promise = Global.player_identity.async_create_guest_account(anchor)
	var result = await PromiseUtils.async_awaiter(promise)

	button_continue_as_guest.disabled = false

	if result is PromiseError:
		waiting_for_new_wallet = false
		var error_text: String = result.get_error()
		push_error("Guest login failed: " + error_text)
		_show_auth_error("Could not start guest session: " + error_text)


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
	if is_inside_tree():
		async_close_sign_in()


func _on_ftue_jump_in(parcel_position: Vector2i, realm_str: String) -> void:
	Global.async_teleport_to(parcel_position, realm_str)


func _on_ftue_jump_in_world(realm_str: String) -> void:
	Global.async_join_world(realm_str)


func _async_run_version_gate() -> String:
	var gate: Node = preload("res://src/version_gate.gd").new()
	add_child(gate)
	var result: String = await gate.async_check()
	if result == "hard":
		gate.show_overlay(false)
	elif result == "soft":
		gate.show_overlay(true)
	return result
