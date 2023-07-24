extends Control

signal submit_message(message: String)

@onready var line_edit_command = $LineEdit_Command
@onready var button_send = $Button_Send


func _on_button_send_pressed():
	if line_edit_command.visible:
		submit_message.emit(line_edit_command.text)
	else:
		start()


func finish():
	if line_edit_command.visible:
		line_edit_command.hide()

	button_send.text = "Talk"


func start():
	if not line_edit_command.visible:
		line_edit_command.text = ""
		line_edit_command.show()
		button_send.text = "Send"

	line_edit_command.grab_focus()


func _on_line_edit_command_text_submitted(new_text: String) -> void:
	submit_message.emit(new_text)
