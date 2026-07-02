class_name CreditsPage
extends Control

## Emitted when the user presses the back button and is already on the main
## credits shop view (i.e. not inside purchase history).  The parent that
## instantiated this page should listen to this signal to hide/free the page.
signal closed

@onready var button_back: Button = %Button_Back
@onready var label_title: Label = %Label_Title
@onready var button_history: Button = %Button_History
@onready var button_credits: CreditsBalanceButton = %Button_Credits
@onready var credits_option: Container = %CreditsOption
@onready var credits_history: Container = %CreditsHistory


func _ready() -> void:
	UiSounds.install_audio_recusirve(self)
	button_back.pressed.connect(_on_button_back_pressed)
	button_history.pressed.connect(_on_button_history_pressed)
	Iap.transaction_history_updated.connect(_on_transaction_history_updated)

	_show_shop()


func _exit_tree() -> void:
	if Iap.transaction_history_updated.is_connected(_on_transaction_history_updated):
		Iap.transaction_history_updated.disconnect(_on_transaction_history_updated)


func _show_shop() -> void:
	label_title.text = "Credits"
	credits_option.show()
	credits_history.hide()
	button_history.show()
	button_history.disabled = Iap.get_transaction_history().size() == 0
	button_credits.visible = Iap.is_available()


func _show_history() -> void:
	label_title.text = "Purchases"
	credits_option.hide()
	credits_history.show()
	button_history.hide()
	button_credits.hide()


func _on_button_back_pressed() -> void:
	if credits_history.visible:
		_show_shop()
		return
	closed.emit()


func _on_button_history_pressed() -> void:
	_show_history()


func _on_transaction_history_updated() -> void:
	if credits_option.visible:
		button_history.show()
		button_history.disabled = false
