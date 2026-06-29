@tool
class_name GuestUpgradeCard
extends MarginContainer

signal email_added(email: String)

## Screen location for metrics tracking. "discover" auto-differentiates between
## discover_pregame and discover_ingame based on whether explorer is active.
@export_enum("discover", "settings") var shown_in = "discover"
## When true, removes the left and right margins so the card stretches edge to edge.
## Use in settings; leave false in discover where lateral margins are needed.
@export var full_width: bool = false:
	set(value):
		full_width = value
		_apply_full_width()
## Left and right margin (px) applied when full_width is false.
@export var side_margin: int = 48:
	set(value):
		side_margin = value
		_apply_full_width()

## True once the network check has been attempted (prevents re-checking every time
## the parent becomes visible after a successful initial check).
var _upgrade_checked: bool = false

@onready var button_add_email: Button = %Button_AddEmail

static var _email_regex: RegEx = RegEx.create_from_string("^[^@\\s]+@[^@\\s]+\\.[^@\\s]+$")


static func is_valid_email(text: String) -> bool:
	return _email_regex.search(text) != null


func _apply_full_width() -> void:
	if full_width:
		add_theme_constant_override("margin_left", 0)
		add_theme_constant_override("margin_right", 0)
	else:
		add_theme_constant_override("margin_left", side_margin)
		add_theme_constant_override("margin_right", side_margin)


func _ready() -> void:
	_apply_full_width()
	if Engine.is_editor_hint():
		return
	button_add_email.pressed.connect(_async_on_add_email_pressed)
	visibility_changed.connect(_on_visibility_changed)
	Global.orientation_changed.connect(_on_orientation_changed)
	_async_update_visibility()


func _on_orientation_changed(_is_portrait: bool) -> void:
	_async_update_visibility()


func refresh_visibility() -> void:
	_async_update_visibility()


# The upgrade affordance is only meaningful for a thirdweb guest that hasn't
# linked anything yet. Real wallets, disposable LocalWallets, or already-upgraded
# guests (email/social linked) must not see it. The "already upgraded" bit can't
# be known locally — a recovered session doesn't record it — so we ask thirdweb
# for the linked profiles. Start hidden, then reveal only once confirmed.
# After the first network check, subsequent calls use the local cached flag.
# gdlint:ignore = async-function-name
func _async_update_visibility() -> void:
	visible = false
	if not Global.is_orientation_portrait():
		return
	if Global.player_identity == null or not Global.player_identity.is_thirdweb_guest():
		return

	if _upgrade_checked:
		# Already have an authoritative result — use the Rust-cached flag.
		visible = not Global.player_identity.is_thirdweb_guest_upgraded()
		return

	_upgrade_checked = true
	var anchor: String = Global.get_device_anchor_id()
	var promise: Promise = Global.player_identity.async_refresh_thirdweb_upgrade_state(anchor)
	var result = await PromiseUtils.async_awaiter(promise)
	var is_upgraded: bool
	if result is PromiseError:
		# Couldn't confirm against thirdweb — fall back to the last-known cached
		# flag rather than assume "not upgraded" and wrongly offer the upgrade.
		is_upgraded = Global.player_identity.is_thirdweb_guest_upgraded()
	else:
		# result is the authoritative upgraded bool.
		is_upgraded = bool(result)
	visible = not is_upgraded
	Global.guest_upgrade_state_refreshed.emit(is_upgraded)


func _get_shown_in() -> String:
	if shown_in == "discover":
		return "discover_ingame" if Global.get_explorer() else "discover_pregame"
	return shown_in


func _on_visibility_changed() -> void:
	if is_visible_in_tree():
		Global.metrics.track_screen_viewed(
			"UPGRADE_NOTICE_SHOW", JSON.stringify({"shown_in": _get_shown_in()})
		)


func _async_on_add_email_pressed() -> void:
	Global.metrics.track_click_button(
		"UPGRADE_NOTICE_TAP", "UPGRADE_NOTICE_SHOW", JSON.stringify({"shown_in": _get_shown_in()})
	)
	var modal = await (
		Global
		. modal_manager
		. async_show_input_modal(
			"Add Email",
			"My email",
			"name@email.com",
			"ADD",
			"CANCEL",
			is_valid_email,
		)
	)
	if modal:
		modal.dismissable = false
		modal.dcl_text_edit.wrap_text = false
		modal.dcl_text_edit.validate_on_blur = true
		# The modal owns the spinner + close timing now: it runs _async_send_code
		# while showing a spinner, keeps itself open (inline error) for a bad
		# address, or closes and emits confirmed/failed for the other outcomes.
		modal.set_submit_handler(_async_send_code)
		modal.confirmed.connect(_async_on_code_sent)
		modal.failed.connect(_async_on_send_code_failed)


# Submit handler for the Add Email modal. Sends the verification code and maps
# the outcome to the modal's status contract: a malformed address stays inline
# as "Invalid email"; any other failure closes the modal and bubbles up via
# `failed`; success closes the modal and emits `confirmed`.
# gdlint:ignore = async-function-name
func _async_send_code(email: String) -> Dictionary:
	Global.metrics.track_click_button("upgrade_to_otp_send_code", "upgrade_otp_modal", "")
	# async_link_email_start both validates the address server-side and sends the
	# verification code to the thirdweb guest wallet.
	var promise: Promise = Global.player_identity.async_link_email_start(email)
	var result = await PromiseUtils.async_awaiter(promise)
	if result is PromiseError:
		var raw: String = result.get_error()
		printerr("Upgrade to OTP - send code failed: ", raw)
		if _is_invalid_email_error(raw):
			return {"status": InputModal.SUBMIT_INVALID, "message": "Invalid email"}
		return {"status": InputModal.SUBMIT_ERROR, "message": raw}
	print("[UpgradeOTP] send_code OK for: ", email)
	return {"status": InputModal.SUBMIT_OK}


# Code was sent successfully (Add Email modal already closed): open the code step.
# gdlint:ignore = async-function-name
func _async_on_code_sent(email: String) -> void:
	var code_modal = await Global.modal_manager.async_show_code_modal(email)
	if code_modal:
		# The code modal owns the spinner + inline error UI; it calls back into
		# _async_verify_code and only emits `confirmed` once verification succeeds.
		code_modal.set_verifier(_async_verify_code.bind(email))
		code_modal.set_resend_handler(_async_send_code.bind(email))
		code_modal.confirmed.connect(_async_on_code_confirmed.bind(email))
		code_modal.cancelled.connect(Global.modal_manager.close_code_modal)


# Non-recoverable send-code failure (Add Email modal already closed): show a
# generic retry-later message.
# gdlint:ignore = async-function-name
func _async_on_send_code_failed(_message: String) -> void:
	await _async_show_error_modal(
		"Something went wrong", "Something went wrong. Please try again later."
	)


# True only when thirdweb rejected the address itself as malformed (HTTP 400 /
# Zod email validation), which we surface inline. Network/5xx/rate-limit and
# every other failure are treated as transient and shown after closing the modal.
func _is_invalid_email_error(raw: String) -> bool:
	var lower := raw.to_lower()
	if lower.contains("invalid email"):
		return true
	return (
		lower.contains("email") and (lower.contains("zoderror") or lower.contains("invalid_string"))
	)


# Returns "" on success, or a friendly error string the code modal shows inline.
# gdlint:ignore = async-function-name
func _async_verify_code(code: String, email: String) -> String:
	Global.metrics.track_click_button("upgrade_to_otp_verify", "upgrade_otp_modal", "")
	var anchor: String = Global.get_device_anchor_id()
	var promise: Promise = Global.player_identity.async_link_email_verify(email, code, anchor)
	var result = await PromiseUtils.async_awaiter(promise)
	print("[UpgradeOTP] verify result: ", result)
	if result is PromiseError:
		var raw: String = result.get_error()
		printerr("[UpgradeOTP] verify FAILED: ", raw)
		if _is_already_linked_error(raw):
			Global.modal_manager.close_code_modal.call_deferred()
			_async_show_email_in_use_modal.call_deferred()
			# Return non-empty so the code modal doesn't emit confirmed
			# (it will try _show_error but the deferred close frees it first).
			return " "
		return _friendly_error(raw)
	print("[UpgradeOTP] verify OK")
	return ""


func _is_already_linked_error(raw: String) -> bool:
	var lower := raw.to_lower()
	return lower.contains("already") or lower.contains("linked") or lower.contains("conflict")


func _async_show_email_in_use_modal() -> void:
	await _async_show_error_modal(
		"Email already in use",
		"This email is already linked to another account.\nTry a different email.",
	)


# gdlint:ignore = async-function-name
func _async_on_code_confirmed(_code: String, email: String) -> void:
	# Verification already succeeded inside the code modal before `confirmed` fired.
	Global.modal_manager.close_code_modal()
	await _async_show_success_modal()
	email_added.emit(email)
	# Now upgraded (Rust set the cached flag on link) — hide the affordance and
	# notify other UI (badge) so they also update without a separate network call.
	visible = false
	Global.guest_upgrade_state_refreshed.emit(true)


# Maps raw thirdweb errors to friendly copy. The raw error is still logged.
func _friendly_error(raw: String) -> String:
	var lower := raw.to_lower()
	if lower.contains("429") or lower.contains("rate"):
		return "Too many attempts. Please wait a moment and try again."
	if lower.contains("already") or lower.contains("linked") or lower.contains("conflict"):
		return "This email is already linked to another account."
	if lower.contains("invalid") or lower.contains("code") or lower.contains("400"):
		return "The code is invalid or expired. Please resend code."
	return "Something went wrong. Please try again."


func _async_show_success_modal() -> void:
	var modal = await Global.modal_manager._async_create_modal()
	if not modal:
		return
	modal.set_title("You're all set")
	modal.set_body("Your email has been added to your account.")
	modal.set_primary_button_text("GOT IT")
	modal.button_secondary.hide()
	modal.hide_url()
	modal.hide_icon()
	modal.blocker = true
	modal.show()
	await modal.button_primary.pressed
	Global.modal_manager.close_current_modal()


func _async_show_error_modal(title: String, body: String) -> void:
	var modal = await Global.modal_manager._async_create_modal()
	if not modal:
		return
	modal.set_title(title)
	modal.set_body(body)
	modal.set_primary_button_text("OK")
	modal.show_icon(Modal.MODAL_ALERT_ICON)
	modal.button_secondary.hide()
	modal.hide_url()
	modal.blocker = true
	modal.show()
	await modal.button_primary.pressed
	Global.modal_manager.close_current_modal()
