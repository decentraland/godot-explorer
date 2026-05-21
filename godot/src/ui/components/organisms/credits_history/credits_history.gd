extends VBoxContainer

const HISTORY_ITEM_SCENE = preload("res://src/ui/components/molecules/credits_history_item/credits_history_item.tscn")

@onready var item_container: VBoxContainer = $VBoxContainer


func _ready() -> void:
	for entry in Iap.get_transaction_history():
		_add_item(entry.credits, entry.is_refund, entry.timestamp)
	Iap.purchase_completed.connect(_on_purchase_completed)


func _on_purchase_completed(_product_id: String, credits: int) -> void:
	var now = Time.get_datetime_dict_from_system()
	var timestamp = "%04d.%02d.%02d" % [now.year, now.month, now.day]
	_add_item(credits, false, timestamp)


func _add_item(credits: int, is_refund: bool, timestamp: String) -> void:
	var item = HISTORY_ITEM_SCENE.instantiate()
	item_container.add_child(item)
	item.setup(credits, is_refund, timestamp)
