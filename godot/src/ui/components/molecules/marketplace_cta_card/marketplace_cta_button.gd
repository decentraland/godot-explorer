class_name MarketplaceCtaCard
extends Button

@export var marketplace_section: String = "wearables"
@export var credits_balance: int = 0:
	set(value):
		credits_balance = value
		_update_text()

var _selected_price: int = -1


func _ready():
	pressed.connect(_on_pressed)
	_update_text()


## Called when a suggestion card is selected. Pass price=-1 to clear selection.
func update_selection(price: int):
	_selected_price = price
	_update_text()


func _update_text():
	if credits_balance <= 0:
		text = "GET CREDITS"
	elif _selected_price >= 0 and credits_balance < _selected_price:
		text = "GET CREDITS"
	else:
		text = "GO TO MARKETPLACE"


func _on_pressed():
	var can_afford = credits_balance > 0
	if _selected_price >= 0:
		can_afford = credits_balance >= _selected_price
	if can_afford:
		Global.open_url(DclUrls.marketplace() + "/browse?section=" + marketplace_section)
	else:
		Global.open_credits.emit()
