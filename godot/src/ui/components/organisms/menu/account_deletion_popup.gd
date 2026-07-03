extends TextureRect

# True when the current deletion targets a non-upgraded guest → automatic
# on-device deletion (no /deletion request, no "Deletion Requested" modal).
# Decided authoritatively in async_start_flow() before the confirmation dialog.
var _guest_auto_delete: bool = false

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
	_close_flow()


func _on_button_ok_pressed() -> void:
	_close_flow()


func _close_flow() -> void:
	hide()
	if Global.get_explorer():
		Global.close_navbar.emit()
		Global.close_menu.emit()


func _hide_all() -> void:
	confirmation_dialog.hide()
	processing_screen.hide()
	done_dialog.hide()
	fail_dialog.hide()


func async_start_flow() -> void:
	show()
	_hide_all()
	processing_screen.show()

	# Non-upgraded guests get an automatic on-device deletion (issue #2335): no
	# server request and no "Deletion Requested" modal. Everyone else (upgraded
	# guest / real wallet) keeps the manual /deletion request flow below.
	_guest_auto_delete = await _async_is_non_upgraded_guest()
	if _guest_auto_delete:
		_hide_all()
		confirmation_dialog.show()
		return

	# Check if deletion was already requested from server
	var response = await Global.async_signed_fetch(
		DclUrls.account_deletion(), HTTPClient.METHOD_GET, ""
	)

	_hide_all()

	if response is PromiseError:
		printerr("Failed to check deletion status: ", response.get_error())
		fail_dialog.show()
		return

	var data = response.get_string_response_as_json()

	if not data is Dictionary or not data.get("ok", false):
		var error_msg = (
			data.get("error", "Unknown error") if data is Dictionary else "Invalid response"
		)
		printerr("Failed to check deletion status: ", error_msg)
		fail_dialog.show()
		return

	var deletion_data = data.get("data")
	if deletion_data != null and deletion_data.get("status") == "pending":
		# Already has a pending deletion request
		done_dialog.show()
	else:
		# No pending request, show confirmation
		confirmation_dialog.show()


# Decides whether the active session is a non-upgraded guest, which takes the
# automatic-deletion path. The upgrade check is AUTHORITATIVE (a fresh thirdweb
# query) rather than the cached flag: the card that warms that cache is
# portrait-only, so in landscape the cache is cold and would misclassify an
# upgraded guest. Falls back to the cached flag only if the query fails, so a
# recoverable (upgraded) account is never auto-deleted on a transient error.
func _async_is_non_upgraded_guest() -> bool:
	var identity = Global.player_identity
	if identity == null:
		return false
	# Disposable dev guest (random LocalWallet): no server account — local-only.
	if identity.is_guest:
		return true
	if not identity.is_thirdweb_guest():
		return false

	var anchor: String = Global.get_device_anchor_id()
	var promise: Promise = identity.async_refresh_thirdweb_upgrade_state(anchor)
	var result = await PromiseUtils.async_awaiter(promise)
	var upgraded: bool
	if result is PromiseError:
		printerr("Delete flow: upgrade check failed, using cached flag: ", result.get_error())
		upgraded = identity.is_thirdweb_guest_upgraded()
	else:
		upgraded = bool(result)
	return not upgraded


# Automatic guest deletion (issue #2335): best-effort unlink of the thirdweb
# guest server-side (so the same device mints a fresh wallet next time), wipe
# all on-disk guest storage, then destroy the session and return to
# ACCOUNT_HOME. sign_out() swaps to the lobby and forces portrait, so this works
# from both portrait and landscape (in-game). No "Deletion Requested" modal.
func _async_perform_guest_auto_delete() -> void:
	_hide_all()
	processing_screen.show()

	var identity = Global.player_identity
	if identity != null and identity.is_thirdweb_guest():
		var anchor: String = Global.get_device_anchor_id()
		var promise: Promise = identity.async_delete_guest_account(anchor)
		var result = await PromiseUtils.async_awaiter(promise)
		if result is PromiseError:
			printerr("Guest deletion (thirdweb unlink) failed: ", result.get_error())

	Global.clear_guest_device_storage()
	hide()
	Global.sign_out()


func _async_on_button_confirm_delete_account_pressed() -> void:
	if _guest_auto_delete:
		await _async_perform_guest_auto_delete()
		return

	_hide_all()
	processing_screen.show()

	var response = await Global.async_signed_fetch(
		DclUrls.account_deletion(), HTTPClient.METHOD_POST, ""
	)

	_hide_all()

	if response is PromiseError:
		printerr("Account deletion request failed: ", response.get_error())
		fail_dialog.show()
		return

	var data = response.get_string_response_as_json()

	if not data is Dictionary or not data.get("ok", false):
		var error_msg = (
			data.get("error", "Unknown error") if data is Dictionary else "Invalid response"
		)
		printerr("Account deletion request failed: ", error_msg)
		fail_dialog.show()
		return

	done_dialog.show()


func _async_on_button_cancel_deletion_pressed() -> void:
	# This function is kept but the button is hidden
	_hide_all()
	processing_screen.show()

	var response = await Global.async_signed_fetch(
		DclUrls.account_deletion(), HTTPClient.METHOD_DELETE, ""
	)

	_hide_all()

	if response is PromiseError:
		printerr("Cancel deletion request failed: ", response.get_error())
		fail_dialog.show()
		return

	var data = response.get_string_response_as_json()

	if data is Dictionary and data.get("ok", false):
		hide()
	else:
		var error_msg = (
			data.get("error", "Unknown error") if data is Dictionary else "Invalid response"
		)
		printerr("Cancel deletion request failed: ", error_msg)
		fail_dialog.show()


func _on_visibility_changed() -> void:
	if Global.get_explorer():
		var navbar = Global.get_explorer().navbar
		if navbar:
			if visible:
				navbar.hide()
			else:
				navbar.show()
