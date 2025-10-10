extends Control

var lobby: Lobby = null


func _ready():
	pass


func set_lobby(new_lobby: Lobby):
	lobby = new_lobby


func async_login(social: bool):
	#TODO: Use Global.is_android() and connect directly with the selected provider
	if OS.get_name() == "Android":
		Global.player_identity.try_connect_account("androidSocial" if social else "androidWeb3")
	else:
		Global.player_identity.try_connect_account("")
	lobby.container_sign_in_step1.hide()
	lobby.container_sign_in_step2.show()
	lobby.waiting_for_new_wallet = true


func _on_button_social_pressed() -> void:
	async_login(true)


func _on_button_wallet_connect_pressed() -> void:
	async_login(false)
