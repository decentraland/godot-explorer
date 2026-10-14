class_name AddEmailFlow
extends Node

## The "Add Email" OTP account-upgrade flow (email → code → verify → reward), split out of
## ModalManager — which sits at the repo's 1900-line gdlint cap — rather than growing that file
## further (see CLAUDE.md "Code hygiene", same convention StreakModalController already follows).
## Leans on ModalManager's generic modal primitives (async_show_input_modal, async_show_code_modal,
## async_show_reward_modal, close_code_modal) rather than owning any modal lifecycle itself.

## Emitted exactly once per async_start_add_email_flow() call, whichever way the flow ends: a
## successful OTP verify, a cancel at either the email or code step, a send/verify error the user
## dismissed, or "email already in use". A caller awaiting ITS OWN flow's conclusion (e.g. the
## streak claim) uses this directly instead of Global.guest_upgrade_state_refreshed, which only
## fires on success and broadcasts to every upgrade-state observer app-wide — awaiting that one
## here would hang forever on any kind of cancel/failure (issue #3018).
signal flow_concluded(upgraded: bool)

static var _add_email_regex: RegEx = RegEx.create_from_string("^[^@\\s]+@[^@\\s]+\\.[^@\\s]+$")


static func is_valid_email(text: String) -> bool:
	return _add_email_regex.search(text) != null


## Opens the "Add Email" input modal and drives the full OTP link flow (email → code → verify →
## reward). Callers fire their own entry CLICK_BUTTON metric before calling this.
## @param grant_campaign_key: RewardCampaigns.CAMPAIGNS key granted on success; pass "" to link the
## email without granting anything (caller has its own post-upgrade reward step, e.g. streak claim).
## @param source: tags ACCOUNT_UPGRADE_SUCCESS/AUTH_SUCCESS with where the upgrade came from (e.g.
## "login_streak") — empty for the default nudge entry points, which carry no source today.
func async_start_add_email_flow(
	grant_campaign_key: String = "MobilePet", source: String = ""
) -> void:
	var modal = await Global.modal_manager.async_show_input_modal(
		TranslationKey.new("MODAL_ADD_EMAIL_TITLE"),
		TranslationKey.new("MODAL_ADD_EMAIL_SUBTITLE"),
		TranslationKey.new("MODAL_ADD_EMAIL_PLACEHOLDER"),
		TranslationKey.new("INPUT_MODAL_ADD"),
		TranslationKey.new("COMMON_CANCEL"),
		is_valid_email
	)
	if is_instance_valid(modal):
		# Add Email modal shown — the OTP upgrade funnel has started (issue #2377).
		Global.metrics.track_screen_viewed("UPGRADE_OTP_START", "")
		modal.dismissable = false
		modal.use_email_field()
		modal.set_submit_handler(_async_add_email_submit)
		modal.confirmed.connect(_async_add_email_code_sent.bind(grant_campaign_key, source))
		modal.failed.connect(_async_add_email_send_failed)
		modal.cancelled.connect(_on_flow_cancelled)


# Cancel at either the email or code step — whichever is actually open gets cleaned up properly
# (close_*_modal() does the is_instance_valid guard + CanvasLayer teardown close() alone skips);
# the other call is a harmless no-op.
func _on_flow_cancelled() -> void:
	Global.modal_manager.close_input_modal()
	Global.modal_manager.close_code_modal()
	flow_concluded.emit(false)


# gdlint:ignore = async-function-name
func _async_add_email_submit(email: String) -> Dictionary:
	Global.metrics.track_screen_viewed("UPGRADE_OTP_SUBMIT", "")
	return await _async_add_email_send_code(email)


# Shared "send verification code" routine used by both the initial submit and resend.
# gdlint:ignore = async-function-name
func _async_add_email_send_code(email: String) -> Dictionary:
	var promise: Promise = Global.player_identity.async_link_email_start(email)
	var result = await PromiseUtils.async_awaiter(promise)
	if result is PromiseError:
		var raw: String = result.get_error()
		push_warning("Upgrade to OTP - send code failed: " + raw)
		if _add_email_is_invalid_error(raw):
			Global.metrics.track_screen_viewed("UPGRADE_OTP_EMAIL_INVALID", "")
			return {"status": InputModal.SUBMIT_INVALID, "message": tr("MODAL_EMAIL_INVALID")}
		return {"status": InputModal.SUBMIT_ERROR, "message": raw}
	print("[UpgradeOTP] send_code OK for: ", email)
	return {"status": InputModal.SUBMIT_OK}


# Code was sent successfully (Add Email modal already closed): open the code step.
# gdlint:ignore = async-function-name
func _async_add_email_code_sent(email: String, grant_campaign_key: String, source: String) -> void:
	var code_modal = await Global.modal_manager.async_show_code_modal(email)
	if is_instance_valid(code_modal):
		Global.metrics.track_screen_viewed("UPGRADE_OTP_ENTERCODE", "")
		code_modal.set_verifier(_async_add_email_verify.bind(email))
		code_modal.set_resend_handler(_async_add_email_resend.bind(email))
		code_modal.confirmed.connect(
			_async_add_email_confirmed.bind(email, grant_campaign_key, source)
		)
		code_modal.cancelled.connect(_on_flow_cancelled)


# gdlint:ignore = async-function-name
func _async_add_email_resend(email: String) -> Dictionary:
	Global.metrics.track_click_button("RESEND_OTP", "UPGRADE_OTP_ENTERCODE", "")
	return await _async_add_email_send_code(email)


# gdlint:ignore = async-function-name
func _async_add_email_send_failed(_message: String) -> void:
	await _async_add_email_error_modal(
		TranslationKey.new("MODAL_ADD_EMAIL_ERROR_TITLE"),
		TranslationKey.new("MODAL_ADD_EMAIL_ERROR_BODY")
	)
	flow_concluded.emit(false)


func _add_email_is_invalid_error(raw: String) -> bool:
	var lower := raw.to_lower()
	if lower.contains("invalid email"):
		return true
	return (
		lower.contains("email") and (lower.contains("zoderror") or lower.contains("invalid_string"))
	)


# Returns "" on success, or a friendly error string the code modal shows inline.
# gdlint:ignore = async-function-name
func _async_add_email_verify(code: String, email: String) -> String:
	Global.metrics.track_screen_viewed("UPGRADE_OTP_VERIFY", "")
	var anchor: String = Global.get_device_anchor_id()
	var promise: Promise = Global.player_identity.async_link_email_verify(email, code, anchor)
	var result = await PromiseUtils.async_awaiter(promise)
	print("[UpgradeOTP] verify result: ", result)
	if result is PromiseError:
		var raw: String = result.get_error()
		push_warning("[UpgradeOTP] verify FAILED: " + raw)
		if _add_email_is_already_linked_error(raw):
			Global.metrics.track_screen_viewed("UPGRADE_OTP_EMAIL_BUSY", "")
			Global.modal_manager.close_code_modal.call_deferred()
			_async_add_email_in_use.call_deferred()
			return " "
		Global.metrics.track_screen_viewed("UPGRADE_OTP_CODE_INVALID", "")
		return _add_email_friendly_error(raw)
	print("[UpgradeOTP] verify OK")
	return ""


func _add_email_is_already_linked_error(raw: String) -> bool:
	var lower := raw.to_lower()
	return lower.contains("already") or lower.contains("linked") or lower.contains("conflict")


func _async_add_email_in_use() -> void:
	await _async_add_email_error_modal(
		TranslationKey.new("MODAL_EMAIL_IN_USE_TITLE"),
		TranslationKey.new("MODAL_EMAIL_IN_USE_BODY")
	)
	flow_concluded.emit(false)


# gdlint:ignore = async-function-name
func _async_add_email_confirmed(
	_code: String, _email: String, grant_campaign_key: String, source: String
) -> void:
	# Verification already succeeded inside the code modal before `confirmed` fired.
	Global.metrics.track_screen_viewed("ACCOUNT_UPGRADE_SUCCESS", JSON.stringify({"method": "otp"}))
	# Mirror the lobby's AUTH_SUCCESS so the funnel sees the upgraded user as registered; tag the
	# source when the caller has one (e.g. "login_streak") so account creation can be attributed.
	var auth_extra := {"login_type": "fully_registered"}
	if not source.is_empty():
		auth_extra["source"] = source
	Global.metrics.track_screen_viewed("AUTH_SUCCESS", JSON.stringify(auth_extra))
	Global.modal_manager.close_code_modal()
	# The reward modal IS the success screen ("...enjoy the free wearable!") when there's one to grant.
	if not grant_campaign_key.is_empty():
		var campaign: Dictionary = RewardCampaigns.CAMPAIGNS[grant_campaign_key]
		var reward_shown := await Global.modal_manager.async_show_reward_modal(campaign)
		if reward_shown:
			Global.metrics.track_screen_viewed(
				"ACCOUNT_UPGRADE_REWARD_SHOW", JSON.stringify({"method": "otp"})
			)
	# The guest is now upgraded (Rust set the cached flag on link) — notify the app so the
	# Discover/Settings notice, badge, credits button, etc. update without a network call.
	Global.guest_upgrade_state_refreshed.emit(true)
	flow_concluded.emit(true)


# Maps raw thirdweb errors to friendly copy. The raw error is still logged.
func _add_email_friendly_error(raw: String) -> String:
	var lower := raw.to_lower()
	if lower.contains("429") or lower.contains("rate"):
		return tr("COMMON_TOO_MANY_ATTEMPTS")
	if lower.contains("already") or lower.contains("linked") or lower.contains("conflict"):
		return tr("COMMON_EMAIL_ALREADY_LINKED")
	if lower.contains("invalid") or lower.contains("code") or lower.contains("400"):
		return tr("MODAL_CODE_INVALID_OR_EXPIRED")
	return tr("COMMON_SOMETHING_WENT_WRONG_RETRY")


# gdlint:ignore = async-function-name
func _async_add_email_error_modal(title: TranslationKey, body: TranslationKey) -> void:
	var modal = await Global.modal_manager._async_create_modal()
	if not is_instance_valid(modal):
		return
	modal.set_title(title)
	modal.set_body(body)
	modal.set_primary_button_text(TranslationKey.new("MENU_OK"))
	modal.show_icon(Modal.MODAL_ALERT_ICON)
	modal.button_secondary.hide()
	modal.hide_url()
	modal.blocker = true
	modal.show()
	await modal.button_primary.pressed
	Global.modal_manager.close_current_modal()
