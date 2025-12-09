extends TextureRect

var success: bool = false

@onready var confirmation_dialog: VBoxContainer = %ConfirmationDialog
@onready var processing_screen: VBoxContainer = %ProcessingScreen
@onready var done_dialog: VBoxContainer = %DoneDialog
@onready var fail_dialog: VBoxContainer = %FailDialog

@onready var timer: Timer = %Timer


func _on_button_cancel_delete_account_pressed() -> void:
	hide()


func _on_button_ok_pressed() -> void:
	hide()


func _hide_all() -> void:
	confirmation_dialog.hide()
	processing_screen.hide()
	done_dialog.hide()
	fail_dialog.hide()


func start_flow() -> void:
	success = false
	_hide_all()
	confirmation_dialog.show()
	show()


func _async_on_button_confirm_delete_account_pressed() -> void:
	_hide_all()
	processing_screen.show()

	# SEND REQUEST (NEED SERVICE HERE)
	timer.start()


func _on_timer_timeout() -> void:
	_hide_all()
	if success:
		done_dialog.show()
	else:
		fail_dialog.show()


func _on_check_button_toggled(toggled_on: bool) -> void:
	success = toggled_on
