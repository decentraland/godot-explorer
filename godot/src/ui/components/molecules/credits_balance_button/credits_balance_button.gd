class_name CreditsBalanceButton
extends Button

## When true the button only displays the balance — pressing it does nothing.
@export var display_only: bool = false


func _ready() -> void:
	visible = Iap.is_available()
	if not visible:
		return
	text = str(Iap.get_balance())
	Iap.balance_changed.connect(_on_balance_changed)
	if not display_only:
		pressed.connect(_on_pressed)


func _exit_tree() -> void:
	if Iap.balance_changed.is_connected(_on_balance_changed):
		Iap.balance_changed.disconnect(_on_balance_changed)


func _on_balance_changed(new_balance: int) -> void:
	text = str(new_balance)


func _on_pressed() -> void:
	Global.metrics.track_click_button("BUTTON_CREDITS", "CREDITS_BALANCE_BUTTON", "")
	Global.open_credits.emit()
