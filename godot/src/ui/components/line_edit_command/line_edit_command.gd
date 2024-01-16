extends Control

signal submit_message(message: String)

@onready var line_edit_command = $HBoxContainer_Send/MarginContainer/LineEdit_Command
@onready var container_send = $HBoxContainer_Send


func _ready():
	container_send.hide()


func finish():
	if container_send.visible:
		container_send.hide()


func start():
	if not container_send.visible:
		line_edit_command.text = ""
		container_send.show()

	line_edit_command.grab_focus()


func _on_line_edit_command_text_submitted(new_text: String) -> void:
	submit_message.emit(new_text)
	Global.explorer_grab_focus()


func _on_send_pressed():
	submit_message.emit(line_edit_command.text)


func _on_button_open_chat_pressed():
	if container_send.visible:
		submit_message.emit(line_edit_command.text)
	else:
		start()
