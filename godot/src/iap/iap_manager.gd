class_name IapManager
extends Node

# Frontend for the IAP purchase flow. Talks to the Swift `DclStoreKit` class
# through the typed `DclStoreKitPlugin` Rust wrapper — no `ClassDB` plumbing
# here; on non-iOS the wrapper's `is_available()` returns false and every
# method is a no-op.
#
# Talks to the IAP backend (mobile-bff) over DCL signed-fetch (ADR-44):
#   - POST /iap/apple/quote  : pre-purchase gate (daily + total caps) BEFORE
#                              StoreKit charges (a consumable can't be un-charged).
#   - POST /iap/apple/verify : verifies the StoreKit JWS, binds it to the wallet,
#                              and credits idempotently. Tri-state outcome
#                              (OK/REJECTED/RETRY) drives StoreKit finish/redelivery.
#   - GET  /iap/balance      : reconciles the server balance into the local cache.
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

# Credit packs. These IDs must exist as consumable products in App Store
# Connect — StoreKit resolves them against Sandbox (TestFlight / sandbox
# account) and Production. No local/mock catalog: an ID not configured in
# ASC simply returns no product.
const PRODUCT_IDS: PackedStringArray = [
	"credits_10",
	"credits_20",
	"credits_50",
]

const _CREDITS_BY_PRODUCT := {
	"credits_10": 10,
	"credits_20": 20,
	"credits_50": 50,
}

# Bound how long the purchase overlay stays up. StoreKit prompt + validation
# should land well inside this; past it we assume something stuck (network
# drop, redelivery loop, missing signal) and let the user retry.
const _PURCHASE_OVERLAY_TIMEOUT_SEC := 15.0

const _OVERLAY_SCENE_PATH := "res://src/ui/components/organisms/iap_purchase_overlay/iap_purchase_overlay.tscn"
const _SUCCESS_MODAL_SCENE_PATH := "res://src/ui/components/organisms/iap_purchase_success_modal/iap_purchase_success_modal.tscn"

# IAP backend base URL. Points at the dedicated sandbox instance used to
# validate the StoreKit + App Store Server Notifications flow end-to-end. All
# calls go through Global.async_signed_fetch (DCL signed-fetch / ADR-44), and
# the host is not part of the signed payload, so this can be repointed at the
# production mobile-bff host without touching the signing path.
const _IAP_BACKEND_BASE_URL := "https://iap-sandbox.dclregenesislabs.xyz"

# Validation outcomes for `_async_validate_with_backend`:
# OK — credits granted (or already granted, idempotent), finish the tx.
# REJECTED — backend refused (forged JWS, unknown product). Finish to stop
#            StoreKit's redelivery loop; we can't recover.
# RETRY — backend unreachable / transient. Do NOT finish; StoreKit will
#         redeliver on next app launch.
const _OUTCOME_OK := 0
const _OUTCOME_REJECTED := 1
const _OUTCOME_RETRY := 2

# Total + daily credit caps are enforced server-side by the IAP backend
# (POST /iap/apple/quote). The client no longer holds these limits.

# Gates all IAP behavior. Default false — must be turned on via the
# `decentraland://open?iap_enabled=true` launch deeplink. Until enable() is
# called, _ready is a no-op: no signal subscriptions, no product load,
# is_available() returns false (which hides the credits UI entry point).
var enabled: bool = false

var _store_kit := DclStoreKitPlugin.new()
var _store_kit_available: bool = false
var _products: Array = []
# Local cache of the server-authoritative balance, reconciled from quote/verify/
# balance responses. Server is the source of truth.
var _balance: int = 0
# Tx-id dedup. Apple delivers the same transaction twice on a fresh purchase
# (once via `purchaseCompleted`, once via the `Transaction.updates` listener
# that picks up any unfinished tx). The real backend dedupes by tx id server
# side; we mirror that here. Cleared on relaunch like `_balance`.
var _seen_tx_ids: Dictionary = {}
# Local cache of the server's transaction history (GET /iap/history). Populated
# on wallet connect and when the history view opens; also gets the just-bought
# entry appended optimistically on a successful purchase.
var _transaction_history: Array = []
# Bumped on each purchase start AND each overlay hide so stale SceneTreeTimer
# timeouts (which can't be cancelled) become no-ops.
var _overlay_token: int = 0
var _overlay: CanvasLayer = null
var _purchase_in_flight: bool = false


func _ready() -> void:
	# IAP starts disabled. DeepLinkRouter calls enable() when the launch
	# deeplink carries iap_enabled=true.
	pass


# Idempotent. Called by DeepLinkRouter when iap_enabled=true is present in
# the launch deeplink. Performs the StoreKit wiring originally in _ready.
func enable() -> void:
	if enabled:
		return
	enabled = true

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
	if not enabled:
		return false
	return _store_kit_available


func get_products() -> Array:
	return _products


func get_balance() -> int:
	return _balance


func get_transaction_history() -> Array:
	return _transaction_history


# IAP terms are a per-wallet legal consent: a flag set by account A must never
# count as consent for account B on the same device. We key acceptance by the
# accepting wallet (lowercased) rather than a device-wide bool.
func are_terms_accepted() -> bool:
	var wallet := _wallet_address().to_lower()
	if wallet.is_empty():
		return false
	return Global.get_config().iap_terms_accepted_wallet == wallet


func accept_terms() -> void:
	var wallet := _wallet_address().to_lower()
	if wallet.is_empty():
		return
	var config := Global.get_config()
	config.iap_terms_accepted_wallet = wallet
	config.save_to_settings_file()


func purchase(product_id: String) -> void:
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
	# Take the overlay + in-flight lock up-front so the button can't be
	# re-tapped while the quote round-trips. The quote is the server-
	# authoritative pre-purchase gate; only on `allowed` do we hand off to
	# StoreKit. _async_begin_purchase owns the flow from here.
	_purchase_in_flight = true
	_show_overlay()
	_async_begin_purchase(product_id, wallet)


# gdlint:ignore = async-function-name
func _async_begin_purchase(product_id: String, wallet: String) -> void:
	# Pre-purchase gate. Enforces the daily + total caps BEFORE StoreKit charges
	# (a consumable cannot be un-charged, so limits must run before the charge).
	var body := JSON.stringify({"productId": product_id})
	var envelope = await _async_signed_iap("/iap/apple/quote", HTTPClient.METHOD_POST, body)
	if envelope == null or not envelope.get("ok", false):
		# Transport/auth failure — fail closed (do NOT let StoreKit charge).
		printerr("[IAP] quote failed; aborting purchase of ", product_id)
		_finish_purchase_flow()
		Global.modal_manager.async_show_purchase_failed_modal()
		purchase_failed.emit(product_id, "quote failed")
		return
	var data = envelope.get("data", {})
	if not (data is Dictionary):
		data = {}
	# Keep the cached balance fresh from the quote.
	if data.has("balance"):
		_balance = int(data.get("balance"))
		balance_changed.emit(_balance)
	if not data.get("allowed", false):
		var reason := str(data.get("reason", ""))
		print("[IAP] quote denied for ", product_id, " reason=", reason)
		_finish_purchase_flow()
		match reason:
			"total_limit":
				Global.modal_manager.async_show_credit_limit_total_modal()
			"daily_limit":
				Global.modal_manager.async_show_credit_limit_daily_modal()
			_:
				Global.modal_manager.async_show_purchase_failed_modal()
		purchase_failed.emit(product_id, "not allowed: " + reason)
		return
	# Gate passed — initiate the real StoreKit purchase. Overlay stays up until
	# the purchase resolves via the StoreKit signal handlers.
	_store_kit.purchase(product_id, wallet)


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
	# Real backend validation. POSTs the StoreKit JWS to the IAP backend, which
	# verifies the Apple signature, binds appAccountToken<->wallet, and credits
	# idempotently. The tri-state mirrors StoreKit's redelivery contract:
	#   OK       -> credited (or already credited) -> finish the tx.
	#   REJECTED -> backend refused permanently (bad signature, wallet mismatch,
	#               unknown product, wrong environment) -> finish to stop the
	#               redelivery loop; retrying won't help.
	#   RETRY    -> transport/transient error -> do NOT finish; StoreKit
	#               redelivers on next launch.
	var jws := str(tx.get("jwsRepresentation", ""))
	if jws.is_empty():
		printerr("[IAP] missing JWS")
		return _OUTCOME_REJECTED
	var wallet := _wallet_address()
	if wallet.is_empty():
		printerr("[IAP] no wallet address yet; deferring grant")
		return _OUTCOME_RETRY

	var body := JSON.stringify({"jwsRepresentation": jws})
	var envelope = await _async_signed_iap("/iap/apple/verify", HTTPClient.METHOD_POST, body)
	# null == transport error (non-2xx / timeout) -> transient -> RETRY.
	if envelope == null:
		printerr("[IAP] verify transport error; will retry on next launch")
		return _OUTCOME_RETRY
	# Permanent rejections come back as HTTP 200 with ok:false (see
	# verify-handler) so they are NEVER mistaken for a transient failure.
	if not envelope.get("ok", false):
		printerr(
			"[IAP] verify rejected: ", envelope.get("code", ""), " ", envelope.get("error", "")
		)
		return _OUTCOME_REJECTED

	var data = envelope.get("data", {})
	if data is Dictionary and data.has("balance"):
		_balance = int(data.get("balance"))
		balance_changed.emit(_balance)
	print("[IAP] verify ok: granted=", data.get("granted", false), " balance=", _balance)
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
		_async_fetch_history()


func _on_wallet_connected(_address: String, _chain_id: int, _is_guest: bool) -> void:
	_async_fetch_balance()
	_async_fetch_history()


# Public trigger so the credits history view can pull fresh data when shown.
func refresh_history() -> void:
	_async_fetch_history()


# gdlint:ignore = async-function-name
func _async_fetch_balance() -> void:
	# Reconciles the server-side balance for the signed-in wallet into the local
	# cache. The wallet is taken from the signed-fetch auth chain server-side, so
	# the path carries no address.
	var wallet := _wallet_address()
	if wallet.is_empty():
		return
	var envelope = await _async_signed_iap("/iap/balance", HTTPClient.METHOD_GET, "")
	# Guard against a wallet switch while the request was in flight.
	if _wallet_address() != wallet:
		return
	if envelope == null or not envelope.get("ok", false):
		return
	var data = envelope.get("data", {})
	if data is Dictionary and data.has("balance"):
		_balance = int(data.get("balance"))
	balance_changed.emit(_balance)


# gdlint:ignore = async-function-name
func _async_fetch_history() -> void:
	# Pulls the wallet's persistent transaction history from the backend (incl.
	# refunds) and replaces the local cache. Entries match the UI shape:
	# {credits, is_refund, timestamp}.
	var wallet := _wallet_address()
	if wallet.is_empty():
		return
	var envelope = await _async_signed_iap("/iap/history", HTTPClient.METHOD_GET, "")
	if _wallet_address() != wallet:
		return
	if envelope == null or not envelope.get("ok", false):
		return
	var data = envelope.get("data", {})
	if not (data is Dictionary):
		return
	var txs = data.get("transactions", [])
	if txs is Array:
		_transaction_history = txs
		transaction_history_updated.emit()


# gdlint:ignore = async-function-name
func _async_signed_iap(path: String, method: int, body: String) -> Variant:
	# DCL signed-fetch (ADR-44) call to the IAP backend. Returns the parsed JSON
	# envelope ({ok, data, ...}) on any HTTP 2xx, or null on a transport error
	# (non-2xx, timeout, unparseable). Business-level rejections come back as
	# 2xx with ok:false, so callers must inspect `ok` themselves.
	var url := _IAP_BACKEND_BASE_URL + path
	var response = await Global.async_signed_fetch(url, method, body)
	if response is PromiseError:
		printerr("[IAP] ", path, " transport error: ", response.get_error())
		return null
	var json = response.get_string_response_as_json()
	if not (json is Dictionary):
		printerr("[IAP] ", path, " unparseable response")
		return null
	return json


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
	# Clear the in-flight guard too, not just the overlay — otherwise a stuck
	# purchase would block every future purchase() via the re-entry check.
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
