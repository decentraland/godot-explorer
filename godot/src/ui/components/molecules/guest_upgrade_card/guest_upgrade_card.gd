class_name GuestUpgradeCard
extends PanelContainer

signal email_added(email: String)

## Screen location for metrics tracking. "discover" auto-differentiates between
## discover_pregame and discover_ingame based on whether explorer is active.
@export_enum("discover", "settings") var shown_in = "discover"

static var _email_regex: RegEx = RegEx.create_from_string("^[^@\\s]+@[^@\\s]+\\.[^@\\s]+$")


static func is_valid_email(text: String) -> bool:
	return _email_regex.search(text) != null


func _ready() -> void:
	$MarginContainer/VBoxContainer/Button_AddEmail.pressed.connect(_async_on_add_email_pressed)
	visibility_changed.connect(_on_visibility_changed)


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
		modal.confirmed.connect(_async_on_email_confirmed)


func _async_on_email_confirmed(email: String) -> void:
	var is_valid = await _async_validate_email(email)
	if not is_valid:
		return

	var code_modal = await Global.modal_manager.async_show_code_modal()
	if code_modal:
		code_modal.confirmed.connect(_async_on_code_confirmed.bind(email))
		code_modal.cancelled.connect(Global.modal_manager.close_code_modal)


func _async_validate_email(_email: String) -> bool:
	return true


func _async_on_code_confirmed(code: String, email: String) -> void:
	Global.modal_manager.close_code_modal()
	# TODO: replace mock with backend validation
	if code == "111111":
		await _async_show_email_in_use_modal()
	else:
		await _async_show_success_modal()
		email_added.emit(email)


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


func _async_show_email_in_use_modal() -> void:
	var modal = await Global.modal_manager._async_create_modal()
	if not modal:
		return
	modal.set_title("Email already in use")
	modal.set_body("This email is already linked to another account.\nTry a different email.")
	modal.set_primary_button_text("OK")
	modal.show_icon(Modal.MODAL_ALERT_ICON)
	modal.button_secondary.hide()
	modal.hide_url()
	modal.blocker = true
	modal.show()
	await modal.button_primary.pressed
	Global.modal_manager.close_current_modal()
