class_name IapManager
extends Node

# Frontend for the IAP purchase flow. Talks to the Swift `DclStoreKit` class
# through the typed `DclStoreKitPlugin` Rust wrapper — no `ClassDB` plumbing
# here; on non-iOS the wrapper's `is_available()` returns false and every
# method is a no-op.
#
# Owns the global purchase overlay (full-screen blocking spinner). The overlay
# is shown the moment a purchase is initiated and stays up until the flow
# resolves (StoreKit cancel/fail OR backend grant) — including the time the
# app is backgrounded for the StoreKit sheet. A 15s timeout guarantees we
# never lock the UI if a signal goes missing.
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
signal balance_changed(new_balance: int)

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

# Backend that verifies the StoreKit JWS, grants credits server-side, and
# tracks the per-wallet balance. Source of truth for `_balance`.
# LOCAL TUNNEL — revert before committing. Sandbox host:
# "https://iap-sandbox.dclregenesislabs.xyz"
const _BACKEND_URL := "https://engineering-broader-msg-distinguished.trycloudflare.com"
const _BACKEND_TIMEOUT_SEC := 15.0

# Bound how long the purchase overlay stays up. StoreKit prompt + backend
# round-trip should land well inside this; past it we assume something stuck
# (network drop, redelivery loop, missing signal) and let the user retry.
const _PURCHASE_OVERLAY_TIMEOUT_SEC := 15.0

const _OVERLAY_SCENE_PATH := "res://src/iap/iap_purchase_overlay.tscn"
const _SUCCESS_MODAL_SCENE_PATH := "res://src/iap/iap_purchase_success_modal.tscn"

# Validation outcomes for `_async_validate_with_backend`:
# OK — credits granted (or already granted, idempotent), finish the tx.
# REJECTED — backend refused (forged JWS, unknown product). Finish to stop
#            StoreKit's redelivery loop; we can't recover.
# RETRY — backend unreachable / transient. Do NOT finish; StoreKit will
#         redeliver on next app launch.
const _OUTCOME_OK := 0
const _OUTCOME_REJECTED := 1
const _OUTCOME_RETRY := 2

var _store_kit := DclStoreKitPlugin.new()
var _store_kit_available: bool = false
var _products: Array = []
# Mirrors the server-side balance for the signed-in wallet. Updated from
# every successful backend call. Empty until first call returns.
var _balance: int = 0
var _dev_panel: Control = null

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

	# Global.player_identity is created during Global._ready (earlier in the
	# autoload chain) but added to the tree via call_deferred — defer so the
	# node is fully wired by the time we connect.
	_connect_wallet_signals.call_deferred()


func is_available() -> bool:
	return _store_kit_available


func get_products() -> Array:
	return _products


func get_balance() -> int:
	return _balance


func purchase(product_id: String) -> void:
	if not _store_kit_available:
		print("[IAP] not available; ignoring purchase(", product_id, ")")
		return
	if _purchase_in_flight:
		print("[IAP] purchase already in flight; ignoring re-entry for ", product_id)
		return
	var wallet := _wallet_address()
	if wallet.is_empty():
		printerr("[IAP] cannot purchase without wallet (sign in first)")
		purchase_failed.emit(product_id, "not signed in")
		return
	_purchase_in_flight = true
	_show_overlay()
	_store_kit.purchase(product_id, wallet)


func toggle_dev_panel() -> void:
	if _dev_panel != null and is_instance_valid(_dev_panel):
		_dev_panel.queue_free()
		_dev_panel = null
		return
	var scene := load("res://src/ui/iap/iap_panel.tscn") as PackedScene
	if scene == null:
		printerr("[IAP] dev panel scene missing")
		return
	_dev_panel = scene.instantiate()
	get_tree().root.add_child(_dev_panel)


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
	purchase_failed.emit(product_id, reason)


func _on_purchase_cancelled(product_id: String) -> void:
	print("[IAP] purchase_cancelled: ", product_id)
	_finish_purchase_flow()
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

	var outcome: int = await _async_validate_with_backend(tx)
	match outcome:
		_OUTCOME_OK:
			var credits: int = _CREDITS_BY_PRODUCT.get(product_id, 0)
			_store_kit.finish_transaction(tx_id)
			_finish_purchase_flow()
			_show_success_modal(credits)
			purchase_completed.emit(product_id, credits)
		_OUTCOME_REJECTED:
			# Backend rejected (forged/unknown product/etc). Finishing breaks
			# the redelivery loop — retrying won't help.
			printerr("[IAP] tx ", tx_id, " rejected by backend; finishing")
			_store_kit.finish_transaction(tx_id)
			_finish_purchase_flow()
			purchase_failed.emit(product_id, "rejected by backend")
		_OUTCOME_RETRY:
			# Network / 5xx / no wallet. Don't finish: StoreKit will re-deliver.
			printerr("[IAP] tx ", tx_id, " transient; will retry on next launch")
			_finish_purchase_flow()
			purchase_failed.emit(product_id, "network error, retry on next launch")


# gdlint:ignore = async-function-name
func _async_validate_with_backend(tx: Dictionary) -> int:
	var jws := str(tx.get("jwsRepresentation", ""))
	if jws.is_empty():
		printerr("[IAP] missing JWS")
		return _OUTCOME_REJECTED
	var wallet := _wallet_address()
	if wallet.is_empty():
		printerr("[IAP] no wallet address yet; deferring grant")
		return _OUTCOME_RETRY

	var headers: Dictionary = {"Content-Type": "application/json"}
	var body := JSON.stringify({"jws": jws, "walletAddress": wallet})
	var promise: Promise = Global.http_requester.request_json(
		_BACKEND_URL + "/iap/grant", HTTPClient.METHOD_POST, body, headers
	)
	var result = await PromiseUtils.async_awaiter(promise)
	if result is PromiseError:
		printerr("[IAP] backend network error: ", result.get_error())
		return _OUTCOME_RETRY
	if not (result is RequestResponse):
		printerr("[IAP] unexpected result type from http_requester")
		return _OUTCOME_RETRY

	var response: RequestResponse = result
	var code: int = response.status_code()
	if code == 0 or code >= 500:
		printerr("[IAP] backend transient: code=", code)
		return _OUTCOME_RETRY
	if code >= 400:
		printerr("[IAP] backend rejected: code=", code, " body=", response.get_response_as_string())
		return _OUTCOME_REJECTED

	var parsed = response.get_string_response_as_json()
	if parsed is Dictionary:
		var status := str(parsed.get("status", ""))
		var server_balance := int(parsed.get("balance", _balance))
		if server_balance != _balance:
			_balance = server_balance
			balance_changed.emit(_balance)
		print("[IAP] backend ", status, " balance=", server_balance)
	return _OUTCOME_OK


# gdlint:ignore = async-function-name
func _async_fetch_balance() -> void:
	var wallet := _wallet_address()
	if wallet.is_empty():
		return
	var promise: Promise = Global.http_requester.request_json(
		"%s/balance/%s" % [_BACKEND_URL, wallet], HTTPClient.METHOD_GET, "", {}
	)
	var result = await PromiseUtils.async_awaiter(promise)
	# Wallet may have changed (logout/relogin) while the fetch was in flight.
	# Drop the response — a newer fetch is or will be running for the new wallet.
	if _wallet_address() != wallet:
		return
	if result is PromiseError or not (result is RequestResponse):
		return
	var response: RequestResponse = result
	if response.status_code() != 200:
		return
	var parsed = response.get_string_response_as_json()
	if not (parsed is Dictionary):
		return
	var server_balance := int(parsed.get("credits", 0))
	if server_balance != _balance:
		_balance = server_balance
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
	_finish_purchase_flow()


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
