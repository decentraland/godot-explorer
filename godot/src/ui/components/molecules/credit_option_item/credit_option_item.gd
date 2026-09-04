@tool
extends PanelContainer

@export var texture: Texture = null:
	set(value):
		texture = value
		if is_node_ready():
			texture_rect.texture = texture

@export var credits: int = 10:
	set(value):
		credits = value
		if is_node_ready():
			label_credits.text = str(credits)

# Binds this item to a StoreKit product. Without a matching product loaded the
# item hides itself — Android/desktop has no products, and unknown IDs never
# resolve a localized price.
@export var product_id: String = "":
	set(value):
		product_id = value
		if is_node_ready():
			_refresh_from_iap()

@onready var texture_rect: TextureRect = $MarginContainer/HBoxContainer/TextureRect
@onready var label_credits: Label = $MarginContainer/HBoxContainer/Label_Credits
@onready var button_price: Button = $MarginContainer/HBoxContainer/Button_Price
@onready var button_card: Button = $Button_Card


func _ready():
	label_credits.text = str(credits)
	texture_rect.texture = texture
	button_price.pressed.connect(_on_button_price_pressed)
	# The whole card buys the pack, not just the price pill. `Button_Card` sits behind
	# the content (the PanelContainer sizes every child to its full rect) and everything
	# in front of it ignores the mouse, so a tap anywhere lands here — except on
	# `Button_Price`, which stops the event so this doesn't fire a second time.
	button_card.pressed.connect(_on_button_price_pressed)
	if Engine.is_editor_hint():
		return
	Iap.products_ready.connect(_on_iap_products_ready)
	_refresh_from_iap()


func _on_iap_products_ready(_products: Array) -> void:
	_refresh_from_iap()


func _refresh_from_iap() -> void:
	if Engine.is_editor_hint():
		return
	if product_id.is_empty():
		hide()
		return
	var product := _find_product(product_id)
	if product.is_empty():
		hide()
		return
	button_price.text = str(product.get("displayPrice", ""))
	show()


func _find_product(pid: String) -> Dictionary:
	for p in Iap.get_products():
		if p is Dictionary and str(p.get("id", "")) == pid:
			return p
	return {}


func _on_button_price_pressed() -> void:
	if product_id.is_empty():
		return
	# The press itself, before the terms gate and before StoreKit. A press with no
	# matching "Request Result" for context iap_purchase is the signal that the flow
	# died on the way — which is exactly what we could not see during App Review.
	if Global.metrics != null:
		var extra := JSON.parse_string(Iap.analytics_context()) as Dictionary
		extra["product_id"] = product_id
		extra["terms_accepted"] = Iap.are_terms_accepted()
		Global.metrics.track_click_button("BUY_PACK", "CREDITS_SHOP", JSON.stringify(extra))
	if not Iap.are_terms_accepted():
		_async_show_terms_then_purchase()
		return
	Iap.purchase(product_id)


func _async_show_terms_then_purchase() -> void:
	_set_purchase_enabled(false)
	# Wire the one-shot BEFORE awaiting the modal: connecting after the await
	# leaves a window where an instant confirm could fire iap_terms_accepted
	# before we're listening, dropping the purchase.
	if not Global.modal_manager.iap_terms_accepted.is_connected(_on_iap_terms_accepted):
		Global.modal_manager.iap_terms_accepted.connect(_on_iap_terms_accepted, CONNECT_ONE_SHOT)
	await Global.modal_manager.async_show_iap_terms_modal()
	# Re-enable the button once the modal is gone (accept or cancel).
	var modal = Global.modal_manager.current_modal
	if modal and not modal.tree_exited.is_connected(_on_terms_modal_exited):
		modal.tree_exited.connect(_on_terms_modal_exited, CONNECT_ONE_SHOT)
	else:
		# No modal left to wait on — it closed while we were awaiting, or another one
		# replaced it. Re-enable now: there is nothing else that would, and a card stuck
		# disabled draws in the washed-out red that reads as a broken pressed state.
		_on_terms_modal_exited()


func _on_iap_terms_accepted() -> void:
	Iap.purchase(product_id)


func _on_terms_modal_exited() -> void:
	_set_purchase_enabled(true)
	# If the user cancelled, the one-shot is still connected — clean it up
	# so a later accept on a different item doesn't trigger this product.
	if Global.modal_manager.iap_terms_accepted.is_connected(_on_iap_terms_accepted):
		Global.modal_manager.iap_terms_accepted.disconnect(_on_iap_terms_accepted)


## Both tap targets move together — disabling only the pill would leave the rest of the
## card buying while the terms modal is up.
func _set_purchase_enabled(enabled: bool) -> void:
	button_price.disabled = not enabled
	button_card.disabled = not enabled
