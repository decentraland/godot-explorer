extends Control

const GOOGLE_ICON = preload("res://src/ui/components/auth/images/google.svg")

# TODO: change final project id
# WalletConnect Project ID from https://dashboard.reown.com/
const WALLETCONNECT_PROJECT_ID = "71f57f4190df9de6326bd07de6c40dcc"
const WC_POLLING_TIMEOUT_MS: int = 30000  # 30 second timeout

var lobby: Lobby = null

# WalletConnect native flow state
var _wc_ephemeral_data: Dictionary = {}
var _wc_polling_timer: Timer = null
var _wc_using_native_flow: bool = false
var _wc_plugin = null  # iOS DclWalletConnect instance
var _wc_polling_start_time: int = 0

@onready var h_box_container_more: HBoxContainer = %HBoxContainer_More
@onready var button_google: Button = %Button_Google
@onready var button_apple: Button = %Button_Apple

@onready var texture_rect_google: TextureRect = $Button_Google/TextureRect_Google
@onready var texture_rect_apple: TextureRect = $HBoxContainer_More/Button_Apple/TextureRect_Apple


func _ready():
	texture_rect_apple.hide()
	texture_rect_google.show()

	if Global.is_ios():
		switch_google_with_apple()


func set_lobby(new_lobby: Lobby):
	lobby = new_lobby


func async_login(provider: String = ""):
	# Use mobile auth flow (deep link based) only for ACTUAL mobile platforms (Android/iOS)
	# Desktop uses polling-based flow even when --force-mobile is used for UI testing
	var is_real_mobile = Global.is_android() or Global.is_ios()
	if is_real_mobile:
		Global.player_identity.start_mobile_connect_account(provider)
	else:
		Global.player_identity.try_connect_account()

	lobby.waiting_for_new_wallet = true
	lobby.show_auth_browser_open_screen()


func switch_google_with_apple():
	button_google.reparent(h_box_container_more)
	button_google.text = ""
	button_google.icon = GOOGLE_ICON
	texture_rect_google.hide()
	h_box_container_more.move_child(button_google, 0)

	button_apple.reparent(self)
	self.move_child(button_apple, 0)
	button_apple.text = "APPLE"
	button_apple.icon = null
	texture_rect_apple.show()


# =============================================================================
# NATIVE WALLETCONNECT FLOW (Android and iOS)
# =============================================================================


func _get_native_plugin():
	var ret_plugin = null

	if Global.is_android():
		ret_plugin = Engine.get_singleton("dcl-godot-android")
	elif Global.is_ios():
		# iOS uses DclSwiftLib GDExtension with DclWalletConnect class
		if ClassDB.class_exists("DclWalletConnect"):
			if _wc_plugin == null:
				print("[WC] iOS: Instantiating DclWalletConnect...")
				_wc_plugin = ClassDB.instantiate("DclWalletConnect")
				print("[WC] iOS: Instance = ", _wc_plugin)
			ret_plugin = _wc_plugin

	return ret_plugin


func _try_native_walletconnect() -> bool:
	print("[WC] _try_native_walletconnect called")
	if not Global.is_android() and not Global.is_ios():
		print("[WC] Not Android or iOS, returning false")
		return false

	var plugin = _get_native_plugin()
	print("[WC] Got plugin: ", plugin)
	if plugin == null:
		push_warning("Native WalletConnect plugin not found, falling back to web flow")
		print("[WC] Plugin is null, returning false")
		return false

	# Initialize WalletConnect if needed
	var is_init = plugin.walletConnectIsInitialized()
	print("[WC] walletConnectIsInitialized = ", is_init)
	if not is_init:
		print("[WC] Calling walletConnectInit with project ID...")
		var init_result = plugin.walletConnectInit(WALLETCONNECT_PROJECT_ID)
		print("[WC] walletConnectInit result = ", init_result)
		if not init_result:
			var error = plugin.walletConnectGetError()
			push_error("WalletConnect init failed: " + error)
			print("[WC] Init failed: ", error)
			return false

	# Generate ephemeral keypair and message for signing
	_wc_ephemeral_data = Global.player_identity.generate_ephemeral_for_signing()
	if _wc_ephemeral_data.is_empty():
		push_error("Failed to generate ephemeral data")
		return false

	# Create pairing and get WalletConnect URI
	plugin.walletConnectCreatePairing()

	_wc_using_native_flow = true

	# Start polling for pairing URI, then open wallet
	_start_wc_pairing_polling()
	return true


func _start_wc_pairing_polling() -> void:
	_stop_wc_polling()
	_wc_polling_start_time = Time.get_ticks_msec()
	_wc_polling_timer = Timer.new()
	_wc_polling_timer.wait_time = 0.2
	_wc_polling_timer.timeout.connect(_async_poll_wc_pairing_uri)
	add_child(_wc_polling_timer)
	_wc_polling_timer.start()


func _async_poll_wc_pairing_uri() -> void:
	var plugin = _get_native_plugin()
	if plugin == null:
		_cleanup_wc_flow("Native plugin lost")
		return

	# Check for timeout
	var elapsed = Time.get_ticks_msec() - _wc_polling_start_time
	if elapsed > WC_POLLING_TIMEOUT_MS:
		_cleanup_wc_flow("Timeout waiting for pairing URI")
		return

	var uri = plugin.walletConnectGetPairingUri()
	if uri.is_empty():
		# Check for error
		var state = plugin.walletConnectGetConnectionState()
		if state == "error":
			_cleanup_wc_flow("Pairing error: " + plugin.walletConnectGetError())
		# Otherwise keep polling
		return

	_stop_wc_polling()

	# Small delay to let the relay fully register the pairing before opening wallet
	print("[WC] URI received, waiting for relay to stabilize...")
	await get_tree().create_timer(0.5).timeout

	# Open wallet app with the URI (empty string = let user choose wallet)
	if not plugin.walletConnectOpenWallet(""):
		# If no wallet app handles WC URIs, try specific wallets
		var wallets_to_try = ["io.metamask", "com.trustwallet.app", "me.rainbow"]
		var opened = false
		for wallet_pkg in wallets_to_try:
			if plugin.walletConnectOpenWallet(wallet_pkg):
				opened = true
				break

		if not opened:
			_cleanup_wc_flow("No wallet app found. Please install MetaMask or Trust Wallet.")
			return

	# Start polling for connection
	_start_wc_connection_polling()


func _start_wc_connection_polling() -> void:
	_stop_wc_polling()
	_wc_polling_timer = Timer.new()
	_wc_polling_timer.wait_time = 0.5
	_wc_polling_timer.timeout.connect(_poll_wc_connection)
	add_child(_wc_polling_timer)
	_wc_polling_timer.start()


func _poll_wc_connection() -> void:
	var plugin = _get_native_plugin()
	if plugin == null:
		_cleanup_wc_flow("Native plugin lost")
		return

	var state = plugin.walletConnectGetConnectionState()

	match state:
		"connected":
			_stop_wc_polling()
			_request_wc_signature()
		"error":
			var error = plugin.walletConnectGetError()
			_cleanup_wc_flow("Connection error: " + error)
		"disconnected":
			# User might have closed the modal without connecting
			if not _wc_using_native_flow:
				_cleanup_wc_flow("Connection cancelled")
		# "connecting" - keep polling


func _request_wc_signature() -> void:
	var plugin = _get_native_plugin()
	if plugin == null:
		_cleanup_wc_flow("Native plugin lost")
		return

	var message = _wc_ephemeral_data.get("message", "")
	if message.is_empty():
		_cleanup_wc_flow("No message to sign")
		return

	if not plugin.walletConnectRequestSign(message):
		var error = plugin.walletConnectGetError()
		_cleanup_wc_flow("Sign request failed: " + error)
		return

	# Start polling for signature
	_start_wc_sign_polling()


func _start_wc_sign_polling() -> void:
	_stop_wc_polling()
	_wc_polling_timer = Timer.new()
	_wc_polling_timer.wait_time = 0.5
	_wc_polling_timer.timeout.connect(_poll_wc_signature)
	add_child(_wc_polling_timer)
	_wc_polling_timer.start()


func _poll_wc_signature() -> void:
	var plugin = _get_native_plugin()
	if plugin == null:
		_cleanup_wc_flow("Native plugin lost")
		return

	var state = plugin.walletConnectGetSignState()

	match state:
		"success":
			_stop_wc_polling()
			var signature = plugin.walletConnectGetSignResult()
			_complete_wc_auth(signature)
		"error":
			var error = plugin.walletConnectGetError()
			_cleanup_wc_flow("Sign error: " + error)
		# "pending", "idle" - keep polling


func _complete_wc_auth(signature: String) -> void:
	var plugin = _get_native_plugin()
	if plugin == null:
		_cleanup_wc_flow("Native plugin lost")
		return

	var signer_address = plugin.walletConnectGetAddress()
	var ephemeral_private_key = _wc_ephemeral_data.get("ephemeral_private_key", PackedByteArray())
	var expiration_timestamp = _wc_ephemeral_data.get("expiration_timestamp", 0)

	var success = Global.player_identity.try_set_walletconnect_auth(
		signer_address, signature, ephemeral_private_key, expiration_timestamp
	)

	if success:
		print("WalletConnect native auth successful!")
		# Note: Do NOT set lobby.waiting_for_new_wallet = false here!
		# The lobby will set it to false when profile_changed signal is received.
		# This allows the UI to properly transition after profile is fetched.
	else:
		_cleanup_wc_flow("Failed to complete authentication")

	# Clean up
	_wc_ephemeral_data = {}
	plugin.walletConnectResetSignState()
	_wc_using_native_flow = false


func _stop_wc_polling() -> void:
	if _wc_polling_timer != null:
		_wc_polling_timer.stop()
		_wc_polling_timer.queue_free()
		_wc_polling_timer = null


func _cleanup_wc_flow(error_message: String) -> void:
	push_error("WalletConnect flow failed: " + error_message)
	_stop_wc_polling()
	_wc_ephemeral_data = {}
	_wc_using_native_flow = false
	lobby.waiting_for_new_wallet = false

	# Optionally disconnect to clean up session
	var plugin = _get_native_plugin()
	if plugin != null:
		plugin.walletConnectResetSignState()


# =============================================================================
# BUTTON HANDLERS
# =============================================================================


func _on_button_wallet_connect_pressed() -> void:
	# Try native WalletConnect flow on Android/iOS
	var native_result = _try_native_walletconnect()

	if native_result == true:
		lobby.waiting_for_new_wallet = true
		lobby.show_auth_browser_open_screen("Opening Wallet...")
		Global.metrics.track_click_button("wallet_connect_native", lobby.current_screen_name, "")
		return

	# On iOS, don't fall back to web - it doesn't work properly
	# Show error message if native WalletConnect failed to initialize
	if Global.is_ios():
		lobby._show_auth_error(
			"WalletConnect failed to initialize. Please try again or use another sign-in method."
		)
		Global.metrics.track_click_button("wallet_connect_ios_error", lobby.current_screen_name, "")
		return

	# On other platforms (desktop), fall back to web-based flow
	async_login("")
	Global.metrics.track_click_button("wallet_connect", lobby.current_screen_name, "")


func _on_button_google_pressed() -> void:
	async_login("google")
	Global.metrics.track_click_button("google", lobby.current_screen_name, "")


func _on_button_discord_pressed() -> void:
	async_login("discord")
	Global.metrics.track_click_button("discord", lobby.current_screen_name, "")


func _on_button_x_pressed() -> void:
	async_login("x")
	Global.metrics.track_click_button("x", lobby.current_screen_name, "")


func _on_button_apple_pressed() -> void:
	async_login("apple")
	Global.metrics.track_click_button("apple", lobby.current_screen_name, "")
