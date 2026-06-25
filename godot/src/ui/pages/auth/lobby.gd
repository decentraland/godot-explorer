class_name Lobby
extends Control
## Lobby entry flow controller.
##
## Panel mapping (Figma screen -> Godot node):
##   VERSION_UPGRADE     -> %VersionUpgrade       (update / not now)
##   ACCOUNT_HOME        -> %AccountHome           (play as guest / sign in)
##   AUTH_HOME           -> %SignIn step1          (auth provider buttons)
##   AUTH_BROWSER_OPEN   -> %SignIn step2          (spinner + cancel)
##   AVATAR_CUSTOMIZE    -> %AvatarCustomize        (backpack avatar editor)
##   AVATAR_NAMING       -> %AvatarNaming           (choose-name mode)
##   COMEBACK            -> %RestoreAndChooseName  (restore mode: welcome back)
##   DCL_SPLASH          -> %DclSplash              (spinner)
##   DISCOVER_FTUE       -> %DiscoverFtue            (first time user experience)
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
# Guest-login (thirdweb) can hang on a flaky network and leave the user stuck on
# the "Getting you ready..." screen forever. Cap the wait and surface a retry.
const GUEST_LOGIN_TIMEOUT_SEC: float = 20.0
# Debug-only: on-disk guest identity state wiped by the "reset guest wallet"
# affordance (revealed via the secret logo double-tap in non-prod). These paths
# are owned by Rust — keep in sync with lib/src/auth/device_anchor.rs (anchor)
# and lib/src/auth/thirdweb_guest.rs (persisted session).
const DEBUG_GUEST_STATE_FILES = [
	"user://device_anchor.txt",
	"user://thirdweb_session.json",
]
const BG_GRADIENT = preload("res://assets/backgrounds/gradient-background.png")
const BG_DISCOVER = preload("res://assets/backgrounds/photo-background.png")
const BG_AVATAR = preload("res://assets/backgrounds/gradient-background.tres")

var is_creating_account: bool = false

var current_profile: DclUserProfile
var guest_account_created: bool = false

var waiting_for_new_wallet: bool = false
var ready_for_redirect_by_deep_link: bool = false

var loading_first_profile: bool = false
var current_screen_name: String = ""

# Debug-only "reset guest wallet" button, created at runtime in non-prod and
# revealed alongside the disposable-account button by the secret logo double-tap.
var button_reset_guest_debug: Button = null

var _skip_lobby: bool = false
var _skip_lobby_to_menu: bool = false
var _last_panel: Control = null
var _playing: String
var _logo_tap_count: int = 0
var _logo_tap_timer: float = 0.0
var _avatar_preview_defaults: Dictionary = {}
var _discard_edit_confirmed = false

# Monotonic token for the guest-login watchdog. Bumped on every attempt (and on
# failure) so a stale watchdog can't clobber a newer attempt. See the watchdog.
var _guest_login_attempt: int = 0

@onready var control_main = %Main
@onready var dcl_line_edit: VBoxContainer = %DclLineEdit
@onready var control_dcl_splash = %DclSplash
@onready var control_version_upgrade = %VersionUpgrade
@onready var control_signin = %SignIn
@onready var control_account_home = %AccountHome
@onready var control_account_home_loading = %AccountHomeLoading
@onready var control_avatar_create = %AvatarCreate
@onready var preset_carousel: PresetAvatarCarousel = %PresetAvatarCarousel
@onready var control_avatar_customize = %AvatarCustomize
@onready var control_avatar_naming: Control = %AvatarNaming
@onready var control_comeback: Control = %Comeback
@onready var background: TextureRect = %Background
@onready var container_sign_in_step1 = %VBoxContainer_SignInStep1
@onready var container_sign_in_step2 = %VBoxContainer_SignInStep2
@onready var auth_spinner_container = %VBoxContainer_AuthSpinner
@onready var auth_error_container = %VBoxContainer_AuthError
@onready var auth_error_label_main = %AuthErrorLabel
@onready var auth_error_label_code = %AuthErrorCodeLabel
@onready var button_cancel = %Button_Cancel
@onready var label_step2_title: Label = %Label_Step2_Title
@onready var button_try_again: Button = %Button_TryAgain
@onready var avatar_preview_container_avatar_create: Control = %AvatarPreviewContainer_AvatarCreate
@onready var avatar_preview_container_comeback: Control = %AvatarPreviewContainer_Comeback
@onready var avatar_preview_container_avatar_naming: Control = %AvatarPreviewContainer_AvatarNaming
@onready var avatar_preview: AvatarPreview = %AvatarPreview
@onready var avatar_loading: MarginContainer = %AvatarLoadingContainer
@onready var button_lets_go = %Button_LetsGo
@onready var backpack = %Backpack
@onready var label_signed_as_name: Label = %Label_SignedAsName
@onready var button_play_as_guest: Button = %Button_PlayAsGuest
@onready var button_enter_as_disposable_account: Button = %Button_EnterAsDisposableAccount
@onready var button_back: Button = %Button_Back
@onready var sign_in_title: Label = %SignInTitle
@onready var label_version = %Label_Version
@onready var control_discover_ftue = %DiscoverFtue
@onready var ftue_screen = %DiscoverFtue/FTUE
@onready var control_with_discover_bg = [control_account_home, control_account_home_loading]


func set_background(texture: Texture2D) -> void:
	background.texture = texture
	background.show()


func show_panel(child_node: Control, subpanel: Control = null):
	if child_node == control_dcl_splash:
		set_background(BG_GRADIENT)
	elif control_with_discover_bg.has(child_node):
		set_background(BG_DISCOVER)
	elif child_node == control_avatar_naming or child_node == control_avatar_create:
		set_background(BG_AVATAR)
	else:
		set_background(BG_GRADIENT)

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
	_restore_avatar_preview_defaults()


# gdlint:ignore = async-function-name
func show_avatar_naming_screen():
	track_lobby_screen("AVATAR_NAMING")
	button_back.show()
	show_panel(control_avatar_naming)
	avatar_preview.reparent(avatar_preview_container_avatar_naming)
	_restore_avatar_preview_defaults()
	_on_button_random_name_pressed()
	if current_profile:
		_show_avatar_loading()
		await avatar_preview.avatar.async_update_avatar_from_profile(current_profile)
	_show_avatar_preview()


func show_dcl_splash_screen():
	current_screen_name = "DCL_SPLASH"
	button_back.hide()
	show_panel(control_dcl_splash)


func show_version_upgrade_screen():
	track_lobby_screen("VERSION_UPGRADE")
	button_back.hide()
	show_panel(control_version_upgrade)


func _accept_eula() -> void:
	if Global.analytics_controller != null:
		Global.analytics_controller.on_eula_accepted_locally()
	Global.get_config().terms_and_conditions_version = Global.TERMS_AND_CONDITIONS_VERSION
	Global.get_config().save_to_settings_file()


func show_account_home_screen():
	track_lobby_screen("ACCOUNT_HOME")
	button_back.hide()
	_request_notification_permission_if_needed()
	show_panel(control_account_home)


func show_account_home_loading_screen():
	track_lobby_screen("ACCOUNT_HOME_LOADING")
	button_back.hide()
	show_panel(control_account_home_loading)


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


func show_discover_ftue_screen():
	current_screen_name = "DISCOVER_FTUE"
	button_back.hide()
	if current_profile:
		ftue_screen.set_username(current_profile.get_name())
	show_panel(control_discover_ftue)
	ftue_screen.load_places()


func async_show_avatar_create_screen():
	track_lobby_screen("AVATAR_CREATE")
	button_back.show()
	show_panel(control_avatar_create)
	avatar_preview.reparent(avatar_preview_container_avatar_create)
	_set_avatar_preview_centered()
	avatar_preview.anchors_preset = Control.PRESET_FULL_RECT
	avatar_preview.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	avatar_preview.size_flags_vertical = Control.SIZE_EXPAND_FILL
	var profile = Global.player_identity.get_mutable_profile()
	if profile:
		current_profile = profile
		_show_avatar_loading()
		await avatar_preview.avatar.async_update_avatar_from_profile(current_profile)
	_show_avatar_preview()


# gdlint:ignore = async-function-name
func show_avatar_edit_screen():
	track_lobby_screen("AVATAR_CUSTOMIZE")
	button_back.show()
	backpack.hide_background = false
	show_panel(control_avatar_customize)
	if current_profile:
		Global.player_identity.set_profile(current_profile)
		await get_tree().process_frame
		await get_tree().process_frame
	backpack.request_update_avatar = true
	backpack.request_show_wearables = true


func _on_button_edit_avatar_pressed():
	Global.metrics.track_click_button("edit", current_screen_name, "")
	show_avatar_edit_screen()


func _on_button_avatar_create_next_pressed():
	Global.metrics.track_click_button("next", current_screen_name, "")
	show_avatar_naming_screen()


# gdlint:ignore = async-function-name
func _on_preset_selected(preset_data: Dictionary) -> void:
	if preset_data.is_empty() or current_profile == null:
		return

	var avatar = current_profile.get_avatar()
	avatar.set_body_shape(preset_data.get("body_shape", ""))

	var wearables = PackedStringArray()
	for w in preset_data.get("wearables", []):
		wearables.append(w)
	avatar.set_wearables(wearables)

	var skin = preset_data.get("skin_color", {})
	if not skin.is_empty():
		avatar.set_skin_color(Color(skin.r, skin.g, skin.b, skin.get("a", 1.0)))

	var hair = preset_data.get("hair_color", {})
	if not hair.is_empty():
		avatar.set_hair_color(Color(hair.r, hair.g, hair.b, hair.get("a", 1.0)))

	var eyes = preset_data.get("eye_color", {})
	if not eyes.is_empty():
		avatar.set_eyes_color(Color(eyes.r, eyes.g, eyes.b, eyes.get("a", 1.0)))

	current_profile.set_avatar(avatar)
	_show_avatar_loading()
	await avatar_preview.avatar.async_update_avatar_from_profile(current_profile)
	_show_avatar_preview()


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
	# Back on a clean screen — release the sign-out re-entrancy guard so a future
	# logout can run (no-op on normal startup, where it is already false).
	Global._signing_out = false
	label_version.set_text(DclGlobal.get_version_with_env())
	button_enter_as_disposable_account.visible = false

	_avatar_preview_defaults = {
		"margin_top": avatar_preview.preview_margin_top,
		"margin_bottom": avatar_preview.preview_margin_bottom,
		"margin_left": avatar_preview.preview_margin_left,
		"margin_right": avatar_preview.preview_margin_right,
		"snap_top": avatar_preview.snap_top_to_viewport,
		"top_node": avatar_preview.top_node_margin,
		"bottom_node": avatar_preview.bottom_node_margin,
		"anchors_preset": avatar_preview.anchors_preset,
		"size_flags_h": avatar_preview.size_flags_horizontal,
		"size_flags_v": avatar_preview.size_flags_vertical,
	}

	backpack.avatar_preview.snap_top_to_viewport = true

	avatar_loading.hide()
	avatar_preview.avatar.avatar_loaded.connect(_on_avatar_preview_loaded)

	# Secret guest mode: double-tap logo when not in prod
	_setup_debug_reset_guest_button()
	preset_carousel.preset_selected.connect(_on_preset_selected)

	# Lobby fires onboarding/auth events one at a time — ship them on a snappy 2s cadence.
	# Menu/Explorer restore the default 10s when leaving the lobby.
	Global.metrics.set_flush_interval(2.0)

	Global.music_player.play.call_deferred("music_builder")

	var login = %Login

	ready_for_redirect_by_deep_link = false
	Global.deep_link_router.deep_link_received.connect(_on_deep_link_received)

	login.set_lobby(self)
	login.show()

	show_dcl_splash_screen()
	var startup_time_ms: int = Time.get_ticks_msec() - Global._startup_time
	print("[Startup] lobby.show_dcl_splash_screen: %dms" % startup_time_ms)

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
		show_dcl_splash_screen()
	elif _skip_lobby:
		show_dcl_splash_screen()
		go_to_explorer.call_deferred()
	elif _skip_lobby_to_menu:
		show_dcl_splash_screen()
		get_tree().change_scene_to_file.call_deferred(
			"res://src/ui/components/organisms/menu/menu.tscn"
		)
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


# Creates the debug-only "reset guest wallet" button (non-prod only) by cloning
# the disposable-account button so it inherits the SecondaryButton styling. It
# starts hidden and is revealed by the same secret logo double-tap. Cloned
# WITHOUT DUPLICATE_SIGNALS so it doesn't inherit the disposable button's
# pressed → create-disposable-account connection.
func _setup_debug_reset_guest_button() -> void:
	# Only meaningful with the rotate flag on — otherwise the anchor is the
	# device-bound native one and deleting user:// wouldn't change the wallet.
	if DclGlobal.is_production() or not Global.DEBUG_GUEST_ROTATE_ANCHOR_ID:
		return
	if button_reset_guest_debug != null:
		return
	var clone: Button = button_enter_as_disposable_account.duplicate(
		Node.DUPLICATE_GROUPS | Node.DUPLICATE_SCRIPTS
	)
	clone.name = "Button_ResetGuestDebug"
	clone.unique_name_in_owner = false
	clone.visible = false
	clone.text = "RESET GUEST WALLET (DEBUG)"
	clone.pressed.connect(_on_button_reset_guest_debug_pressed)
	button_enter_as_disposable_account.get_parent().add_child(clone)
	button_reset_guest_debug = clone


# Wipes the on-disk guest identity then confirms. The next "Play as guest"
# re-derives a brand-new guest wallet from a freshly minted user:// anchor.
# gdlint:ignore = async-function-name
func _on_button_reset_guest_debug_pressed() -> void:
	var removed := _debug_clear_guest_state()
	push_warning("[guest] debug reset: cleared %d guest state file(s)" % removed)
	var modal = await Global.modal_manager._async_create_modal()
	if not modal:
		return
	modal.set_title("Guest wallet reset")
	modal.set_body(
		(
			"Cleared the local guest anchor + session. "
			+ "Tap Play as guest to mint a brand-new guest wallet."
		)
	)
	modal.set_primary_button_text("OK")
	modal.show_icon(Modal.MODAL_ALERT_ICON)
	modal.button_secondary.hide()
	modal.hide_url()
	modal.show()
	await modal.button_primary.pressed
	Global.modal_manager.close_current_modal()


# Deletes the on-disk guest identity (anchor + persisted thirdweb session) and
# clears the cached guest profile so the next "Play as guest" derives a fresh
# wallet instead of reusing the old one. Returns how many files were removed.
func _debug_clear_guest_state() -> int:
	var removed := 0
	for path in DEBUG_GUEST_STATE_FILES:
		if FileAccess.file_exists(path):
			var err := DirAccess.remove_absolute(path)
			if err == OK:
				removed += 1
			else:
				push_error("[guest] debug reset: failed to delete %s (err %d)" % [path, err])
	# The cached guest profile would otherwise be reused by the fresh wallet.
	Global.get_config().guest_profile = {}
	Global.get_config().save_to_settings_file()
	return removed


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
			async_show_avatar_create_screen()
	else:
		ready_for_redirect_by_deep_link = true
		if _should_go_to_explorer_from_deeplink():
			go_to_explorer()
			return


func _on_need_open_url(url: String, _description: String, use_webview: bool) -> void:
	Global.open_url(url, use_webview)


func _on_wallet_connected(address: String, _chain_id: int, is_guest: bool) -> void:
	_accept_eula()
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
	show_auth_home_screen()


func _on_button_continue_pressed():
	Global.metrics.track_click_button("done", current_screen_name, "")
	current_profile = Global.player_identity.get_mutable_profile()
	show_avatar_naming_screen()


# gdlint:ignore = async-function-name
func _on_button_lets_go_pressed():
	Global.metrics.track_click_button("next", current_screen_name, "")
	if dcl_line_edit.line_edit.text.is_empty():
		return

	avatar_preview.hide()
	show_dcl_splash_screen()
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

	show_discover_ftue_screen()


func _on_button_random_name_pressed():
	dcl_line_edit.set_text_value(RandomGeneratorUtil.generate_unique_name())


func _on_button_go_to_sign_in_pressed():
	Global.metrics.track_click_button("sign_in", current_screen_name, "")
	sign_in_title.text = "Sign in to Decentraland"
	is_creating_account = false
	show_auth_home_screen()


# gdlint:ignore = async-function-name
func _on_button_back_pressed():
	Global.metrics.track_click_button("back", current_screen_name, "")
	if current_screen_name == "AVATAR_CUSTOMIZE":
		await _async_confirm_discard_edit()
		return
	match current_screen_name:
		"AVATAR_CREATE":
			show_account_home_screen()
		"AVATAR_NAMING":
			async_show_avatar_create_screen()
		_:
			show_account_home_screen()


# gdlint:ignore = async-function-name
func _async_confirm_discard_edit() -> void:
	var modal = await Global.modal_manager._async_create_modal()
	if not modal:
		return
	modal.set_title("Discard changes?")
	modal.set_body("Your avatar changes won't be saved.")
	modal.set_primary_button_text("DISCARD")
	modal.set_secondary_button_text("CANCEL")
	modal.hide_url()
	modal.hide_icon()
	modal.blocker = true
	modal.show()

	_discard_edit_confirmed = false
	modal.button_primary.pressed.connect(_on_discard_confirmed)
	modal.button_secondary.pressed.connect(_on_discard_cancelled)

	await modal.tree_exited

	if _discard_edit_confirmed:
		Global.player_identity.set_profile(current_profile)
		async_show_avatar_create_screen()


func _on_discard_confirmed() -> void:
	_discard_edit_confirmed = true
	Global.modal_manager.close_current_modal()


func _on_discard_cancelled() -> void:
	Global.modal_manager.close_current_modal()


func _handle_back_action():
	match current_screen_name:
		"ACCOUNT_HOME":
			show_account_home_screen()
		"AUTH_HOME_ANDROID", "AUTH_HOME_IOS", "AUTH_HOME_DESKTOP":
			show_account_home_screen()
		"AUTH_BROWSER_OPEN":
			_on_button_cancel_pressed()
		"AVATAR_CREATE":
			show_account_home_screen()
		"AVATAR_CUSTOMIZE", "AVATAR_NAMING":
			async_show_avatar_create_screen()


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
	async_show_avatar_create_screen()


# Returns the guest anchor used to derive the deterministic thirdweb guest
# wallet (see Global.get_device_anchor_id for the full resolution order). With
# DEBUG_GUEST_ROTATE_ANCHOR_ID on it resolves to a resettable per-install UUID in
# `user://`; with it off it's the device-bound native anchor (SSAID/Keychain).
func _get_device_anchor_id() -> String:
	return Global.get_device_anchor_id()


# gdlint:ignore = async-function-name
func _on_button_play_as_guest_pressed():
	Global.metrics.track_click_button("PLAY_GUEST", "ACCOUNT_HOME", "")
	_accept_eula()
	show_account_home_loading_screen()

	waiting_for_new_wallet = true

	# The login is only "done" once the wallet_connected → profile_changed chain
	# navigates us OFF the loading screen — and that whole chain (request, profile
	# fetch, avatar load) can hang on a flaky network. Awaiting the create-guest
	# promise alone doesn't cover that, so arm a screen-state watchdog instead: if
	# we're still on ACCOUNT_HOME_LOADING after the timeout, bail and offer a retry.
	# A per-attempt token stops a stale watchdog from clobbering a fresh attempt.
	_guest_login_attempt += 1
	var attempt := _guest_login_attempt
	get_tree().create_timer(GUEST_LOGIN_TIMEOUT_SEC).timeout.connect(
		func(): _on_guest_login_watchdog_timeout(attempt)
	)

	var anchor: String = _get_device_anchor_id()
	var guest_promise: Promise = Global.player_identity.async_create_guest_account(anchor)
	var result = await PromiseUtils.async_awaiter(guest_promise)

	# Superseded by a newer attempt, or the watchdog already failed this one.
	if attempt != _guest_login_attempt:
		return

	if result is PromiseError:
		await _fail_guest_login(attempt, result.get_error())
	# On success the profile_changed chain navigates away; the watchdog stays armed
	# and only fires if that never happens.


# Fires GUEST_LOGIN_TIMEOUT_SEC after a guest-login attempt. If we're still stuck
# on the loading screen (navigation never happened), abort and show the retry.
# gdlint:ignore = async-function-name
func _on_guest_login_watchdog_timeout(attempt: int) -> void:
	if attempt != _guest_login_attempt:
		return  # superseded / already handled
	if current_screen_name != "ACCOUNT_HOME_LOADING":
		return  # navigation succeeded — nothing to do
	push_error("Guest login watchdog: stuck on loading screen after %ss" % GUEST_LOGIN_TIMEOUT_SEC)
	await _fail_guest_login(attempt, "Guest login timed out")


# Aborts a stuck/failed guest login: returns to Account Home and shows the retry
# prompt. Bumps the attempt token so neither the watchdog nor the awaited request
# can act on this attempt again. No-op if the attempt was already superseded.
# gdlint:ignore = async-function-name
func _fail_guest_login(attempt: int, reason: String) -> void:
	if attempt != _guest_login_attempt:
		return
	if current_screen_name != "ACCOUNT_HOME_LOADING":
		return
	_guest_login_attempt += 1
	waiting_for_new_wallet = false
	push_error("Guest login failed: " + reason)
	show_account_home_screen()
	await _async_show_guest_login_error()


# gdlint:ignore = async-function-name
func _async_show_guest_login_error() -> void:
	var modal = await Global.modal_manager._async_create_modal()
	if not modal:
		return
	modal.set_title("Something went wrong")
	modal.set_body("We couldn't start your guest session. Please try again.")
	modal.set_primary_button_text("TRY AGAIN")
	modal.show_icon(Modal.MODAL_ALERT_ICON)
	modal.button_secondary.hide()
	modal.hide_url()
	modal.blocker = true
	modal.show()
	await modal.button_primary.pressed
	Global.modal_manager.close_current_modal()


func _set_avatar_preview_centered() -> void:
	avatar_preview.preview_margin_top = 0
	avatar_preview.preview_margin_bottom = 0
	avatar_preview.preview_margin_left = 0
	avatar_preview.preview_margin_right = 0
	avatar_preview.snap_top_to_viewport = false
	avatar_preview.top_node_margin = null
	avatar_preview.bottom_node_margin = null


func _restore_avatar_preview_defaults() -> void:
	avatar_preview.preview_margin_top = _avatar_preview_defaults.get("margin_top", 0)
	avatar_preview.preview_margin_bottom = _avatar_preview_defaults.get("margin_bottom", 0)
	avatar_preview.preview_margin_left = _avatar_preview_defaults.get("margin_left", 0)
	avatar_preview.preview_margin_right = _avatar_preview_defaults.get("margin_right", 0)
	avatar_preview.snap_top_to_viewport = _avatar_preview_defaults.get("snap_top", false)
	avatar_preview.top_node_margin = _avatar_preview_defaults.get("top_node", null)
	avatar_preview.bottom_node_margin = _avatar_preview_defaults.get("bottom_node", null)
	avatar_preview.anchors_preset = _avatar_preview_defaults.get("anchors_preset", 0)
	avatar_preview.size_flags_horizontal = _avatar_preview_defaults.get("size_flags_h", 0)
	avatar_preview.size_flags_vertical = _avatar_preview_defaults.get("size_flags_v", 0)


func _show_avatar_preview():
	avatar_loading.hide()
	avatar_preview.show()
	avatar_preview.avatar.emote_controller.async_play_emote("wave")


func _show_avatar_loading():
	avatar_preview.hide()
	avatar_loading.reparent(avatar_preview.get_parent())
	avatar_loading.anchors_preset = Control.PRESET_FULL_RECT
	avatar_loading.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	avatar_loading.size_flags_vertical = Control.SIZE_EXPAND_FILL
	avatar_loading.add_theme_constant_override("margin_top", avatar_preview.preview_margin_top)
	avatar_loading.add_theme_constant_override(
		"margin_bottom", avatar_preview.preview_margin_bottom
	)
	avatar_loading.add_theme_constant_override("margin_left", avatar_preview.preview_margin_left)
	avatar_loading.add_theme_constant_override("margin_right", avatar_preview.preview_margin_right)
	avatar_loading.show()


func _on_avatar_preview_loaded():
	avatar_loading.hide()
	avatar_preview.show()


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
	button_lets_go.disabled = dcl_line_edit.error
	if dcl_line_edit.error:
		if not avatar_preview.avatar.emote_controller.is_playing() or _playing != "shrug":
			avatar_preview.avatar.emote_controller.async_play_emote("shrug")
			_playing = "shrug"
	else:
		if (
			!button_lets_go.disabled and not avatar_preview.avatar.emote_controller.is_playing()
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
