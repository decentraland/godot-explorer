extends PanelContainer

@onready var label_balance: Label = %Label_Balance


func _ready() -> void:
	label_balance.text = str(Iap.get_balance())
	Iap.balance_changed.connect(_on_balance_changed)


func _exit_tree() -> void:
	if Iap.balance_changed.is_connected(_on_balance_changed):
		Iap.balance_changed.disconnect(_on_balance_changed)


func _on_balance_changed(new_balance: int) -> void:
	label_balance.text = str(new_balance)
