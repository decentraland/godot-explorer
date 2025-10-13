extends Control

const GOOGLE = preload("res://src/ui/components/auth/images/google.svg")
const APPLE = preload("res://src/ui/components/auth/images/apple.svg")

var lobby: Lobby = null
@onready var button_social_large: Button = $Button_SocialLarge
@onready var button_social: Button = $HBoxContainer_SocialButtons/Button_Social


func _ready():
	if OS.get_name() == "Android":
		button_social.icon = GOOGLE
		button_social_large.icon = GOOGLE
		button_social_large.text = "GOOGLE"
	else:
		button_social.icon = APPLE
		button_social_large.icon = APPLE
		button_social_large.text = "APPLE"


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
