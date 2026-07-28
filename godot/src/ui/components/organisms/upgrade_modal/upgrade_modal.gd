class_name UpgradeModal
extends ColorRect

## Aspirational guest-upgrade nudge (issue #2372). Encourages a guest to link an email
## (account upgrade) in exchange for cross-device access and an exclusive wearable. It is
## shown on a capped schedule by UpgradeNudgeCoordinator; the reward itself is granted
## afterwards by the reward modal when the upgrade succeeds.

@onready var button_add_email: Button = %Button_AddEmail
@onready var button_maybe_later: Button = %Button_MaybeLater


func _ready() -> void:
	button_add_email.pressed.connect(_on_add_email_pressed)
	button_maybe_later.pressed.connect(_on_maybe_later_pressed)


## Reveals the modal and reports the SCREEN_VIEW analytics event.
func open() -> void:
	show()
	if Global.metrics != null:
		Global.metrics.track_screen_viewed("ACCOUNT_UPGRADE_MODAL_SHOW", "")


func _on_add_email_pressed() -> void:
	if Global.metrics != null:
		# Screen name must match the SCREEN_VIEW above so the click joins it in analytics.
		Global.metrics.track_click_button("add_email", "ACCOUNT_UPGRADE_MODAL_SHOW", "")
	# Close this nudge and open the same "Add Email" input-modal flow the Discover upgrade
	# notice uses — one shared experience that ends in the reward (issues #2377 / #2372).
	Global.modal_manager.close_upgrade_modal()
	Global.modal_manager.async_start_add_email_flow()


func _on_maybe_later_pressed() -> void:
	if Global.metrics != null:
		Global.metrics.track_click_button("maybe_later", "ACCOUNT_UPGRADE_MODAL_SHOW", "")
	Global.modal_manager.close_upgrade_modal()


# Backdrop taps must not dismiss the nudge — the user decides via the explicit buttons.
func _on_gui_input(_event: InputEvent) -> void:
	pass
