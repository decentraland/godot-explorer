class_name IapManager
extends Node

# Wrapper around the iOS-only DclStoreKit GDExtension class.
#
# Hides platform plumbing: callers can use this on any platform; on non-iOS
# `is_available()` returns false and methods become no-ops.
#
# IMPORTANT: credits are consumable products. We MUST NOT call
# finish_transaction until backend validation succeeds. Without finish(),
# StoreKit re-delivers the transaction at every app launch — that's the
# safety net while the backend isn't wired yet.

signal products_ready(products: Array)
signal products_load_failed(error: String)
signal purchase_completed(product_id: String, credits: int)
signal purchase_failed(product_id: String, reason: String)
signal purchase_cancelled(product_id: String)
signal purchase_pending(product_id: String)

# Credit packs. Today we only ship the local/dev catalog (served by
# `godot/ios/LocalStoreKit.storekit` via the Xcode scheme). Real ASC product
# IDs will live alongside these once production wiring is added.
const PRODUCT_IDS: PackedStringArray = [
	"local_credits_10",
	"local_credits_100",
]

const _CREDITS_BY_PRODUCT := {
	"local_credits_10": 10,
	"local_credits_100": 100,
}

var _store_kit = null
var _products: Array = []


func _ready() -> void:
	if not ClassDB.class_exists("DclStoreKit"):
		print("[IAP] DclStoreKit not registered (expected on non-iOS platforms)")
		return
	_store_kit = ClassDB.instantiate("DclStoreKit")
	if _store_kit == null:
		printerr("[IAP] failed to instantiate DclStoreKit")
		return

	_store_kit.products_loaded.connect(_on_products_loaded)
	_store_kit.products_load_failed.connect(_on_products_load_failed)
	_store_kit.purchase_completed.connect(_on_purchase_completed)
	_store_kit.purchase_failed.connect(_on_purchase_failed)
	_store_kit.purchase_cancelled.connect(_on_purchase_cancelled)
	_store_kit.purchase_pending.connect(_on_purchase_pending)
	_store_kit.transaction_updated.connect(_on_transaction_updated)

	print("[IAP] starting StoreKit listener; can_make_payments=", _store_kit.can_make_payments())
	_store_kit.start_listening()
	_store_kit.load_products(PRODUCT_IDS)


func is_available() -> bool:
	return _store_kit != null


func get_products() -> Array:
	return _products


func purchase(product_id: String) -> void:
	if _store_kit == null:
		print("[IAP] not available; ignoring purchase(", product_id, ")")
		return
	_store_kit.purchase(product_id)


func _on_products_loaded(json: String) -> void:
	var parsed = JSON.parse_string(json)
	if parsed is Array:
		_products = parsed
	else:
		_products = []
	print("[IAP] products_loaded: ", _products.size(), " products")
	products_ready.emit(_products)


func _on_products_load_failed(error: String) -> void:
	printerr("[IAP] products_load_failed: ", error)
	products_load_failed.emit(error)


func _on_purchase_completed(json: String) -> void:
	var tx = JSON.parse_string(json)
	if not (tx is Dictionary):
		printerr("[IAP] purchase_completed: malformed JSON: ", json)
		return
	print("[IAP] purchase_completed: ", tx)
	_handle_verified_transaction(tx)


func _on_purchase_failed(product_id: String, reason: String) -> void:
	printerr("[IAP] purchase_failed: ", product_id, " - ", reason)
	purchase_failed.emit(product_id, reason)


func _on_purchase_cancelled(product_id: String) -> void:
	print("[IAP] purchase_cancelled: ", product_id)
	purchase_cancelled.emit(product_id)


func _on_purchase_pending(product_id: String) -> void:
	print("[IAP] purchase_pending: ", product_id)
	purchase_pending.emit(product_id)


func _on_transaction_updated(json: String) -> void:
	# Re-delivered or async-arrived transaction (e.g. crash mid-purchase,
	# Ask-to-Buy approval on another device). Same handling as a fresh purchase.
	var tx = JSON.parse_string(json)
	if not (tx is Dictionary):
		printerr("[IAP] transaction_updated: malformed JSON: ", json)
		return
	print("[IAP] transaction_updated: ", tx)
	_handle_verified_transaction(tx)


func _handle_verified_transaction(tx: Dictionary) -> void:
	var product_id := str(tx.get("productId", ""))
	var tx_id := str(tx.get("id", ""))
	if product_id.is_empty() or tx_id.is_empty():
		printerr("[IAP] verified tx missing productId/id: ", tx)
		return

	# TODO(iap-backend): replace with real validation. Should POST
	# tx.jwsRepresentation to a server endpoint that verifies it against
	# Apple's public key, credits the user's account, and returns success.
	# Until that exists, we trust client-side and grant locally — INSECURE
	# in production.
	if not _validate_with_backend(tx):
		printerr(
			"[IAP] backend validation rejected tx ",
			tx_id,
			"; not finishing — StoreKit will redeliver"
		)
		return

	var credits: int = _CREDITS_BY_PRODUCT.get(product_id, 0)
	if credits <= 0:
		printerr("[IAP] unknown product, no credits granted: ", product_id)
		_store_kit.finish_transaction(tx_id)
		return

	_grant_credits_locally(credits)
	_store_kit.finish_transaction(tx_id)
	purchase_completed.emit(product_id, credits)


func _validate_with_backend(_tx: Dictionary) -> bool:
	# TODO(iap-backend): real validation. Returning true unconditionally
	# means a malicious client could fake purchases. Acceptable only for
	# dev/sandbox builds.
	return true


func _grant_credits_locally(credits: int) -> void:
	# TODO: persist to local storage and surface in user balance UI.
	print("[IAP] granted ", credits, " credits locally (no persistence yet)")
