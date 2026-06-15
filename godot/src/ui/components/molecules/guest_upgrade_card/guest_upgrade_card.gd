class_name GuestUpgradeCard
extends PanelContainer

signal email_added(email: String)

static var _email_regex: RegEx = RegEx.create_from_string("^[^@\\s]+@[^@\\s]+\\.[^@\\s]+$")


static func is_valid_email(text: String) -> bool:
	return _email_regex.search(text) != null


func _on_button_add_email_pressed() -> void:
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
		modal.confirmed.connect(_on_email_confirmed)


func _on_email_confirmed(email: String) -> void:
	email_added.emit(email)
