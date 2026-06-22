class_name GuestUpgradeCard
extends MarginContainer

signal email_added(email: String)

## Screen location for metrics tracking. "discover" auto-differentiates between
## discover_pregame and discover_ingame based on whether explorer is active.
@export_enum("discover", "settings") var shown_in = "discover"
@onready var button_add_email: Button = %Button_AddEmail

static var _email_regex: RegEx = RegEx.create_from_string("^[^@\\s]+@[^@\\s]+\\.[^@\\s]+$")


static func is_valid_email(text: String) -> bool:
	return _email_regex.search(text) != null


func _ready() -> void:
	button_add_email.pressed.connect(_async_on_add_email_pressed)
	visibility_changed.connect(_on_visibility_changed)
	_async_update_visibility()


# The upgrade affordance is only meaningful for a thirdweb guest that hasn't
# linked anything yet. Real wallets, disposable LocalWallets, or already-upgraded
# guests (email/social linked) must not see it. The "already upgraded" bit can't
# be known locally — a recovered session doesn't record it — so we ask thirdweb
# for the linked profiles. Start hidden, then reveal only once confirmed.
# gdlint:ignore = async-function-name
func _async_update_visibility() -> void:
	visible = false
	if not Global.is_orientation_portrait():
		return
	if Global.player_identity == null or not Global.player_identity.is_thirdweb_guest():
		return

	var anchor: String = Global.get_device_anchor_id()
	var promise: Promise = Global.player_identity.async_refresh_thirdweb_upgrade_state(anchor)
	var result = await PromiseUtils.async_awaiter(promise)
	if result is PromiseError:
		# Couldn't confirm against thirdweb — fall back to the last-known cached
		# flag rather than assume "not upgraded" and wrongly offer the upgrade.
		visible = not Global.player_identity.is_thirdweb_guest_upgraded()
		return
	# result is the authoritative upgraded bool.
	visible = not result


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
		modal.dcl_text_edit.wrap_text = false
		modal.confirmed.connect(_async_on_email_confirmed)


# gdlint:ignore = async-function-name
func _async_on_email_confirmed(email: String) -> void:
	Global.metrics.track_click_button("upgrade_to_otp_send_code", "upgrade_otp_modal", "")
	# async_link_email_start both validates the address server-side and sends the
	# verification code to the thirdweb guest wallet. Only open the code step on
	# success; on failure surface a friendly error and let the user retry.
	var promise: Promise = Global.player_identity.async_link_email_start(email)
	var result = await PromiseUtils.async_awaiter(promise)
	if result is PromiseError:
		printerr("Upgrade to OTP - send code failed: ", result.get_error())
		await _async_show_error_modal("Couldn't send code", _friendly_error(result.get_error()))
		return

	var code_modal = await Global.modal_manager.async_show_code_modal(email)
	if code_modal:
		# The code modal owns the spinner + inline error UI; it calls back into
		# _async_verify_code and only emits `confirmed` once verification succeeds.
		code_modal.set_verifier(_async_verify_code.bind(email))
		code_modal.confirmed.connect(_async_on_code_confirmed.bind(email))
		code_modal.cancelled.connect(Global.modal_manager.close_code_modal)


# Returns "" on success, or a friendly error string the code modal shows inline.
# gdlint:ignore = async-function-name
func _async_verify_code(code: String, email: String) -> String:
	Global.metrics.track_click_button("upgrade_to_otp_verify", "upgrade_otp_modal", "")
	var anchor: String = Global.get_device_anchor_id()
	var promise: Promise = Global.player_identity.async_link_email_verify(email, code, anchor)
	var result = await PromiseUtils.async_awaiter(promise)
	if result is PromiseError:
		printerr("Upgrade to OTP - verify failed: ", result.get_error())
		return _friendly_error(result.get_error())
	return ""


# gdlint:ignore = async-function-name
func _async_on_code_confirmed(_code: String, email: String) -> void:
	# Verification already succeeded inside the code modal before `confirmed` fired.
	Global.modal_manager.close_code_modal()
	await _async_show_success_modal()
	email_added.emit(email)
	# Now upgraded (Rust set the cached flag on link) — hide the affordance.
	visible = false


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
