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
	if Global.deep_link_obj.is_location_defined():
		var realm = Global.deep_link_obj.preview
		if realm.is_empty():
			realm = Global.deep_link_obj.realm
		if realm.is_empty():
			realm = DclUrls.main_realm()
		Global.teleport_to(Global.deep_link_obj.location, realm)
	elif not Global.deep_link_obj.preview.is_empty():
		Global.teleport_to(Vector2i.ZERO, Global.deep_link_obj.preview)
	elif not Global.deep_link_obj.realm.is_empty():
		Global.teleport_to(Vector2i.ZERO, Global.deep_link_obj.realm)


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
