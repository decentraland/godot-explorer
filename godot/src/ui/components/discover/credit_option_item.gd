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
	# IapManager owns the global purchase overlay (full-screen blocking
	# spinner) and the in-flight guard, so re-taps are safely ignored there.
	Iap.purchase(product_id)
