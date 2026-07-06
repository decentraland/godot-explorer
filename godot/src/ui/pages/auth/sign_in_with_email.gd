class_name SignInWithEmail
extends Control

const SEPARATOR_HEIGHT_DEFAULT := 150
const SEPARATOR_HEIGHT_KB_OPEN := 30

var lobby: Lobby = null

@onready var email_input: DclTextEdit = %DclTextEdit
@onready var button_confirm: Button = %Button_Confirm
@onready var vb_separator: Control = %VSeparatorVK


func _ready() -> void:
	email_input.validate_on_blur = true
	button_confirm.pressed.connect(_on_button_confirm_pressed)
	email_input.dcl_text_edit_changed.connect(_on_email_input_changed)
	Global.change_virtual_keyboard.connect(_on_virtual_keyboard_changed)


func _on_virtual_keyboard_changed(keyboard_height: int) -> void:
	if keyboard_height > 0:
		vb_separator.custom_minimum_size.y = SEPARATOR_HEIGHT_KB_OPEN
	else:
		vb_separator.custom_minimum_size.y = SEPARATOR_HEIGHT_DEFAULT


func set_lobby(new_lobby: Lobby) -> void:
	lobby = new_lobby


func setup() -> void:
	button_confirm.disabled = true
	email_input.set_text_value("")


func _on_email_input_changed() -> void:
	button_confirm.disabled = email_input.error or email_input.get_text_value().is_empty()


func _on_button_confirm_pressed() -> void:
	if email_input.error or email_input.get_text_value().is_empty():
		return

	var email: String = email_input.get_text_value()
	button_confirm.disabled = true

	var promise: Promise = Global.player_identity.async_link_email_start(email)
	var result = await PromiseUtils.async_awaiter(promise)

	button_confirm.disabled = false

	if result is PromiseError:
		var raw: String = result.get_error()
		push_warning("[OTP SignIn] send_code failed: " + raw)
		Global.metrics.track_screen_viewed("AUTH_OTP_EMAIL_INVALID", "")
		email_input.show_external_error(
			"Couldn't send code. Please check your email and try again."
		)
		return

	DisplayServer.virtual_keyboard_hide()
	Global.metrics.track_screen_viewed("AUTH_OTP_ENTERCODE", "")
	lobby.waiting_for_new_wallet = true
	lobby.show_auth_browser_open_screen("Signing in...", "otp")

	var code_modal = await Global.modal_manager.async_show_code_modal(email)
	if code_modal:
		code_modal.set_verifier(_async_otp_verify_code.bind(email))
		code_modal.set_resend_handler(_async_otp_resend_code.bind(email))
		code_modal.confirmed.connect(_async_otp_code_confirmed.bind(email))
		code_modal.cancelled.connect(
			func():
				lobby.waiting_for_new_wallet = false
				Global.modal_manager.close_code_modal()
				lobby.show_auth_email_screen()
		)


# gdlint:ignore = async-function-name
func _async_otp_verify_code(code: String, email: String) -> String:
	Global.metrics.track_screen_viewed("AUTH_OTP_VERIFY", "")
	var promise: Promise = Global.player_identity.async_login_email_verify(email, code)
	var result = await PromiseUtils.async_awaiter(promise)
	if result is PromiseError:
		var raw: String = result.get_error()
		push_warning("[OTP SignIn] verify failed: " + raw)
		Global.metrics.track_screen_viewed("AUTH_OTP_ERROR", "")
		return raw
	return ""


# gdlint:ignore = async-function-name
func _async_otp_resend_code(email: String) -> Dictionary:
	var promise: Promise = Global.player_identity.async_link_email_start(email)
	var result = await PromiseUtils.async_awaiter(promise)
	if result is PromiseError:
		return {"status": 2, "message": result.get_error()}
	return {"status": 0}


# gdlint:ignore = async-function-name
func _async_otp_code_confirmed(_code: String, email: String) -> void:
	Global.metrics.track_screen_viewed(
		"AUTH_SUCCESS", JSON.stringify({"login_type": "otp", "email": email})
	)
	lobby.show_auth_browser_open_screen("Signing in...", "otp")
	Global.modal_manager.close_code_modal()
