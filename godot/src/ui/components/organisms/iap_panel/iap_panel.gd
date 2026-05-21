class_name IapPanel
extends Control

# Dev panel for triggering and observing IAP purchases. Built fully in code
# to avoid coupling to the main settings .tscn while the feature is in flux.
# Wired to the `Iap` autoload — no per-instance plumbing needed.

const _PANEL_BG := Color(0.05, 0.05, 0.07, 0.92)

var _balance_label: Label
var _status_label: Label
var _products_container: VBoxContainer


func _ready() -> void:
	set_anchors_preset(Control.PRESET_FULL_RECT)
	mouse_filter = Control.MOUSE_FILTER_STOP

	var bg := ColorRect.new()
	bg.color = _PANEL_BG
	bg.set_anchors_preset(Control.PRESET_FULL_RECT)
	bg.mouse_filter = Control.MOUSE_FILTER_STOP
	add_child(bg)

	var margin := MarginContainer.new()
	margin.set_anchors_preset(Control.PRESET_FULL_RECT)
	for side in ["left", "right", "top", "bottom"]:
		margin.add_theme_constant_override("margin_" + side, 32)
	add_child(margin)

	var col := VBoxContainer.new()
	col.add_theme_constant_override("separation", 16)
	margin.add_child(col)

	var header := HBoxContainer.new()
	header.add_theme_constant_override("separation", 12)
	col.add_child(header)

	var title := Label.new()
	title.text = "In-App Purchases (dev)"
	title.add_theme_font_size_override("font_size", 28)
	title.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	header.add_child(title)

	var close_btn := Button.new()
	close_btn.text = "Close"
	close_btn.pressed.connect(_on_close_pressed)
	header.add_child(close_btn)

	_balance_label = Label.new()
	_balance_label.add_theme_font_size_override("font_size", 18)
	col.add_child(_balance_label)

	_status_label = Label.new()
	_status_label.add_theme_font_size_override("font_size", 14)
	_status_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	col.add_child(_status_label)

	var divider := HSeparator.new()
	col.add_child(divider)

	var products_title := Label.new()
	products_title.text = "Products"
	products_title.add_theme_font_size_override("font_size", 20)
	col.add_child(products_title)

	var scroll := ScrollContainer.new()
	scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	col.add_child(scroll)

	_products_container = VBoxContainer.new()
	_products_container.add_theme_constant_override("separation", 8)
	_products_container.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	scroll.add_child(_products_container)

	Iap.products_ready.connect(_on_products_ready)
	Iap.products_load_failed.connect(_on_products_load_failed)
	Iap.purchase_completed.connect(_on_purchase_completed)
	Iap.purchase_failed.connect(_on_purchase_failed)
	Iap.purchase_cancelled.connect(_on_purchase_cancelled)
	Iap.purchase_pending.connect(_on_purchase_pending)
	Iap.balance_changed.connect(_on_balance_changed)

	_refresh_balance(Iap.get_balance())
	_set_status("")
	if not Iap.is_available():
		_set_status("StoreKit not available on this platform.")
		return
	_render_products(Iap.get_products())


func _on_close_pressed() -> void:
	queue_free()


func _set_status(text: String) -> void:
	_status_label.text = text


func _refresh_balance(balance: int) -> void:
	_balance_label.text = "Balance: %d credits" % balance


func _render_products(products: Array) -> void:
	for child in _products_container.get_children():
		child.queue_free()
	if products.is_empty():
		var empty := Label.new()
		empty.text = "No products loaded yet."
		_products_container.add_child(empty)
		return
	for raw in products:
		if not (raw is Dictionary):
			continue
		_products_container.add_child(_build_product_row(raw))


func _build_product_row(product: Dictionary) -> Control:
	var row := PanelContainer.new()
	var inner := HBoxContainer.new()
	inner.add_theme_constant_override("separation", 12)
	row.add_child(inner)

	var info := VBoxContainer.new()
	info.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	info.add_theme_constant_override("separation", 2)
	inner.add_child(info)

	var name_label := Label.new()
	name_label.text = str(product.get("displayName", product.get("id", "?")))
	name_label.add_theme_font_size_override("font_size", 16)
	info.add_child(name_label)

	var desc_label := Label.new()
	desc_label.text = str(product.get("description", ""))
	desc_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	desc_label.add_theme_font_size_override("font_size", 12)
	info.add_child(desc_label)

	var meta_label := Label.new()
	meta_label.text = "id=%s · %s" % [product.get("id", "?"), product.get("displayPrice", "?")]
	meta_label.add_theme_font_size_override("font_size", 11)
	meta_label.modulate = Color(1, 1, 1, 0.7)
	info.add_child(meta_label)

	var buy_btn := Button.new()
	buy_btn.text = "Buy"
	buy_btn.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	var product_id := str(product.get("id", ""))
	buy_btn.pressed.connect(_on_buy_pressed.bind(product_id))
	inner.add_child(buy_btn)

	return row


func _on_buy_pressed(product_id: String) -> void:
	if product_id.is_empty():
		return
	_set_status("Purchasing %s…" % product_id)
	Iap.purchase(product_id)


func _on_products_ready(products: Array) -> void:
	_set_status("Loaded %d products" % products.size())
	_render_products(products)


func _on_products_load_failed(error: String) -> void:
	_set_status("Products load failed: %s" % error)


func _on_purchase_completed(product_id: String, credits: int) -> void:
	_set_status("Purchase OK: %s (+%d credits)" % [product_id, credits])


func _on_purchase_failed(product_id: String, reason: String) -> void:
	_set_status("Purchase failed: %s — %s" % [product_id, reason])


func _on_purchase_cancelled(product_id: String) -> void:
	_set_status("Purchase cancelled: %s" % product_id)


func _on_purchase_pending(product_id: String) -> void:
	_set_status("Purchase pending: %s (Ask-to-Buy / SCA)" % product_id)


func _on_balance_changed(new_balance: int) -> void:
	_refresh_balance(new_balance)
