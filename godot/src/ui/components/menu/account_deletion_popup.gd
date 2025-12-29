extends TextureRect

const DELETION_API_URL = "https://mobile-bff.decentraland.zone/deletion"

@onready var confirmation_dialog: VBoxContainer = %ConfirmationDialog
@onready var processing_screen: VBoxContainer = %ProcessingScreen
@onready var done_dialog: VBoxContainer = %DoneDialog
@onready var fail_dialog: VBoxContainer = %FailDialog
@onready var button_cancel_deletion: Button = %Button_CancelDeletion


func _ready() -> void:
	hide()
	# Hide the cancel deletion button
	if button_cancel_deletion:
		button_cancel_deletion.get_parent().hide()


func _on_button_cancel_delete_account_pressed() -> void:
	hide()


func _on_button_ok_pressed() -> void:
	hide()


func _hide_all() -> void:
	confirmation_dialog.hide()
	processing_screen.hide()
	done_dialog.hide()
	fail_dialog.hide()


func async_start_flow() -> void:
	show()
	_hide_all()
	processing_screen.show()

	# Check if deletion was already requested from server
	var response = await Global.async_signed_fetch(DELETION_API_URL, HTTPClient.METHOD_GET, "")

	_hide_all()

	if response is PromiseError:
		printerr("Failed to check deletion status: ", response.get_error())
		fail_dialog.show()
		return

	var data: Dictionary = response.get_string_response_as_json()

	if not data.get("ok", false):
		printerr("Failed to check deletion status: ", data.get("error", "Unknown error"))
		fail_dialog.show()
		return

	var deletion_data = data.get("data")
	if deletion_data != null and deletion_data.get("status") == "pending":
		# Already has a pending deletion request
		done_dialog.show()
	else:
		# No pending request, show confirmation
		confirmation_dialog.show()


func _async_on_button_confirm_delete_account_pressed() -> void:
	_hide_all()
	processing_screen.show()

	var response = await Global.async_signed_fetch(DELETION_API_URL, HTTPClient.METHOD_POST, "")

	_hide_all()

	if response is PromiseError:
		printerr("Account deletion request failed: ", response.get_error())
		fail_dialog.show()
		return

	var data: Dictionary = response.get_string_response_as_json()

	if not data.get("ok", false):
		printerr("Account deletion request failed: ", data.get("error", "Unknown error"))
		fail_dialog.show()
		return

	done_dialog.show()


func _async_on_button_cancel_deletion_pressed() -> void:
	# This function is kept but the button is hidden
	_hide_all()
	processing_screen.show()

	var response = await Global.async_signed_fetch(DELETION_API_URL, HTTPClient.METHOD_DELETE, "")

	_hide_all()

	if response is PromiseError:
		printerr("Cancel deletion request failed: ", response.get_error())
		fail_dialog.show()
		return

	var data: Dictionary = response.get_string_response_as_json()

	if data.get("ok", false):
		hide()
	else:
		printerr("Cancel deletion request failed: ", data.get("error", "Unknown error"))
		fail_dialog.show()
