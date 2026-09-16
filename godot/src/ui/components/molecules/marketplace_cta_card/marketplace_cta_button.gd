class_name MarketplaceCtaCard
extends Button

@export var marketplace_section: String = "wearables"

var _selected_price: int = -1


func _ready():
	pressed.connect(_on_pressed)
	Iap.balance_changed.connect(_on_balance_changed)
	_update_text()


func _exit_tree():
	if Iap.balance_changed.is_connected(_on_balance_changed):
		Iap.balance_changed.disconnect(_on_balance_changed)


## Called when a suggestion card is selected. Pass price=-1 to clear selection.
func update_selection(price: int):
	_selected_price = price
	_update_text()


func _on_balance_changed(_new_balance: int):
	_update_text()


func _update_text():
	# Keys, not text: the Button auto-translates, so the label also follows a language change.
	var balance = Iap.get_balance()
	if balance <= 0:
		text = "WEARABLE_ITEM_GET_CREDITS"
	elif _selected_price >= 0 and balance < _selected_price:
		text = "WEARABLE_ITEM_GET_CREDITS"
	else:
		text = "MARKETPLACE_CTA_GO_TO_SHOP"


func _on_pressed():
	var balance = Iap.get_balance()
	var can_afford = balance > 0
	if _selected_price >= 0:
		can_afford = balance >= _selected_price
	if can_afford:
		MarketplaceTracker.open_and_track(DclUrls.marketplace_browse(marketplace_section))
	else:
		Global.open_credits.emit("MARKETPLACE_CTA")
