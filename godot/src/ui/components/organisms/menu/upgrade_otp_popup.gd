extends TextureRect

# Multi-step "Upgrade to OTP" modal: links an email to the active thirdweb guest
# wallet so the SAME address becomes email-recoverable. Modeled on
# account_deletion_popup.gd (email step → processing → code step → done / error).
# Only meaningful while Global.player_identity.is_thirdweb_guest() is true; the
# Settings button that opens it is gated on that flag.

const _EMAIL_REGEX := r"^[^@\s]+@[^@\s]+\.[^@\s]+$"
const _CODE_REGEX := r"^[0-9]{4,8}$"

var _email: String = ""

@onready var email_step: VBoxContainer = %EmailStep
@onready var processing_screen: VBoxContainer = %ProcessingScreen
@onready var code_step: VBoxContainer = %CodeStep
@onready var done_dialog: VBoxContainer = %DoneDialog

@onready var line_edit_email: LineEdit = %LineEdit_Email
@onready var label_email_error: Label = %Label_EmailError

@onready var line_edit_code: LineEdit = %LineEdit_Code
@onready var label_code_subtitle: Label = %Label_CodeSubtitle
@onready var label_code_error: Label = %Label_CodeError


func _ready() -> void:
	hide()


# Public entry point: reset and show the email step. Called by the menu when the
# Settings "Upgrade to OTP" button fires Global.upgrade_to_otp.
func async_start_flow() -> void:
	_email = ""
	line_edit_email.text = ""
	line_edit_code.text = ""
	label_email_error.hide()
	label_code_error.hide()
	show()
	_show_email_step()


func _hide_all() -> void:
	email_step.hide()
	processing_screen.hide()
	code_step.hide()
	done_dialog.hide()


func _show_email_step() -> void:
	_hide_all()
	email_step.show()
	line_edit_email.grab_focus()


func _show_processing() -> void:
	_hide_all()
	processing_screen.show()


func _show_code_step() -> void:
	_hide_all()
	label_code_subtitle.text = "We sent a code to %s" % _email
	code_step.show()
	line_edit_code.grab_focus()


func _show_done() -> void:
	_hide_all()
	done_dialog.show()


func _close_flow() -> void:
	hide()
	if Global.get_explorer():
		Global.close_navbar.emit()
		Global.close_menu.emit()


# Maps raw thirdweb errors to friendly copy. The raw error is still logged.
func _friendly_error(raw: String) -> String:
	var lower := raw.to_lower()
	if lower.contains("429") or lower.contains("rate"):
		return "Too many attempts. Please wait a moment and try again."
	if lower.contains("already") or lower.contains("linked") or lower.contains("conflict"):
		return "This email is already linked to another account."
	if lower.contains("invalid") or lower.contains("code") or lower.contains("400"):
		return "That code didn't work. Check it and try again."
	return "Something went wrong. Please try again."


# gdlint:ignore = async-function-name
func _on_button_send_code_pressed() -> void:
	var email := line_edit_email.text.strip_edges()
	var regex := RegEx.new()
	regex.compile(_EMAIL_REGEX)
	if regex.search(email) == null:
		label_email_error.text = "Please enter a valid email address."
		label_email_error.show()
		return

	label_email_error.hide()
	_email = email
	Global.metrics.track_click_button("upgrade_to_otp_send_code", "upgrade_otp_modal", "")
	_show_processing()

	var promise: Promise = Global.player_identity.async_link_email_start(email)
	var result = await PromiseUtils.async_awaiter(promise)

	if result is PromiseError:
		printerr("Upgrade to OTP - send code failed: ", result.get_error())
		label_email_error.text = _friendly_error(result.get_error())
		label_email_error.show()
		_show_email_step()
		return

	_show_code_step()


# gdlint:ignore = async-function-name
func _on_button_resend_pressed() -> void:
	label_code_error.hide()
	_show_processing()

	var promise: Promise = Global.player_identity.async_link_email_start(_email)
	var result = await PromiseUtils.async_awaiter(promise)

	if result is PromiseError:
		printerr("Upgrade to OTP - resend failed: ", result.get_error())
		label_code_error.text = _friendly_error(result.get_error())
		_show_code_step()
		label_code_error.show()
		return

	_show_code_step()


# gdlint:ignore = async-function-name
func _on_button_verify_pressed() -> void:
	var code := line_edit_code.text.strip_edges()
	var regex := RegEx.new()
	regex.compile(_CODE_REGEX)
	if regex.search(code) == null:
		label_code_error.text = "Enter the numeric code we sent you."
		label_code_error.show()
		return

	label_code_error.hide()
	Global.metrics.track_click_button("upgrade_to_otp_verify", "upgrade_otp_modal", "")
	_show_processing()

	var anchor: String = Global.get_device_anchor_id()
	var promise: Promise = Global.player_identity.async_link_email_verify(_email, code, anchor)
	var result = await PromiseUtils.async_awaiter(promise)

	if result is PromiseError:
		printerr("Upgrade to OTP - verify failed: ", result.get_error())
		label_code_error.text = _friendly_error(result.get_error())
		_show_code_step()
		label_code_error.show()
		return

	_show_done()


func _on_button_cancel_pressed() -> void:
	_close_flow()


func _on_button_done_pressed() -> void:
	_close_flow()


func _on_visibility_changed() -> void:
	if Global.get_explorer():
		var navbar = Global.get_explorer().navbar
		if navbar:
			if visible:
				navbar.hide()
			else:
				navbar.show()
