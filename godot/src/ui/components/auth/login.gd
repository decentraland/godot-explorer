extends Control

const GOOGLE_ICON = preload("res://src/ui/components/auth/images/google.svg")

var lobby: Lobby = null

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


func async_login(social: bool):
	var target_config_id := ""
	if Global.is_android():
		target_config_id = "androidSocial" if social else "androidWeb3"
	elif Global.is_ios():
		target_config_id = "ios"

	# Use mobile auth flow (deep link based) for mobile platforms
	# Desktop continues to use polling-based flow
	if Global.is_mobile():
		Global.player_identity.start_mobile_connect_account(target_config_id)
	else:
		Global.player_identity.try_connect_account(target_config_id)

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


func _on_button_wallet_connect_pressed() -> void:
	async_login(false)
	Global.metrics.track_click_button("wallet_connect", lobby.current_screen_name, "")


func _on_button_google_pressed() -> void:
	async_login(true)
	Global.metrics.track_click_button("google", lobby.current_screen_name, "")


func _on_button_discord_pressed() -> void:
	async_login(true)
	Global.metrics.track_click_button("discord", lobby.current_screen_name, "")


func _on_button_x_pressed() -> void:
	async_login(true)
	Global.metrics.track_click_button("x", lobby.current_screen_name, "")


func _on_button_apple_pressed() -> void:
	async_login(true)
	Global.metrics.track_click_button("apple", lobby.current_screen_name, "")
