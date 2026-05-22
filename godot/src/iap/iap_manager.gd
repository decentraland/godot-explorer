class_name IapManager
extends Node

# Frontend for the IAP purchase flow. Talks to the Swift `DclStoreKit` class
# through the typed `DclStoreKitPlugin` Rust wrapper — no `ClassDB` plumbing
# here; on non-iOS the wrapper's `is_available()` returns false and every
# method is a no-op.
#
# The backend is currently SIMULATED locally — `_async_validate_with_backend`
# returns OK after a short delay and credits live only in memory (no disk
# persistence, balance resets on relaunch). The async/tri-state-outcome
# shape mirrors the real flow so swapping in a real HTTP call later is a
# localized change.
#
# Owns the global purchase overlay (full-screen blocking spinner). The overlay
# is shown the moment a purchase is initiated and stays up until the flow
# resolves — including the time the app is backgrounded for the StoreKit
# sheet. A 15s timeout guarantees we never lock the UI if a signal goes
# missing.

signal products_ready(products: Array)
signal products_load_failed(error: String)
signal purchase_completed(product_id: String, credits: int)
signal purchase_failed(product_id: String, reason: String)
signal purchase_cancelled(product_id: String)
signal purchase_pending(product_id: String)
signal balance_changed(new_balance: int)
signal transaction_history_updated

# Credit packs. Today we only ship the local/dev catalog (served by
# `godot/ios/LocalStoreKit.storekit` via the Xcode scheme). Real ASC product
# IDs will live alongside these once production wiring is added.
const PRODUCT_IDS: PackedStringArray = [
	"local_credits_10",
	"local_credits_50",
	"local_credits_100",
]

const _CREDITS_BY_PRODUCT := {
	"local_credits_10": 10,
	"local_credits_50": 50,
	"local_credits_100": 100,
}

# Synthetic latency for the simulated backend round-trip — long enough that
# the purchase overlay actually flashes during testing.
const _SIMULATED_VALIDATION_DELAY_SEC := 0.5

# Bound how long the purchase overlay stays up. StoreKit prompt + validation
# should land well inside this; past it we assume something stuck (network
# drop, redelivery loop, missing signal) and let the user retry.
const _PURCHASE_OVERLAY_TIMEOUT_SEC := 15.0

const _OVERLAY_SCENE_PATH := "res://src/ui/components/organisms/iap_purchase_overlay/iap_purchase_overlay.tscn"
const _SUCCESS_MODAL_SCENE_PATH := "res://src/ui/components/organisms/iap_purchase_success_modal/iap_purchase_success_modal.tscn"

# Validation outcomes for `_async_validate_with_backend`:
# OK — credits granted (or already granted, idempotent), finish the tx.
# REJECTED — backend refused (forged JWS, unknown product). Finish to stop
#            StoreKit's redelivery loop; we can't recover.
# RETRY — backend unreachable / transient. Do NOT finish; StoreKit will
#         redeliver on next app launch.
const _OUTCOME_OK := 0
const _OUTCOME_REJECTED := 1
const _OUTCOME_RETRY := 2

# TODO: replace with backend/endpoint query
const _MAX_CREDITS := 115

var _store_kit := DclStoreKitPlugin.new()
var _store_kit_available: bool = false
var _products: Array = []
# In-memory only; resets on relaunch.
var _balance: int = 0
# Tx-id dedup. Apple delivers the same transaction twice on a fresh purchase
# (once via `purchaseCompleted`, once via the `Transaction.updates` listener
# that picks up any unfinished tx). The real backend dedupes by tx id server
# side; we mirror that here. Cleared on relaunch like `_balance`.
var _seen_tx_ids: Dictionary = {}
# TODO: replace with backend endpoint query to get persistent transaction history
var _transaction_history: Array = []
# Bumped on each purchase start AND each overlay hide so stale SceneTreeTimer
# timeouts (which can't be cancelled) become no-ops.
var _overlay_token: int = 0
var _overlay: CanvasLayer = null
var _purchase_in_flight: bool = false


func _ready() -> void:
	# is_available() lazily instantiates the Swift class and wires the
	# Rust-side signal forwarders on the first call.
	if not _store_kit.is_available():
		print("[IAP] DclStoreKit not registered (expected on non-iOS platforms)")
		return
	_store_kit_available = true

	# StoreKit signals arrive from a Swift Task (background thread). Using
	# CONNECT_DEFERRED ensures callbacks run on the main thread, which is
	# required for scene-tree operations (add_child, emit_signal, etc.).
	_store_kit.products_loaded.connect(_on_products_loaded, CONNECT_DEFERRED)
	_store_kit.products_load_failed.connect(_on_products_load_failed, CONNECT_DEFERRED)
	_store_kit.purchase_completed.connect(_on_purchase_completed, CONNECT_DEFERRED)
	_store_kit.purchase_failed.connect(_on_purchase_failed, CONNECT_DEFERRED)
	_store_kit.purchase_cancelled.connect(_on_purchase_cancelled, CONNECT_DEFERRED)
	_store_kit.purchase_pending.connect(_on_purchase_pending, CONNECT_DEFERRED)
	_store_kit.transaction_updated.connect(_on_transaction_updated, CONNECT_DEFERRED)

	print("[IAP] starting StoreKit listener; can_make_payments=", _store_kit.can_make_payments())
	_store_kit.start_listening()
	_store_kit.load_products(PRODUCT_IDS)

	# Global.player_identity is created during Global._ready (earlier in the
	# autoload chain) but added to the tree via call_deferred — defer so the
	# node is fully wired by the time we connect.
	_connect_wallet_signals.call_deferred()


func is_available() -> bool:
	# TODO: remove editor override — forces IAP UI visible for desktop testing
	if OS.has_feature("editor"):
		return true
	return _store_kit_available


func get_products() -> Array:
	# TODO: remove editor override — returns mock products for desktop testing
	if OS.has_feature("editor") and _products.is_empty():
		return [
			{"id": "local_credits_10", "displayName": "10 Credits", "displayPrice": "$0.99"},
			{"id": "local_credits_50", "displayName": "50 Credits", "displayPrice": "$3.99"},
			{"id": "local_credits_100", "displayName": "100 Credits", "displayPrice": "$6.99"},
		]
	return _products


func get_balance() -> int:
	return _balance


func get_transaction_history() -> Array:
	return _transaction_history


func purchase(product_id: String) -> void:
	# TODO: remove — simulates purchase flow in the editor for testing
	if not _store_kit_available and OS.has_feature("editor"):
		_async_simulate_editor_purchase(product_id)
		return
	if not _store_kit_available:
		print("[IAP] not available; ignoring purchase(", product_id, ")")
		return
	if _purchase_in_flight:
		print("[IAP] purchase already in flight; ignoring re-entry for ", product_id)
		Global.modal_manager.async_show_purchase_in_flight_modal()
		return
	var wallet := _wallet_address()
	if wallet.is_empty():
		printerr("[IAP] cannot purchase without wallet (sign in first)")
		purchase_failed.emit(product_id, "not signed in")
		return
	var credits_to_add: int = _CREDITS_BY_PRODUCT.get(product_id, 0)
	if _balance + credits_to_add > _MAX_CREDITS:
		printerr(
			"[IAP] credit limit reached: ", _balance, " + ", credits_to_add, " > ", _MAX_CREDITS
		)
		Global.modal_manager.async_show_credit_limit_modal()
		return
	_purchase_in_flight = true
	_show_overlay()
	_store_kit.purchase(product_id, wallet)


# TODO: remove — editor-only simulated purchase for desktop testing
func _async_simulate_editor_purchase(product_id: String) -> void:
	var credits: int = _CREDITS_BY_PRODUCT.get(product_id, 0)
	if _balance + credits > _MAX_CREDITS:
		Global.modal_manager.async_show_credit_limit_modal()
		return
	_purchase_in_flight = true
	_show_overlay()
	await get_tree().create_timer(1.0).timeout
	_finish_purchase_flow()
	_balance += credits
	balance_changed.emit(_balance)
	_show_success_modal(credits)
	_record_transaction(credits, false)
	purchase_completed.emit(product_id, credits)


func _record_transaction(credits: int, is_refund: bool) -> void:
	var now = Time.get_datetime_dict_from_system()
	(
		_transaction_history
		. append(
			{
				"credits": credits,
				"is_refund": is_refund,
				"timestamp": "%04d.%02d.%02d" % [now.year, now.month, now.day],
			}
		)
	)
	transaction_history_updated.emit()


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
		_finish_purchase_flow()
		return
	print("[IAP] purchase_completed: ", tx)
	_async_handle_verified_transaction(tx)


func _on_purchase_failed(product_id: String, reason: String) -> void:
	printerr("[IAP] purchase_failed: ", product_id, " - ", reason)
	_finish_purchase_flow()
	Global.modal_manager.async_show_purchase_failed_modal()
	purchase_failed.emit(product_id, reason)


func _on_purchase_cancelled(product_id: String) -> void:
	print("[IAP] purchase_cancelled: ", product_id)
	_finish_purchase_flow()
	Global.modal_manager.async_show_purchase_failed_modal()
	purchase_cancelled.emit(product_id)


func _on_purchase_pending(product_id: String) -> void:
	# Ask-to-Buy / SCA — StoreKit is waiting for an out-of-band approval that
	# may take hours. Drop the overlay so the user isn't stuck staring at a
	# spinner; transaction_updated will fire later if/when it resolves.
	print("[IAP] purchase_pending: ", product_id)
	_finish_purchase_flow()
	purchase_pending.emit(product_id)


func _on_transaction_updated(json: String) -> void:
	# Re-delivered or async-arrived transaction (e.g. crash mid-purchase,
	# Ask-to-Buy approval on another device). Same handling as a fresh purchase.
	var tx = JSON.parse_string(json)
	if not (tx is Dictionary):
		printerr("[IAP] transaction_updated: malformed JSON: ", json)
		return
	print("[IAP] transaction_updated: ", tx)
	_async_handle_verified_transaction(tx)


# gdlint:ignore = async-function-name
func _async_handle_verified_transaction(tx: Dictionary) -> void:
	var product_id := str(tx.get("productId", ""))
	var tx_id := str(tx.get("id", ""))
	if product_id.is_empty() or tx_id.is_empty():
		printerr("[IAP] verified tx missing productId/id: ", tx)
		_finish_purchase_flow()
		return
	if _seen_tx_ids.has(tx_id):
		# Duplicate emission (purchaseCompleted + Transaction.updates for the
		# same fresh tx). Original invocation owns the overlay lifecycle — bail
		# without touching it.
		print("[IAP] tx ", tx_id, " already in-flight/processed; skipping duplicate")
		return
	_seen_tx_ids[tx_id] = true

	var outcome: int = await _async_validate_with_backend(tx)
	match outcome:
		_OUTCOME_OK:
			var credits: int = _CREDITS_BY_PRODUCT.get(product_id, 0)
			_store_kit.finish_transaction(tx_id)
			_finish_purchase_flow()
			_show_success_modal(credits)
			_record_transaction(credits, false)
			purchase_completed.emit(product_id, credits)
		_OUTCOME_REJECTED:
			# Sim rejected (unknown product). Finishing breaks the redelivery
			# loop — retrying won't help.
			printerr("[IAP] tx ", tx_id, " rejected; finishing")
			_store_kit.finish_transaction(tx_id)
			_finish_purchase_flow()
			purchase_failed.emit(product_id, "rejected by backend")
		_OUTCOME_RETRY:
			# Transient (no wallet, etc). Don't finish: StoreKit will re-deliver.
			# Unmark so the next delivery gets another chance.
			_seen_tx_ids.erase(tx_id)
			printerr("[IAP] tx ", tx_id, " transient; will retry on next launch")
			_finish_purchase_flow()
			purchase_failed.emit(product_id, "network error, retry on next launch")


# gdlint:ignore = async-function-name
func _async_validate_with_backend(tx: Dictionary) -> int:
	# SIMULATED backend. Shape mirrors the real flow (async, tri-state
	# outcome) so the real HTTP call is a localized swap later. The JWS /
	# wallet checks here are sanity-only — nothing on this side is actually
	# verifying signatures or persisting state.
	var jws := str(tx.get("jwsRepresentation", ""))
	if jws.is_empty():
		printerr("[IAP] missing JWS")
		return _OUTCOME_REJECTED
	var wallet := _wallet_address()
	if wallet.is_empty():
		printerr("[IAP] no wallet address yet; deferring grant")
		return _OUTCOME_RETRY

	var product_id := str(tx.get("productId", ""))
	var credits: int = _CREDITS_BY_PRODUCT.get(product_id, 0)
	if credits <= 0:
		printerr("[IAP] sim rejected: unknown product ", product_id)
		return _OUTCOME_REJECTED

	await get_tree().create_timer(_SIMULATED_VALIDATION_DELAY_SEC).timeout

	_balance += credits
	balance_changed.emit(_balance)
	print("[IAP] sim granted ", credits, " for ", product_id, " balance=", _balance)
	return _OUTCOME_OK


func _connect_wallet_signals() -> void:
	if Global.player_identity == null:
		return
	if not Global.player_identity.wallet_connected.is_connected(_on_wallet_connected):
		Global.player_identity.wallet_connected.connect(_on_wallet_connected)
	# If the session was restored synchronously the wallet is already there
	# and the signal won't fire — fetch now to cover that case.
	if not _wallet_address().is_empty():
		_async_fetch_balance()


func _on_wallet_connected(_address: String, _chain_id: int, _is_guest: bool) -> void:
	_async_fetch_balance()


# gdlint:ignore = async-function-name
func _async_fetch_balance() -> void:
	# SIMULATED. In prod this would GET /balance/<wallet> and reconcile the
	# server-side balance into `_balance`. With no backend or persistence
	# there's nothing to reconcile against — just re-emit the current value
	# after a synthetic delay so listeners refresh.
	var wallet := _wallet_address()
	if wallet.is_empty():
		return
	await get_tree().create_timer(_SIMULATED_VALIDATION_DELAY_SEC).timeout
	if _wallet_address() != wallet:
		return
	balance_changed.emit(_balance)


func _wallet_address() -> String:
	if Global.player_identity == null:
		return ""
	return Global.player_identity.get_address_str()


func _show_overlay() -> void:
	if _overlay == null or not is_instance_valid(_overlay):
		var scene := load(_OVERLAY_SCENE_PATH) as PackedScene
		if scene == null:
			printerr("[IAP] purchase overlay scene missing at ", _OVERLAY_SCENE_PATH)
			return
		_overlay = scene.instantiate()
		get_tree().root.add_child(_overlay)
	_overlay.visible = true
	_overlay_token += 1
	var token: int = _overlay_token
	get_tree().create_timer(_PURCHASE_OVERLAY_TIMEOUT_SEC).timeout.connect(
		_on_overlay_timeout.bind(token)
	)


func _hide_overlay() -> void:
	# Bump the token so any pending SceneTreeTimer timeout becomes a no-op.
	_overlay_token += 1
	if _overlay != null and is_instance_valid(_overlay):
		_overlay.queue_free()
	_overlay = null


func _on_overlay_timeout(token: int) -> void:
	if token != _overlay_token:
		return
	printerr("[IAP] purchase overlay timed out after ", _PURCHASE_OVERLAY_TIMEOUT_SEC, "s")
	_hide_overlay()


func _finish_purchase_flow() -> void:
	_purchase_in_flight = false
	_hide_overlay()


func _show_success_modal(credits: int) -> void:
	var scene := load(_SUCCESS_MODAL_SCENE_PATH) as PackedScene
	if scene == null:
		printerr("[IAP] success modal scene missing at ", _SUCCESS_MODAL_SCENE_PATH)
		return
	var modal: CanvasLayer = scene.instantiate()
	# setup() before add_child so the credits value is in place by the time
	# the modal's _ready runs.
	if modal.has_method("setup"):
		modal.call("setup", credits)
	get_tree().root.add_child(modal)
