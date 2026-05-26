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


func _ready():
	label_credits.text = str(credits)
	texture_rect.texture = texture
	button_price.pressed.connect(_on_button_price_pressed)
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
	if not Global.get_config().iap_terms_accepted:
		_show_terms_then_purchase()
		return
	Iap.purchase(product_id)


func _show_terms_then_purchase() -> void:
	button_price.disabled = true
	await Global.modal_manager.async_show_iap_terms_modal()
	if not Global.modal_manager.iap_terms_accepted.is_connected(_on_iap_terms_accepted):
		Global.modal_manager.iap_terms_accepted.connect(_on_iap_terms_accepted, CONNECT_ONE_SHOT)
	# Re-enable the button once the modal is gone (accept or cancel).
	var modal = Global.modal_manager.current_modal
	if modal and not modal.tree_exited.is_connected(_on_terms_modal_exited):
		modal.tree_exited.connect(_on_terms_modal_exited, CONNECT_ONE_SHOT)


func _on_iap_terms_accepted() -> void:
	Iap.purchase(product_id)


func _on_terms_modal_exited() -> void:
	button_price.disabled = false
	# If the user cancelled, the one-shot is still connected — clean it up
	# so a later accept on a different item doesn't trigger this product.
	if Global.modal_manager.iap_terms_accepted.is_connected(_on_iap_terms_accepted):
		Global.modal_manager.iap_terms_accepted.disconnect(_on_iap_terms_accepted)
