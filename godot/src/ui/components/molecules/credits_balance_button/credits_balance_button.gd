class_name CreditsBalanceButton
extends Button

## When true the button only displays the balance — pressing it does nothing.
@export var display_only: bool = false


func _ready() -> void:
	visible = false
	if not Iap.is_available():
		return
	if not _is_account_eligible():
		Global.guest_upgrade_state_refreshed.connect(_on_guest_upgrade_state_refreshed)
		return
	_show()


func _exit_tree() -> void:
	if Iap.balance_changed.is_connected(_on_balance_changed):
		Iap.balance_changed.disconnect(_on_balance_changed)


func _is_account_eligible() -> bool:
	if Global.player_identity == null:
		return false
	return (
		not Global.player_identity.is_thirdweb_guest()
		or Global.player_identity.is_thirdweb_guest_upgraded()
	)


func _show() -> void:
	text = str(Iap.get_balance())
	Iap.balance_changed.connect(_on_balance_changed)
	if not display_only:
		pressed.connect(_on_pressed)
	visible = true


func _on_guest_upgrade_state_refreshed(is_upgraded: bool) -> void:
	if not is_upgraded:
		return
	Global.guest_upgrade_state_refreshed.disconnect(_on_guest_upgrade_state_refreshed)
	_show()


func _on_balance_changed(new_balance: int) -> void:
	text = str(new_balance)


func _on_pressed() -> void:
	Global.metrics.track_click_button("BUTTON_CREDITS", "CREDITS_BALANCE_BUTTON", "")
	Global.open_credits.emit()
