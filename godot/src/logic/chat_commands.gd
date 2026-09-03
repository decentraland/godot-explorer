class_name ChatCommands
extends RefCounted

## Chat "/command" dispatch and parsing, split out of explorer.gd (which sits
## against the gdlint max-file-lines cap). Scene access (player, viewport,
## loading UI, jump-to) goes through the Explorer node given to the constructor.

var _int_regex := RegEx.create_from_string(r"^-?\d+$")
var _explorer  # Explorer scene root; untyped to avoid preloading explorer.gd


func _init(explorer) -> void:
	_explorer = explorer


func submit_message(message: String) -> void:
	if message.length() == 0:
		return

	var params := message.split(" ")
	var command_str := params[0].to_lower()
	if command_str.begins_with("/"):
		if (command_str == "/go" or command_str == "/goto") and params.size() > 1:
			var arg_string = " ".join(params.slice(1)).strip_edges()
			if _is_coordinate_string(arg_string):
				var dest_vector = _parse_coordinates(arg_string)
				Global.on_chat_message.emit(
					"system",
					tr("CHAT_SYSTEM_TELEPORTED").format({"location": str(dest_vector)}),
					Time.get_unix_time_from_system()
				)
				_explorer._on_control_menu_jump_to(dest_vector)
			elif Realm.is_dcl_ens(arg_string) or not arg_string.contains("."):
				var world_realm = (
					arg_string if arg_string.ends_with(".dcl.eth") else arg_string + ".dcl.eth"
				)
				Global.async_join_world(world_realm)
			else:
				_async_try_change_realm(arg_string, "on_goto_realm")
		elif command_str == "/changerealm" and params.size() > 1:
			var target_realm = params[1]
			if Realm.is_dcl_ens(target_realm):
				Global.async_join_world(target_realm)
			else:
				_async_try_change_realm(target_realm, "on_changerealm")

		elif command_str == "/pos":
			_emit_pos_command_message()
		elif command_str == "/clear":
			Global.realm.async_clear_realm()
		elif command_str == "/reload":
			Global.realm.async_set_realm(Global.realm.get_realm_string())
		elif command_str == "/scenecrash":
			Global.scene_runner.debug_force_crash_current_scene()
		elif command_str == "/godotcrash":
			OS.crash("User crashed on purpose")
		elif command_str == "/instantcrash":
			DclCrashGenerator.static_crash()
		elif command_str == "/delayedcrash":
			_explorer.add_child(DclCrashGenerator.new())
		else:
			Global.on_chat_message.emit(
				"system", tr("CHAT_SYSTEM_UNKNOWN_COMMAND"), Time.get_unix_time_from_system()
			)
	else:
		Global.comms.send_chat(message)
		Global.on_chat_message.emit(
			Global.player_identity.get_address_str(), message, Time.get_unix_time_from_system()
		)


func _is_coordinate_string(text: String) -> bool:
	var cleaned = text.strip_edges().replace("(", "").replace(")", "").replace(" ", "")
	var parts = cleaned.split(",")
	if parts.size() < 2:
		return false
	return _int_regex.search(parts[0]) != null and _int_regex.search(parts[1]) != null


func _parse_coordinates(coord_string: String) -> Vector2i:
	# Remove parentheses if present
	var cleaned = coord_string.strip_edges()
	cleaned = cleaned.replace("(", "").replace(")", "")

	# Remove all spaces
	cleaned = cleaned.replace(" ", "")

	# Split by comma
	var parts = cleaned.split(",")
	if parts.size() >= 2:
		var x_str = parts[0].strip_edges()
		var y_str = parts[1].strip_edges()

		# Validate and parse integers (including negative values)
		if _int_regex.search(x_str) != null and _int_regex.search(y_str) != null:
			return Vector2i(int(x_str), int(y_str))

	return Vector2i(0, 0)


func _async_try_change_realm(realm_string: String, when: String) -> void:
	Global.on_chat_message.emit(
		"system",
		tr("CHAT_SYSTEM_CHANGING_REALM").format({"realm": realm_string}),
		Time.get_unix_time_from_system()
	)
	Global.get_config().last_realm_joined = realm_string
	_explorer.loading_ui.enable_loading_screen(realm_string, when)
	var success = await Global.realm.async_set_realm(realm_string, true)
	if not success:
		_explorer.loading_ui.hide_loading_screen()


func _emit_pos_command_message() -> void:
	# Coordinates: Decentraland uses X right, Y up, Z forward (north). Godot uses X right, Y up, Z backward.
	# So DCL position = (godot.x, godot.y, -godot.z). Parcels are 16m; parcel = (floor(x/16), floor(z/16)).
	var cam = _explorer.get_viewport().get_camera_3d()
	if not cam:
		Global.on_chat_message.emit(
			"system", tr("CHAT_SYSTEM_NO_CAMERA"), Time.get_unix_time_from_system()
		)
		return

	var pos_godot_player: Vector3 = _explorer.player.global_position
	var pos_dcl_player := Vector3(pos_godot_player.x, pos_godot_player.y, -pos_godot_player.z)
	var parcel_player := Vector2i(floori(pos_dcl_player.x / 16.0), floori(pos_dcl_player.z / 16.0))

	var pos_godot_cam: Vector3 = cam.global_position
	var pos_dcl_cam := Vector3(pos_godot_cam.x, pos_godot_cam.y, -pos_godot_cam.z)
	var parcel_cam := Vector2i(floori(pos_dcl_cam.x / 16.0), floori(pos_dcl_cam.z / 16.0))

	# Relative to current parcel (origin at parcel corner, 0-16 m on XZ)
	var rel_parcel_player := Vector3(
		pos_dcl_player.x - parcel_player.x * 16.0,
		pos_dcl_player.y,
		pos_dcl_player.z - parcel_player.y * 16.0
	)
	var rel_parcel_cam := Vector3(
		pos_dcl_cam.x - parcel_cam.x * 16.0, pos_dcl_cam.y, pos_dcl_cam.z - parcel_cam.y * 16.0
	)

	# Relative to current scene base parcel
	var current_scene_id: int = Global.scene_runner.get_current_parcel_scene_id()
	var base_parcel: Vector2i = Global.scene_runner.get_scene_base_parcel(current_scene_id)
	var rel_base_player := Vector3(
		pos_dcl_player.x - base_parcel.x * 16.0,
		pos_dcl_player.y,
		pos_dcl_player.z - base_parcel.y * 16.0
	)
	var rel_base_cam := Vector3(
		pos_dcl_cam.x - base_parcel.x * 16.0, pos_dcl_cam.y, pos_dcl_cam.z - base_parcel.y * 16.0
	)

	# Camera forward in Godot is -basis.z; convert to DCL axis (Z_dcl = -Z_godot)
	var forward_godot: Vector3 = -cam.global_transform.basis.z
	var forward_dcl := Vector3(forward_godot.x, forward_godot.y, -forward_godot.z)
	if forward_dcl.length_squared() > 0.0001:
		forward_dcl = forward_dcl.normalized()

	# Realm: display name and type (main / world / preview)
	var realm_display: String = Global.realm.get_realm_string()
	if realm_display.is_empty():
		realm_display = Global.realm.realm_url
	var realm_type: String
	if Realm.is_genesis_city(Global.realm.realm_url):
		realm_type = "main"
	elif Realm.is_dcl_ens(realm_display) or realm_display.ends_with(".dcl.eth"):
		realm_type = "world"
	elif Realm.is_local_preview(Global.realm.realm_url):
		realm_type = "preview"
	else:
		realm_type = "realm"

	var msg := (
		(
			"[color=#cfc][b]Position (DCL)[/b][/color]\n"
			+ "Realm: %s  [%s]\n"
			+ "Player world: (%.2f, %.2f, %.2f)  Parcel: (%d, %d)\n"
			+ "  rel parcel: (%.2f, %.2f, %.2f)  rel base: (%.2f, %.2f, %.2f)\n"
			+ "Camera world: (%.2f, %.2f, %.2f)  Parcel: (%d, %d)\n"
			+ "  rel parcel: (%.2f, %.2f, %.2f)  rel base: (%.2f, %.2f, %.2f)\n"
			+ "Camera dir (unit): (%.4f, %.4f, %.4f)"
		)
		% [
			realm_display,
			realm_type,
			pos_dcl_player.x,
			pos_dcl_player.y,
			pos_dcl_player.z,
			parcel_player.x,
			parcel_player.y,
			rel_parcel_player.x,
			rel_parcel_player.y,
			rel_parcel_player.z,
			rel_base_player.x,
			rel_base_player.y,
			rel_base_player.z,
			pos_dcl_cam.x,
			pos_dcl_cam.y,
			pos_dcl_cam.z,
			parcel_cam.x,
			parcel_cam.y,
			rel_parcel_cam.x,
			rel_parcel_cam.y,
			rel_parcel_cam.z,
			rel_base_cam.x,
			rel_base_cam.y,
			rel_base_cam.z,
			forward_dcl.x,
			forward_dcl.y,
			forward_dcl.z
		]
	)
	Global.on_chat_message.emit("system", msg, Time.get_unix_time_from_system())
