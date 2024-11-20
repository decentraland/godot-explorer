extends Control

var lobby: Lobby = null


func is_platform_supported():
	return OS.get_name() == "Android" and not Global.is_xr()


func _ready():
	if !is_platform_supported():
		queue_free()
		return


func set_lobby(new_lobby: Lobby):
	lobby = new_lobby

func async_login(social: bool):
	Global.player_identity.try_connect_account("androidSocial" if social else "androidWeb3")
	lobby.container_sign_in_step1.hide()
	lobby.container_sign_in_step2.show()
	lobby.waiting_for_new_wallet = true


func _on_button_social_pressed() -> void:
	async_login(true)


func _on_button_wallet_connect_pressed() -> void:
	async_login(false)
