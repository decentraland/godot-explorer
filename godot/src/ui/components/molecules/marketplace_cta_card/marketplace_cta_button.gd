class_name MarketplaceCtaCard
extends Button

@export var marketplace_section: String = "wearables"
@export var has_credits: bool = false:
	set(value):
		has_credits = value
		_update_text()


func _ready():
	pressed.connect(_on_pressed)
	_update_text()


func _update_text():
	text = "GO TO MARKETPLACE" if has_credits else "GET CREDITS"


func _on_pressed():
	if has_credits:
		Global.open_url(DclUrls.marketplace() + "/browse?section=" + marketplace_section)
	# else: placeholder for StoreKit purchase flow (#2115)
