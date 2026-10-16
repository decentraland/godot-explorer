class_name Explorer
extends Node
## The world the engine runs: the player, the scene UI and the on-screen controls. Every screen
## around it is the shell's.

## Where a profile with no realm history boots. A dev realm, not genesis: this only
## happens on a fresh install that has never joined anywhere.
const FALLBACK_BOOT_REALM := "https://sdk-team-cdn.decentraland.org/ipfs/goerli-plaza-main-latest"

var player: Node3D = null
var parcel_position: Vector2i
var parcel_position_real: Vector2
var dirty_save_position: bool = false
var disable_move_to = false
var virtual_joystick_orig_position: Vector2i
## The shell's chat is focused: the movement controls and the emote wheel stay out of its way.
var chat_open := false

var _chat_commands := ChatCommands.new(self)
var _last_parcel_position: Vector2i = Vector2i.MAX
var _avatar_under_crosshair: DclAvatar = null
var _last_outlined_avatar: DclAvatar = null
var _last_outlined_entity: Node3D = null
var _mobile_controls_hidden_for_hide_ui: bool = false
# Applies scene-driven (PBTouchScreenControls) joystick/crosshair hiding; see the class doc.
var _sdk_touch_controls: SdkTouchControlsApplier = null

@onready var ui_root: Control = %UI
@onready var ui_safe_area: Control = %SceneUIContainer
@onready var label_crosshair = %Label_Crosshair
@onready var control_pointer_tooltip = %Control_PointerTooltip
@onready var mobile_ui = %MobileUI
@onready var mobile_camera_input: Control = %MobileCameraInput
@onready var safe_area_hud: MarginContainer = %SafeAreaHud
@onready var virtual_joystick: VirtualJoystick = %VirtualJoystick_Left
@onready var emote_wheel = %EmoteWheel
@onready var world: Node3D = %world
@onready var timer_broadcast_position: PositionBroadcaster = %Timer_BroadcastPosition
@onready var joypad: Control = %Joypad
@onready var loading: LoadingSession = %LoadingSession


func _process(_dt):
	_sdk_touch_controls.apply(_mobile_controls_hidden_for_hide_ui)

	parcel_position_real = Vector2(player.position.x * 0.0625, -player.position.z * 0.0625)

	parcel_position = Vector2i(floori(parcel_position_real.x), floori(parcel_position_real.y))
	if _last_parcel_position != parcel_position:
		Global.scene_fetcher.update_position(parcel_position, false)
		_last_parcel_position = parcel_position
		Global.engine.set_setting("last_parcel_x", parcel_position.x, false)
		Global.engine.set_setting("last_parcel_y", parcel_position.y, false)
		dirty_save_position = true
		Global.change_parcel.emit(parcel_position)
		Global.metrics.update_position("%d,%d" % [parcel_position.x, parcel_position.y])


func get_params_from_cmd():
	var realm_string = Global.cli.realm if not Global.cli.realm.is_empty() else null
	var location_vector = Global.cli.get_location_vector()
	if location_vector == Vector2i.MAX:
		location_vector = null

	# Preview deeplink takes priority - use it as the realm for hot reload development
	if not Global.deep_link_obj.preview.is_empty() and realm_string == null:
		realm_string = Global.deep_link_obj.preview

	if not Global.deep_link_obj.realm.is_empty() and realm_string == null:
		realm_string = Global.deep_link_obj.realm

	if Global.deep_link_obj.is_location_defined() and location_vector == null:
		location_vector = Global.deep_link_obj.location
		if realm_string == null:
			realm_string = DclUrls.main_realm()

	return [realm_string, location_vector]


func _ready():
	# Out of the lobby — restore the relaxed 10s flush cadence (the lobby drops it to 2s).
	Global.metrics.set_flush_interval(10.0)

	GraphicSettings.apply_full_processor_mode()

	DclGlobal.set_orientation_landscape()
	UiSounds.install_audio_recursive(self)
	Global.music_player.stop()

	Global.orientation_changed.connect(_on_orientation_changed)

	player = load("res://src/logic/player/player.tscn").instantiate()

	player.set_name("Player")
	world.add_child(player)

	timer_broadcast_position.player_node = player
	emote_wheel.avatar_node = player.avatar
	Global.engine.attach_world(self)

	# The episode is opened by the navigation below, which knows the realm this boot is
	# actually heading for -- last_realm_joined is the wrong one for a deeplink.
	loading.begin(Global.engine.setting("last_realm_joined"), "on_explorer_ready", false)
	var cmd_params = get_params_from_cmd()
	var cmd_realm = cmd_params[0]
	var cmd_location = cmd_params[1]

	# Scene Inspector file output: --scene-inspector-file, or ?scene-inspector-file=true off prod
	var scene_inspector_file: bool = (
		(Global.deep_link_obj.scene_inspector_file and not DclGlobal.is_production())
		or Global.cli.scene_inspector_file
	)
	if scene_inspector_file:
		Global.scene_inspector_dispatcher.set_file_logging(true)

	# Clear deep link after initial setup to prevent re-teleporting on first app resume
	Global.deep_link_router.clear_deep_link()

	virtual_joystick.mouse_filter = Control.MOUSE_FILTER_IGNORE
	virtual_joystick_orig_position = virtual_joystick.get_position()
	_sdk_touch_controls = SdkTouchControlsApplier.create(
		virtual_joystick, label_crosshair, player, control_pointer_tooltip
	)

	if Global.is_mobile():
		mobile_ui.show()
		label_crosshair.show()
		reset_cursor_position()
		_update_virtual_controls_visibility()
	else:
		# Desktop is development-only in this build: show the on-screen controls so the
		# native joystick/joypad (and the TouchscreenInputControls / UiInputBinding features)
		# stay visible and debuggable. Keep the desktop crosshair/cursor behaviour as-is.
		mobile_ui.show()
		_update_virtual_controls_visibility()

	control_pointer_tooltip.hide()
	var start_parcel_position := Vector2i(
		Global.engine.setting("last_parcel_x"), Global.engine.setting("last_parcel_y")
	)
	if cmd_location != null:
		start_parcel_position = cmd_location

	player.position = (
		16 * Vector3(start_parcel_position.x, 0.1, -start_parcel_position.y)
		+ Vector3(8.0, 0.0, -8.0)
	)
	player.look_at(16 * Vector3(start_parcel_position.x + 1, 0, -(start_parcel_position.y + 1)))

	Global.player_camera_node = player.camera
	Global.player_camera_node.far = Global.engine.setting("view_distance")
	Global.scene_runner.player_avatar_node = player.avatar
	Global.scene_runner.player_body_node = player
	Global.scene_runner.pointer_tooltip_changed.connect(self._on_pointer_tooltip_changed)
	# Recreate base_ui before use: the previous instance is freed when the Explorer
	# scene is torn down (logout/change_scene_to_file), leaving a dangling reference.
	Global.scene_runner.recreate_base_ui()
	ui_safe_area.add_child(Global.scene_runner.base_ui)
	ui_safe_area.move_child(Global.scene_runner.base_ui, 0)

	ui_safe_area.resized.connect(self._push_scene_interactable_area)
	get_window().size_changed.connect(self._push_scene_interactable_area)
	_push_scene_interactable_area.call_deferred()

	Global.scene_fetcher.update_position(start_parcel_position, true)

	if cmd_realm != null:
		if DclRealmRules.is_dcl_ens(cmd_realm) and Global.deep_link_obj.preview.is_empty():
			DclNavigator.join_world(cmd_realm)
		else:
			DclNavigator.go_restore(cmd_realm, start_parcel_position, "on_explorer_ready")
			if not Global.deep_link_obj.preview.is_empty():
				Global.scene_fetcher.set_preview_url(cmd_realm)
	else:
		var boot_realm: String = Global.engine.setting("last_realm_joined")
		if boot_realm.is_empty():
			boot_realm = FALLBACK_BOOT_REALM
		DclNavigator.go_restore(boot_realm, start_parcel_position, "on_explorer_ready")
	Global.scene_runner.process_mode = Node.PROCESS_MODE_INHERIT

	Global.player_identity.logout.connect(self._on_player_logout)
	Global.player_identity.profile_changed.connect(Global.avatars.update_primary_player_profile)

	var profile = Global.player_identity.get_profile_or_null()
	if profile != null:
		Global.player_identity.profile_changed.emit(profile)

	Global.dcl_tokio_rpc.need_open_url.connect(self._on_need_open_url)
	Global.scene_runner.set_pause(false)

	if Global.testing_scene_mode:
		Global.player_identity.create_disposable_account()

	Global.metrics.update_identity(
		Global.player_identity.get_address_str(), Global.player_identity.is_guest
	)

	ui_root.grab_focus.call_deferred()

	if OS.get_cmdline_args().has("--scene-renderer"):
		var scene_renderer_orchestor = (
			load("res://src/tool/scene_renderer/scene_orchestor.tscn").instantiate()
		)
		add_child(scene_renderer_orchestor)

	add_child(DclGlobalCameraController.new())


func _on_need_open_url(url: String, _description: String, _use_webkit: bool) -> void:
	if not Global.player_identity.get_address_str().is_empty():
		DclGlobal.open_url(url)


## The HUD safe rectangle (in SceneUIContainer/canvas coords): device safe area + the
## symmetric 108/32/108/45 floor from SafeAreaHud, WITHOUT the chat column. This is the
## "screen inset" area — safe from device insets; the chat may still overlap it.
func get_safe_area() -> Rect2:
	var canvas: Vector2 = ui_safe_area.size
	var m_left: float = safe_area_hud.get_theme_constant("margin_left")
	var m_right: float = safe_area_hud.get_theme_constant("margin_right")
	var m_top: float = safe_area_hud.get_theme_constant("margin_top")
	var m_bottom: float = safe_area_hud.get_theme_constant("margin_bottom")
	return Rect2(
		m_left,
		m_top,
		maxf(canvas.x - m_left - m_right, 0.0),
		maxf(canvas.y - m_top - m_bottom, 0.0)
	)


## Where scenes may draw interactive UI without being covered by the client UI: the safe
## rectangle minus the left chat column. The joypad (bottom-right) may still overlap it.
func get_interactable_area() -> Rect2:
	var safe: Rect2 = get_safe_area()
	var chat: float = Global.chat_notifications_width
	return Rect2(
		safe.position.x + chat, safe.position.y, maxf(safe.size.x - chat, 0.0), safe.size.y
	)


## Pushes the safe-area rect (in canvas/logical pixels) to the scene runner so scenes get a
## correct UiCanvasInformation.interactable_area on every resize, including the virtual margins.
func _push_scene_interactable_area() -> void:
	if not is_instance_valid(ui_safe_area) or not is_instance_valid(safe_area_hud):
		return
	var area: Rect2 = get_interactable_area()
	if is_instance_valid(Global.scene_runner) and area.size.x > 0 and area.size.y > 0:
		Global.scene_runner.set_interactable_area(Rect2i(area))
		Global.scene_runner.set_safe_area(Rect2i(get_safe_area()))


func _on_player_logout():
	# Funnel any logout signal (e.g. session expiry) into the single canonical
	# teardown instead of quitting the app. DclGlobal.sign_out() is re-entrancy
	# guarded, so this is safe even when sign_out() is what emitted the signal.
	DclGlobal.sign_out()


## Sever this Explorer from every persistent (autoload / window / Rust singleton)
## emitter while the node is still in the tree. Called by DclGlobal.sign_out() and the world
## exit BEFORE they kill scenes / clear the realm, so none of those re-emit into this
## about-to-be-freed Explorer. Godot severs the connections whose receiver is freed; these
## are the ones that can fire synchronously during the teardown, and Global -> Global ones.
func prepare_for_logout() -> void:
	_safe_disconnect(Global.scene_runner.pointer_tooltip_changed, _on_pointer_tooltip_changed)
	_safe_disconnect(Global.orientation_changed, _on_orientation_changed)
	_safe_disconnect(Global.player_identity.logout, _on_player_logout)
	if Global.avatars != null:
		var profile_changed: Signal = Global.player_identity.profile_changed
		_safe_disconnect(profile_changed, Global.avatars.update_primary_player_profile)


func _safe_disconnect(sig: Signal, callable: Callable) -> void:
	if sig.is_connected(callable):
		sig.disconnect(callable)


func _on_pointer_tooltip_changed():
	change_tooltips.call_deferred()


func change_tooltips():
	var tooltip_data = Global.scene_runner.pointer_tooltips.duplicate()

	_avatar_under_crosshair = player.get_avatar_under_crosshair()
	if _avatar_under_crosshair != _last_outlined_avatar:
		player.outline_system.set_outlined_avatar(_avatar_under_crosshair)
		_last_outlined_avatar = _avatar_under_crosshair

	# Handle the highlight (outline) for scene objects with show_highlight=true
	var highlighted_entity: Node3D = Global.scene_runner.highlighted_entity
	if not is_instance_valid(highlighted_entity):
		highlighted_entity = null
	if highlighted_entity != _last_outlined_entity:
		player.outline_system.set_outlined_entity(highlighted_entity)
		_last_outlined_entity = highlighted_entity

	if not tooltip_data.is_empty():
		control_pointer_tooltip.set_pointer_data(tooltip_data)
		control_pointer_tooltip.show()
	else:
		control_pointer_tooltip.hide()


func _unhandled_input(event):
	if not Global.is_mobile():
		if event is InputEventMouseButton and event.pressed and ui_root.has_focus():
			if Input.mouse_mode != Input.MOUSE_MODE_CAPTURED:
				capture_mouse()

	if event is InputEventKey and ui_root.has_focus():
		if event.pressed and event.keycode == KEY_ESCAPE:
			if Input.mouse_mode == Input.MOUSE_MODE_CAPTURED:
				release_mouse()


func _on_virtual_joystick_right_stick_position(stick_position: Vector2):
	player.stick_position = stick_position


func _on_virtual_joystick_right_is_hold(hold: bool):
	player.stick_holded = hold


func _on_touch_screen_button_pressed():
	Input.action_press("ia_jump")


func _on_touch_screen_button_released():
	Input.action_release("ia_jump")


func submit_chat_message(message: String):
	_chat_commands.submit_message(message)


## Moves the player to a specific position
##
## @param position: The 3D position to move the player to
## @param skip_loading: When true, skips the loading session. This is used when teleporting
##                      inside a scene to avoid loading again an already-loaded area.
func move_to(position: Vector3, skip_loading: bool, check_stuck: bool = true):
	if disable_move_to:
		return

	# Set grace period on avatar's emote controller to prevent emote cancellation during teleport
	if player.avatar:
		player.avatar.set_teleport_grace()

	player.move_to(position, check_stuck)
	# Announce the instant reposition to the Pulse transport so remote peers
	# snap to it instead of interpolating across the jump (no-op when inactive).
	Global.comms.notify_player_teleported(position)
	var cur_parcel_position = Vector2i(
		floor(player.position.x * 0.0625), -floor(player.position.z * 0.0625)
	)
	if not skip_loading:
		if not Global.scene_fetcher.is_scene_loaded(cur_parcel_position.x, cur_parcel_position.y):
			if not loading.is_running():
				loading.begin("", "on_moveto")


## Places the player on a parcel of the realm already loaded, and records it as where
## they are. Changing realm on the way is a navigation: Navigator resolves that first and
## then calls this, so the move never happens toward a place that turned out not to exist.
func teleport_to(parcel: Vector2i) -> void:
	move_to(Vector3i(parcel.x * 16 + 8, 3, -parcel.y * 16 - 8), false)
	Global.scene_fetcher.update_position(parcel, true)
	dirty_save_position = true


func player_look_at(look_at_position: Vector3):
	player.avatar_look_at(look_at_position)


func camera_look_at(look_at_position: Vector3):
	player.camera_look_at(look_at_position)


func avatar_look_at_independent(look_at_position: Vector3):
	player.set_avatar_rotation_independent(look_at_position)


func capture_mouse():
	if DisplayServer.has_feature(DisplayServer.FEATURE_MOUSE):
		Input.set_mouse_mode(Input.MOUSE_MODE_CAPTURED)
	if label_crosshair and ui_root:
		if not Global.touch_controls_hide_crosshair:  # respect scene-driven crosshair hide
			label_crosshair.show()
		DclGlobal.explorer_grab_focus()


func release_mouse():
	if DisplayServer.has_feature(DisplayServer.FEATURE_MOUSE):
		Input.set_mouse_mode(Input.MOUSE_MODE_VISIBLE)
	if not Global.is_mobile():
		if label_crosshair:
			label_crosshair.hide()


func set_visible_ui(value: bool, _use_hud_mode: bool = false):
	ui_root.visible = value


func apply_mobile_controls_hide_ui(hidden: bool) -> void:
	_mobile_controls_hidden_for_hide_ui = hidden
	if hidden:
		joypad.hide()
		# Keep the joystick functional (walking) but fully transparent. A hidden Control
		# stops receiving input, so show() it and only drop its alpha.
		virtual_joystick.show()
		virtual_joystick.modulate.a = 0.0
	else:
		joypad.show()
		virtual_joystick.modulate.a = 1.0


func _show_joypad() -> void:
	if _mobile_controls_hidden_for_hide_ui:
		return
	joypad.show()


func _hide_movement_controls() -> void:
	joypad.hide()
	virtual_joystick.hide()


func _show_movement_controls() -> void:
	_show_joypad()
	virtual_joystick.show()


func _update_virtual_controls_visibility() -> void:
	if _mobile_controls_hidden_for_hide_ui:
		joypad.hide()
		# Transparent but functional, so walking keeps working while the UI is hidden.
		virtual_joystick.show()
		virtual_joystick.modulate.a = 0.0
		return
	_show_joypad()
	virtual_joystick.show()


## The shell's chat takes the screen: out of its way go the emote wheel and the movement controls.
func set_chat_focus(open: bool) -> void:
	chat_open = open
	if DclGlobal.is_orientation_portrait() or _mobile_controls_hidden_for_hide_ui:
		return
	if open:
		emote_wheel.close()
		emote_wheel.hide()
		_hide_movement_controls()
	else:
		emote_wheel.show()
		_show_movement_controls()


func set_cursor_position(position: Vector2):
	if Global.scene_runner.raycast_use_cursor_position:
		var crosshair_position = position - (label_crosshair.size / 2) - Vector2(0, 1)
		label_crosshair.set_global_position(crosshair_position)
		control_pointer_tooltip.set_global_cursor_position(position)
		Global.scene_runner.set_cursor_position(position)


func reset_cursor_position():
	# Position crosshair at center of screen
	var viewport_size = get_tree().root.get_viewport().get_visible_rect()
	var center_position = viewport_size.size * 0.5
	var crosshair_position = center_position - (label_crosshair.size / 2) - Vector2(0, 1)
	label_crosshair.set_global_position(crosshair_position)
	control_pointer_tooltip.set_global_cursor_position(center_position)


func _on_timer_save_position_timeout():
	if dirty_save_position:
		dirty_save_position = false
		Global.engine.save_settings()


func _on_orientation_changed(is_portrait: bool) -> void:
	if is_portrait:
		mobile_ui.hide()
		virtual_joystick.hide()
		emote_wheel.hide()
		_set_scene_ui_visible(false)
	else:
		mobile_ui.show()
		_update_virtual_controls_visibility()
		emote_wheel.show()
		_set_scene_ui_visible(true)


func _set_scene_ui_visible(is_visible: bool) -> void:
	var base_ui = Global.scene_runner.base_ui
	if is_instance_valid(base_ui):
		base_ui.visible = is_visible


func _on_emote_wheel_emote_wheel_closed() -> void:
	if not DclGlobal.is_orientation_portrait() and not chat_open:
		virtual_joystick.show()


func _on_emote_wheel_emote_wheel_opened() -> void:
	virtual_joystick.hide()
