class_name MagicLinkPluginInterface

signal connected(address: String)
signal logout

const PLUGIN_NAME = "GodotAndroidPluginMagicLink"

var wallet_connected: bool = false
var public_address: String = ""

static var _plugin_singleton = null


func get_singleton():
	if _plugin_singleton != null:
		return _plugin_singleton

	if Engine.has_singleton(PLUGIN_NAME):
		_plugin_singleton = Engine.get_singleton(PLUGIN_NAME)
		return _plugin_singleton

	printerr("Initialization error: unable to access the java logic")
	return null


func _on_connected(address: String):
	public_address = address
	wallet_connected = true

	connected.emit(address)


func _on_logout():
	public_address = ""
	wallet_connected = false

	logout.emit()


func setup(magic_key: String, callback_url: String, network: String = "ethereum"):
	var singleton = get_singleton()
	if singleton == null:
		printerr("Initialization error")
		return

	singleton.setup(magic_key, callback_url, network)
	singleton.connected.connect(self._on_connected)
	singleton.on_logout.connect(self._on_logout)


func async_check_connection() -> bool:
	var singleton = get_singleton()
	if singleton == null:
		printerr("Initialization error")
		return false

	singleton.checkConnection()

	return await singleton.connection_state == "true"


func async_login_email(email: String):
	var singleton = get_singleton()
	if singleton == null:
		printerr("Initialization error")
		return

	singleton.loginEmailOTP(email)
	await singleton.connected


func async_login_social(oauth_provider: String):
	var singleton = get_singleton()
	if singleton == null:
		printerr("Initialization error")
		return

	singleton.loginSocial(oauth_provider)
	await singleton.connected


func async_logout():
	var singleton = get_singleton()
	if singleton == null:
		printerr("Initialization error")
		return

	singleton.logout()
	await singleton.on_logout


func open_wallet():
	if wallet_connected != true:
		printerr("Please, check if you're connected first...")
		return

	var singleton = get_singleton()
	if singleton == null:
		printerr("Initialization error")
		return

	singleton.openWallet()


func async_sign(message: String) -> String:
	if wallet_connected != true:
		printerr("Please, check if you're connected first...")
		return ""
	var singleton = get_singleton()
	if singleton == null:
		printerr("Initialization error")
		return ""

	singleton.sign(message)
	return await singleton.signed_message
