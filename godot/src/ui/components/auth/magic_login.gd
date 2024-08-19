extends Control

var lobby: Lobby = null


func is_platform_supported():
	return OS.get_name() == "Android"


func _ready():
	if !is_platform_supported():
		queue_free()
		return

	Global.magic_link.setup(
		"pk_live_212568025B158355", "org.decentraland.godotexplorer://callback", "ethereum"
	)

	Global.magic_link.message_signed.connect(self._on_magic_message_signed)
	Global.dcl_tokio_rpc.magic_sign.connect(self._on_request_magic_sign)


func set_lobby(lobby: Lobby):
	self.lobby = lobby


func _on_magic_message_signed(signature: String):
	var address = Global.magic_link.get_public_address()
	Global.dcl_tokio_rpc.magic_signed_message(address, signature)


func _on_request_magic_sign(message: String):
	Global.magic_link.sign(message)


func async_magic_login(oauth_provider: String):
	lobby.container_sign_in_step1.hide()
	lobby.container_sign_in_step2.show()
	lobby.waiting_for_new_wallet = true

	Global.magic_link.login_social(oauth_provider)
	await Global.magic_link.connected
	Global.player_identity.try_connect_account_with_magic()


func _on_button_google_pressed():
	async_magic_login("google")


func _on_button_discord_pressed():
	async_magic_login("discord")


func _on_button_apple_pressed():
	async_magic_login("apple")


func _on_button_x_pressed():
	async_magic_login("twitter")
