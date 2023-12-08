extends Control

@onready var panel_main = $Panel_Main

@onready var v_box_container_connect = $Panel_Main/VBoxContainer_Connect
@onready var v_box_container_waiting = $Panel_Main/VBoxContainer_Waiting
@onready var v_box_container_guest_confirm = $Panel_Main/VBoxContainer_GuestConfirm

@onready var label_waiting = $Panel_Main/VBoxContainer_Waiting/Label_Waiting


func show_panel(child_node: Control):
	for child in panel_main.get_children():
		child.hide()

	child_node.show()


func show_waiting_panel(text: String):
	label_waiting.text = text
	show_panel(v_box_container_waiting)


func close_sign_in():
	self.hide()
	self.queue_free.call_deferred()


func _ready():
	Global.player_identity.need_open_url.connect(self._on_need_open_url)
	Global.player_identity.wallet_connected.connect(self._on_wallet_connected)

	Global.scene_runner.set_pause(true)
	show_panel(v_box_container_connect)


func _on_button_sign_in_pressed():
	Global.player_identity.try_connect_account()
	show_waiting_panel("Please follow the steps to connect your account and sign the message")


func _on_button_guest_pressed():
	show_panel(v_box_container_guest_confirm)


func _on_button_confirm_guest_risk_pressed():
	Global.player_identity.create_guest_account()


func _on_need_open_url(url: String, _description: String) -> void:
	OS.shell_open(url)


func _on_wallet_connected(_address: String, _chain_id: int, is_guest: bool) -> void:
	Global.scene_runner.set_pause(false)
	Global.config.session_account = {}

	if not is_guest:
		var new_stored_account := {}
		if Global.player_identity.get_recover_account_to(new_stored_account):
			Global.config.session_account = new_stored_account

		Global.config.save_to_settings_file()

	# TODO: fetch profile
	# Global.player_identity.update_profile(Global.config.avatar_profile)
	
	close_sign_in()
