class_name DeepLinkRouter
extends RefCounted

## Centralized deep link processing and path routing.
##
## Handles incoming deep links from iOS signals, Android intents, and
## --fake-deeplink CLI args. Parses the URL, updates Global.deep_link_obj/url,
## and emits the appropriate signal based on the URL path.

signal deep_link_received
signal deep_link_jump
signal deep_link_open_event(event_id: String)
signal deep_link_open_place(place_id: String)


## Parse and store a deep link URL, then emit deep_link_received.
## Called from _notification(FOCUS_IN) and iOS deeplink_received signal.
func process_deep_link(url: String) -> void:
	if url.is_empty():
		return

	# Consume receivedUrl from the iOS plugin so that _notification(FOCUS_IN)
	# doesn't re-read the same URL via get_deeplink_url() on the next resume.
	if DclIosPlugin.is_available():
		DclIosPlugin.get_deeplink_args()

	Global.deep_link_url = url
	Global.deep_link_obj = DclParseDeepLink.parse_decentraland_link(url)
	print("[DEEPLINK] process_deep_link params: ", Global.deep_link_obj.params)

	# Apply rust-log from deeplink params
	var rust_log_value = Global.deep_link_obj.params.get("rust-log", "")
	if not rust_log_value.is_empty():
		print("[DEEPLINK] Found rust-log param: ", rust_log_value)
		DclGlobal.set_rust_log_filter(rust_log_value)

	Global._apply_optimized_content_base_url(Global.deep_link_obj)

	# `skip-gltf` toggle has to be set BEFORE any scene's GLTF_CONTAINER
	# component dirty-set is processed by `update_gltf_container`. The
	# bench runner's `_apply_deeplink_overrides` runs too late — by then
	# the first scene's GLTFs are already instantiated. Apply here, in
	# the deeplink router, which lands before any scene starts loading.
	var skip_gltf_value = Global.deep_link_obj.params.get("skip-gltf", "")
	if not skip_gltf_value.is_empty():
		Global.cli.set_skip_gltf_load(skip_gltf_value.to_lower() in ["true", "1", "yes"])
		print("[DEEPLINK] skip-gltf=", Global.cli.get_skip_gltf_load())

	var kill_sky_value = Global.deep_link_obj.params.get("kill-sky", "")
	if not kill_sky_value.is_empty():
		Global.cli.set_kill_sky(kill_sky_value.to_lower() in ["true", "1", "yes"])
		print("[DEEPLINK] kill-sky=", Global.cli.get_kill_sky())

	# Start/retarget the unified log stream from deeplink params (hot-reconnect).
	var log_stream_value = Global.deep_link_obj.params.get("log-stream", "")
	if not log_stream_value.is_empty():
		print("[DEEPLINK] Found log-stream param: ", log_stream_value)
		DclGlobal.start_log_stream(log_stream_value)

	# Toggle the loopback debug WS server from the deeplink. Lets it be enabled
	# before reaching Settings (e.g. on the login/lobby screens).
	apply_debug_ws_param(Global.deep_link_obj.params.get("debug-ws", ""))

	# Genesis Plaza profiling benchmark (issue #1862). The CLI path spawns the
	# runner from Global._ready, but on mobile the deep link is not parsed by
	# then — spawn here once the deeplink lands and only if no runner exists.
	if Global.deep_link_obj.gp_benchmark and Global.get_node_or_null("GPBenchmarkRunner") == null:
		# Flip bench_mode BEFORE spawning the runner: this fires earlier than
		# DG's deferred _init_dynamic_graphics_manager (07.4xx vs 07.7xx on A54)
		# and before lobby.gd's first-launch HW bench trigger, so both honor it.
		Global.cli.bench_mode = true
		print("[DEEPLINK] bench_mode=true (gp-benchmark deeplink)")
		print("[DEEPLINK] Spawning GP benchmark runner")
		var gp_runner = load("res://src/tools/gp_benchmark_runner.gd").new()
		gp_runner.set_name("GPBenchmarkRunner")
		Global.add_child(gp_runner)

	if Global.deep_link_obj.safe_margin_debug:
		Global.set_safe_margin_debug_enable(true)

	if Global.deep_link_obj.iap_enabled:
		Iap.enable()

	# Trigger avatar impostor benchmark
	var bench_param = Global.deep_link_obj.params.get("benchmark", "")
	if bench_param == "avatar-impostors":
		print("[DEEPLINK] Triggering avatar impostor benchmark")
		if Global.player_identity.get_profile_or_null() == null:
			Global.player_identity.set_default_profile()
		Global.set_meta("avatar_impostor_benchmark_auto_quit", true)
		Global.get_tree().change_scene_to_file.call_deferred(
			"res://src/tools/avatar_impostor_benchmark.tscn"
		)
		_clear_deep_link()
		return

	# Ignore WalletConnect callbacks (decentraland://walletconnect)
	if Global.deep_link_obj.is_walletconnect_callback:
		print("[DEEPLINK] Ignoring WalletConnect callback")
		return

	# Check for environment change — requires restart (sign out back to lobby)
	if Global._check_dclenv_change():
		return

	# Handle signin deep link for mobile auth flow
	if Global.deep_link_obj.is_signin_request():
		_handle_signin_deep_link(Global.deep_link_obj.signin_identity_id)
		_clear_deep_link()
	else:
		deep_link_received.emit.call_deferred()


## Route the current deep link based on its path.
## Called after the explorer or menu is ready to act on the deep link.
func route() -> void:
	# Only process deep links on real mobile devices (not emulation/desktop)
	if not Global.is_mobile() or Global.is_virtual_mobile():
		return

	# Skip if no pending deep link (already consumed or none received)
	if Global.deep_link_url.is_empty():
		return

	# Ignore WalletConnect callbacks
	if Global.deep_link_obj.is_walletconnect_callback:
		_clear_deep_link()
		return

	var path: String = Global.deep_link_obj.path
	# Normalize: strip trailing slashes, treat root as empty
	path = path.rstrip("/")

	match path:
		"/jump", "/open":
			# If location or realm is provided, teleport; otherwise open jump panel
			if (
				Global.deep_link_obj.is_location_defined()
				or not Global.deep_link_obj.realm.is_empty()
				or not Global.deep_link_obj.preview.is_empty()
			):
				_route_teleport()
			else:
				deep_link_jump.emit()
		"/events":
			var event_id: String = Global.deep_link_obj.params.get("id", "")
			if not event_id.is_empty():
				deep_link_open_event.emit(event_id)
			else:
				Global.open_discover.emit()
		"/places":
			var place_id: String = Global.deep_link_obj.params.get("id", "")
			if not place_id.is_empty():
				deep_link_open_place.emit(place_id)
			else:
				Global.open_discover.emit()
		_:
			# "/mobile", "", or any other path -> existing teleport behavior
			_route_teleport()

	_clear_deep_link()


func _route_teleport() -> void:
	var realm = Global.deep_link_obj.preview
	if realm.is_empty():
		realm = Global.deep_link_obj.realm

	# World realm without explicit location → join_world, skip ban pre-check (deferred post-loading)
	if (
		not realm.is_empty()
		and Realm.is_dcl_ens(realm)
		and not Global.deep_link_obj.is_location_defined()
	):
		Global.async_join_world(realm)
		return

	if Global.deep_link_obj.is_location_defined():
		if realm.is_empty():
			realm = DclUrls.main_realm()
		Global.async_teleport_to(Global.deep_link_obj.location, realm)
	elif not realm.is_empty():
		Global.async_teleport_to(Vector2i.ZERO, realm)


## Start/stop the developer debug WS server from a deeplink `debug-ws` param.
## Mirrors the Settings → Developer → "Debug WS Server" toggle, but reachable
## before Settings exists (login/lobby). Developer-only: ignored in production,
## same as the hidden Settings toggle.
##
## Accepted values: empty -> no-op; "0"/"false"/"off"/"stop"/"disable" -> stop;
## a port number (e.g. "9300") -> start on that port; anything else
## ("1"/"true"/"on") -> start on the default port.
func apply_debug_ws_param(value: String) -> void:
	if value.is_empty():
		return
	if Global.is_production():
		print("[DEEPLINK] Ignoring debug-ws param in production build")
		return

	if value.to_lower() in ["0", "false", "off", "stop", "disable"]:
		DebugWs.stop()
		print("[DEEPLINK] Debug WS server stopped")
		return

	var port: int = DebugWs.DEFAULT_PORT
	if value.is_valid_int() and int(value) > 1:
		port = int(value)
	if DebugWs.start(port):
		print("[DEEPLINK] Debug WS server listening on port ", DebugWs.get_port())
	else:
		printerr("[DEEPLINK] Failed to start debug WS server on port ", port)


func _handle_signin_deep_link(identity_id: String) -> void:
	if Global.player_identity.has_pending_mobile_auth():
		Global.player_identity.complete_mobile_connect_account(identity_id)
	else:
		printerr("[DEEPLINK] Received signin deep link but no pending mobile auth")


func _clear_deep_link() -> void:
	# Only clear the URL flag, not deep_link_obj.
	# deep_link_obj is still needed by scene_fetcher (preview mode)
	# and other systems that check deep link parameters.
	Global.deep_link_url = ""
