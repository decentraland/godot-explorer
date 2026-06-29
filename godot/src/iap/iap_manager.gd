class_name IapManager
extends Node

# Frontend for the IAP purchase flow. Talks to the Swift `DclStoreKit` class
# through the typed `DclStoreKitPlugin` Rust wrapper — no `ClassDB` plumbing
# here; on non-iOS the wrapper's `is_available()` returns false and every
# method is a no-op.
#
# Backend is the Decentraland credits-server, reached over DCL signed-fetch
# (ADR-44). Crediting is DEVICE-DRIVEN: once StoreKit reports a purchase we post
# its Apple-signed JWS to the server, which re-verifies Apple's signature and
# mints the (non-expiring, on-chain) credit right away — no waiting on Apple's
# out-of-band webhook (which stays as an idempotent backstop). The device's job:
#   - POST /credits/iap/register : on wallet connect, register this wallet's
#                                  appAccountToken so the webhook backstop can
#                                  resolve who to credit (StoreKit can redeliver).
#   - POST /credits/iap/quote    : per-wallet pre-purchase gate (daily + total
#                                  caps) BEFORE StoreKit charges. Only on
#                                  `allowed` do we hand off to StoreKit.
#   - POST /credits/iap/verify   : after StoreKit success, submit the JWS to be
#                                  verified + credited. Idempotent with the
#                                  webhook (server dedupes by Apple tx id).
#   - GET  /users/:address/credits : reconcile the on-chain balance (the IAP
#                                  share is `totals.nonExpiring`, in wei) and
#                                  build the history view.
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
# ASC simply returns no product. The credits granted per pack are authoritative
# on the server (IAP_PRODUCT_CATALOG); this map is only used for the optimistic
# success modal and must stay in sync with the server catalog.
const PRODUCT_IDS: PackedStringArray = [
	"credits_tier1",
	"credits_tier2",
	"credits_tier3",
]

const _CREDITS_BY_PRODUCT := {
	"credits_tier1": 30,
	"credits_tier2": 100,
	"credits_tier3": 225,
}

# Bound how long the purchase overlay stays up. StoreKit prompt + validation
# should land well inside this; past it we assume something stuck (network
# drop, redelivery loop, missing signal) and let the user retry.
const _PURCHASE_OVERLAY_TIMEOUT_SEC := 15.0

const _OVERLAY_SCENE_PATH := "res://src/ui/components/organisms/iap_purchase_overlay/iap_purchase_overlay.tscn"
const _SUCCESS_MODAL_SCENE_PATH := "res://src/ui/components/organisms/iap_purchase_success_modal/iap_purchase_success_modal.tscn"

# The IAP backend (Decentraland credits-server) base URL comes from
# `DclUrls.credits_server()`, which resolves it per environment (org/zone/today)
# and can change at runtime (deeplink / DclGlobal.set_dcl_environment). We resolve
# it on every request in `_async_signed_iap` rather than caching it, so an env
# switch takes effect immediately. The host isn't part of the signed payload, so
# repointing it doesn't affect the signing path.

# credits-server stores amounts in wei (1 MANA = 1e18). The IAP balance is
# reported as `totals.nonExpiring` in wei; divide to get whole MANA (== credits).
const _WEI_PER_MANA := 1e18

# Outcomes of POST /credits/iap/verify, mapped to StoreKit's redelivery contract:
# OK — credited (or already credited, idempotent); finish the tx.
# REJECTED — server refused permanently (invalid_jws, token_mismatch,
#            revoked_transaction, unknown_product, cap_exceeded). Finish to stop
#            StoreKit's redelivery loop; retrying won't help.
# RETRY — server unreachable / transient (HTTP 500). Do NOT finish; StoreKit
#         redelivers on the next launch (and the webhook backstop may credit meanwhile).
# DEFERRED — server at its global daily ceiling (service_daily_limit). Apple
#            already charged; the S2S webhook credits it once the cap resets.
#            Finish the tx and tell the user it's on the way — no client retry.
const _OUTCOME_OK := 0
const _OUTCOME_REJECTED := 1
const _OUTCOME_RETRY := 2
const _OUTCOME_DEFERRED := 3

# After a purchase the credit is minted server-side (by /verify or, racing it, the
# webhook), but the reported balance can lag a moment behind the mint. Poll the
# balance every _POST_PURCHASE_POLL_INTERVAL_SEC, for up to _POST_PURCHASE_POLL_ATTEMPTS
# tries (~60s), stopping as soon as it changes — so the UI reflects the new credits
# without the user reopening the view.
const _POST_PURCHASE_POLL_ATTEMPTS := 12
const _POST_PURCHASE_POLL_INTERVAL_SEC := 5.0

# "Option D" hybrid environment, applied when StoreKit reports a `sandbox`
# environment (see _apply_storekit_env). Scenes and comms stay on .org; identity,
# credits, profile and the marketplace catalog go to .zone (default zone, with
# catalyst/comms/events/mobilebff pinned back to org). This is the dclenv config
# verified cross-device for the IAP sandbox flow.
const _SANDBOX_DCLENV := "catalyst::org,comms::org,events::org,mobilebff::org,profile::zone,zone"

# Total + daily credit caps are enforced server-side by the IAP backend
# (POST /credits/iap/quote). The client no longer holds these limits.

# Gates all IAP behavior. Enabled by default on iOS — the only platform with
# StoreKit — and left off everywhere else (see _ready). Until enable() is
# called, _ready is a no-op: no signal subscriptions, no product load, and
# is_available() returns false (which hides the credits UI entry point).
var enabled: bool = false

var _store_kit := DclStoreKitPlugin.new()
var _store_kit_available: bool = false
# Synchronous receipt-read environment captured at startup, held until the
# authoritative AppTransaction resolves so both go out in one atomic event.
var _env_sync_value: String = ""
# ms-since-startup when the synchronous read above was taken (same clock the
# `[Startup]` logs use: Time.get_ticks_msec() - Global._startup_time).
var _env_sync_at_ms: int = 0
# Set true once the authoritative environment has resolved AND any resulting
# Option D switch has been applied. Lets env-dependent callers (session restore)
# block on async_await_env_resolved() until the backend choice is final.
var _env_resolved: bool = false
var _products: Array = []
# Local cache of the server-authoritative balance, reconciled from
# GET /users/:address/credits. Server is the source of truth.
var _balance: int = 0
# Tx-id dedup. Apple delivers the same transaction twice on a fresh purchase
# (once via `purchaseCompleted`, once via the `Transaction.updates` listener
# that picks up any unfinished tx). Cleared on relaunch like `_balance`.
var _seen_tx_ids: Dictionary = {}
# Local cache of the server's IAP credits, rebuilt from
# GET /users/:address/credits. Populated on wallet connect and when the history
# view opens; also gets the just-bought entry appended optimistically.
var _transaction_history: Array = []
# Bumped on each purchase start AND each overlay hide so stale SceneTreeTimer
# timeouts (which can't be cancelled) become no-ops.
var _overlay_token: int = 0
var _overlay: CanvasLayer = null
var _purchase_in_flight: bool = false


func _ready() -> void:
	# IAP is enabled by default on iOS — the only platform with StoreKit. On
	# every other platform there's no native backend, so we leave it disabled
	# (enable() would no-op anyway via is_available(), and the credits UI entry
	# points stay hidden).
	if Global.is_ios():
		enable()


# Reports the StoreKit environment (production / sandbox / xcode) to analytics.
# Runs unconditionally on iOS at startup — independent of enable()
# — so we collect production telemetry from real App Store installs and validate
# the environment-detection mechanism BEFORE wiring it to backend selection.
# StoreKit's environment is fixed by how the binary was distributed and can't be
# chosen by the app (see docs/iap-zone-submission/), so it's the ground truth for
# which IAP backend a device will hit. No-op on non-iOS.
func report_environment_to_analytics() -> void:
	# is_available() lazily instantiates the Swift class; on non-iOS the class
	# isn't registered so this returns false. Note this does NOT start the
	# Transaction.updates listener (that only happens in enable()).
	if not _store_kit.is_available():
		return

	# Capture the synchronous receipt read NOW, but don't emit yet: we send a
	# single atomic event once the authoritative AppTransaction resolves, so the
	# two readings can never arrive asymmetrically. The Swift side guarantees the
	# signal fires exactly once (hard timeout falls back to the receipt read), so
	# the event is never missing.
	_env_sync_value = _store_kit.current_environment()
	_env_sync_at_ms = Time.get_ticks_msec() - Global._startup_time

	if not _store_kit.environment_resolved.is_connected(_on_environment_resolved):
		_store_kit.environment_resolved.connect(_on_environment_resolved, CONNECT_DEFERRED)
	_store_kit.resolve_environment()


func _on_environment_resolved(environment: String, source: String, resolve_ms: float) -> void:
	var env_at_ms := Time.get_ticks_msec() - Global._startup_time
	var matched := environment == _env_sync_value
	print(
		(
			"[IAP] StoreKit env: authoritative=%s (%s) sync=%s match=%s resolve=%dms sync@%dms env@%dms"
			% [
				environment,
				source,
				_env_sync_value,
				matched,
				int(resolve_ms),
				_env_sync_at_ms,
				env_at_ms
			]
		)
	)
	if Global.metrics != null:
		# One atomic event with BOTH readings + resolve latency + the startup-relative
		# times each reading was taken + agreement. flush() ships it immediately so a
		# short session can't drop it (it respects the EULA consent gate, so
		# pre-consent it stays queued until consent opens). dcl_environment is the
		# launch env (recorded before the sandbox switch below).
		Global.metrics.track_ios_storekit_environment(
			environment,
			_env_sync_value,
			source,
			resolve_ms,
			_env_sync_at_ms,
			env_at_ms,
			str(DclGlobal.get_dcl_environment()),
			_store_kit.can_make_payments()
		)
		Global.metrics.flush()

	# Route identity/credits/catalog to .zone when StoreKit is sandbox (Option D).
	# After tracking, so the analytics event records the launch env, not the switch.
	_apply_storekit_env(environment)

	# Unblock any caller awaiting the env decision (session restore) — set AFTER the
	# switch so they observe the final env, never an intermediate one.
	_env_resolved = true


# Switches the app to the Option D hybrid environment when StoreKit reports a
# `sandbox` environment (TestFlight, Apple App Review, or Xcode), so credits,
# profile and the marketplace catalog transact against .zone while scenes/comms
# stay on .org. A `production` environment (real App Store install) needs no
# change — it stays on the .org default.
#
# Keys off the AUTHORITATIVE environment only (never the synchronous receipt
# read): Apple's reviewer device reports authoritative=sandbox but receipt=
# production, and it must reach the zone backend or every purchase fails. Callers
# that depend on the result gate on async_await_env_resolved() so they never run
# before this has applied.
#
# Gated on two conditions so it never overrides a developer/QA choice:
#  - PRODUCTION builds only (release export template). A debug/dev build run on a
#    device with a sandbox Apple ID also reports `sandbox`, but there the dev's
#    chosen env (default org or --dclenv) must be respected.
#  - Only an EXPLICIT `dclenv=org` exempts from the switch. Passing
#    `decentraland://open?dclenv=org` (or `--dclenv org`) means "really use the prod
#    .org backend" and wins. Any OTHER explicit env is not special-cased here — it
#    falls through to the `current != "org"` backstop below (which already leaves any
#    non-org env untouched). A non-explicit default org is the only thing that switches.
# Runtime-only: not persisted, since StoreKit re-reports the ground-truth
# environment on every launch.
func _apply_storekit_env(environment: String) -> void:
	if environment != "sandbox":
		return
	# Only production (release export template) builds auto-switch. A debug/dev build
	# run on a device with a sandbox Apple ID also reports "sandbox", but there we must
	# respect the dev's chosen env (org by default or --dclenv).
	if OS.is_debug_build():
		print("[IAP] StoreKit sandbox on a debug build — not forcing the Option D env")
		return
	var current := str(DclGlobal.get_dcl_environment())
	# The only explicit override that exempts from the switch is dclenv=org
	# (decentraland://open?dclenv=org → "really use the prod .org backend"). Any other
	# explicit env isn't special-cased here: it falls through to the current != "org"
	# check below.
	if Global.dcl_env_explicit and current == "org":
		print("[IAP] StoreKit sandbox, but explicit dclenv=org — staying on .org")
		return
	if current != "org":
		print("[IAP] StoreKit sandbox, but dclenv already '", current, "' — leaving as-is")
		return
	print("[IAP] StoreKit sandbox → switching to Option D hybrid env: ", _SANDBOX_DCLENV)
	DclGlobal.set_dcl_environment(_SANDBOX_DCLENV)


# Blocks the caller until the AUTHORITATIVE StoreKit environment has resolved and
# any resulting Option D switch is applied — so an env-dependent fetch (chiefly
# session restore's profile/credits load) never races the switch. Returns at once
# when there's nothing to wait for: non-iOS, StoreKit unavailable, or already
# resolved. In the common case the env resolved during the boot/version-gate
# round-trip, so this returns without ever yielding.
#
# `timeout_sec` is a safety valve: the Swift side hard-caps the resolve at 10s,
# but a network-bound AppTransaction could still stall the loading screen, so we
# proceed with the current env past the timeout. On iOS the resolve signal always
# fires, so this only ever waits when the resolve is genuinely slow.
# gdlint:ignore = async-function-name
func async_await_env_resolved(timeout_sec: float = 5.0) -> void:
	if _env_resolved or not _store_kit.is_available():
		return
	var timer := get_tree().create_timer(timeout_sec)
	while not _env_resolved and timer.time_left > 0.0:
		await get_tree().process_frame
	if not _env_resolved:
		printerr(
			"[IAP] env resolve timed out after ", timeout_sec, "s; proceeding with current env"
		)


# Idempotent. Called from _ready on iOS. Performs the StoreKit wiring (signal
# subscriptions, product load, wallet hookup).
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
		return
	if _purchase_in_flight:
		Global.modal_manager.async_show_purchase_in_flight_modal()
		return
	var wallet := _wallet_address()
	if wallet.is_empty():
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
	# On `allowed` the server also registers this wallet's appAccountToken so the
	# webhook can resolve who to credit.
	var body := JSON.stringify({"productId": product_id})
	var envelope = await _async_signed_iap("/credits/iap/quote", HTTPClient.METHOD_POST, body)
	# null == transport/auth failure (non-2xx, timeout). Fail closed: do NOT charge.
	if envelope == null:
		_finish_purchase_flow()
		Global.modal_manager.async_show_purchase_failed_modal()
		purchase_failed.emit(product_id, "quote failed")
		return
	# Unified envelope: a denial is HTTP 200 with ok:false + a business `code`
	# (cap_exceeded[+reason] / service_daily_limit / unknown_product). There is no
	# `data.allowed` anymore.
	if not envelope.get("ok", false):
		var code := str(envelope.get("code", ""))
		var reason := str(envelope.get("reason", ""))
		_finish_purchase_flow()
		_show_quote_denied_modal(code, reason)
		purchase_failed.emit(product_id, "not allowed: " + code)
		return
	# Allowed — initiate the real StoreKit purchase. The Swift side derives the same
	# appAccountToken from the wallet (== data.appAccountToken the server registered),
	# and /verify is signed with this same wallet, so the server's token check passes.
	# Overlay stays up until the purchase resolves via the StoreKit handlers.
	_store_kit.purchase(product_id, wallet)


func _show_quote_denied_modal(code: String, reason: String) -> void:
	# Map a /quote denial code to the matching UI. `reason` only accompanies
	# `cap_exceeded` and names the breached axis (total_limit / daily_limit).
	match code:
		"cap_exceeded":
			if reason == "total_limit":
				Global.modal_manager.async_show_credit_limit_total_modal()
			else:
				Global.modal_manager.async_show_credit_limit_daily_modal()
		"service_daily_limit":
			# Server-wide daily ceiling (not the user's fault). Nothing was charged
			# at quote time; ask them to try again later.
			Global.modal_manager.async_show_purchase_unavailable_modal()
		_:
			# unknown_product or any unexpected code.
			Global.modal_manager.async_show_purchase_failed_modal()


func _record_transaction(credits: int, is_refund: bool) -> void:
	var now = Time.get_datetime_dict_from_system()
	(
		_transaction_history
		. push_front(
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
	_async_handle_purchased_transaction(tx)


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
	_async_handle_purchased_transaction(tx)


# gdlint:ignore = async-function-name
func _async_handle_purchased_transaction(tx: Dictionary) -> void:
	# StoreKit reports an Apple-verified purchase. We drive the credit from the
	# device: POST the StoreKit JWS to credits-server, which re-verifies Apple's
	# signature and mints. The outcome maps to StoreKit's redelivery contract:
	#   OK       -> finish the tx (Apple stops redelivering).
	#   REJECTED -> finish anyway (a bad JWS / over-cap won't succeed on retry).
	#   DEFERRED -> finish the tx; the webhook credits it later (server at daily cap).
	#   RETRY    -> do NOT finish; StoreKit redelivers on the next launch (and the
	#               webhook backstop may credit the same tx meanwhile).
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

	var outcome: int = await _async_credit_with_backend(tx)
	match outcome:
		_OUTCOME_OK:
			var credits: int = _CREDITS_BY_PRODUCT.get(product_id, 0)
			_store_kit.finish_transaction(tx_id)
			_finish_purchase_flow()
			_show_success_modal(credits)
			# Optimistic entry; the balance/history refresh below replaces it with
			# the authoritative on-chain list.
			_record_transaction(credits, false)
			purchase_completed.emit(product_id, credits)
			print("[IAP] tx ", tx_id, " credited; polling balance")
			_async_poll_balance_after_purchase()
		_OUTCOME_REJECTED:
			# Permanent refusal (bad JWS, unknown product, over cap). Finishing
			# breaks the redelivery loop — retrying won't help.
			printerr("[IAP] tx ", tx_id, " rejected by backend; finishing")
			_store_kit.finish_transaction(tx_id)
			_finish_purchase_flow()
			Global.modal_manager.async_show_purchase_failed_modal()
			purchase_failed.emit(product_id, "rejected by backend")
		_OUTCOME_DEFERRED:
			# Apple charged, but the server is at its global daily ceiling. The S2S
			# webhook is independent of StoreKit redelivery and credits this tx once
			# the cap resets, so we finish the tx, tell the user it's on the way, and
			# poll the balance in case it lands shortly. No failure, no client retry.
			_store_kit.finish_transaction(tx_id)
			_finish_purchase_flow()
			Global.modal_manager.async_show_purchase_processing_modal()
			purchase_pending.emit(product_id)
			print("[IAP] tx ", tx_id, " deferred (service_daily_limit); webhook will credit")
			_async_poll_balance_after_purchase()
		_OUTCOME_RETRY:
			# Transient (server unreachable / not ready). Don't finish: StoreKit
			# redelivers on the next launch. Unmark so that delivery retries.
			_seen_tx_ids.erase(tx_id)
			printerr("[IAP] tx ", tx_id, " transient; will retry on next launch")
			_finish_purchase_flow()
			purchase_failed.emit(product_id, "network error, retry on next launch")


# gdlint:ignore = async-function-name
func _async_credit_with_backend(tx: Dictionary) -> int:
	# POSTs the StoreKit transaction JWS to credits-server /credits/iap/verify.
	# The server re-verifies Apple's signature and mints idempotently. Maps the
	# unified envelope to an outcome (see caller / the _OUTCOME_* docs):
	#   OK       -> ok:true (credited, or alreadyExisted via the webhook race).
	#   DEFERRED -> ok:false code:service_daily_limit (charged; webhook credits later).
	#   RETRY    -> transport error / HTTP 500 (transient); redeliver on next launch.
	#   REJECTED -> ok:false with any other code (invalid_jws / token_mismatch /
	#               revoked_transaction / unknown_product / cap_exceeded). Permanent.
	var jws := str(tx.get("jwsRepresentation", ""))
	if jws.is_empty():
		printerr("[IAP] missing JWS; cannot credit")
		return _OUTCOME_REJECTED
	var body := JSON.stringify({"jwsRepresentation": jws})
	var envelope = await _async_signed_iap("/credits/iap/verify", HTTPClient.METHOD_POST, body)
	# null == transport error (non-2xx / HTTP 500 / timeout) -> transient -> RETRY.
	if envelope == null:
		printerr("[IAP] verify transport error; will retry on next launch")
		return _OUTCOME_RETRY
	# Business rejections come back as HTTP 200 with ok:false + a `code`.
	if not envelope.get("ok", false):
		var code := str(envelope.get("code", ""))
		if code == "service_daily_limit":
			# Charged OK, but the server is at its global daily ceiling. The S2S
			# webhook credits it once the cap resets — not a failure, don't retry.
			print("[IAP] verify deferred (service_daily_limit); webhook will credit")
			return _OUTCOME_DEFERRED
		# invalid_jws / token_mismatch / revoked_transaction / unknown_product /
		# cap_exceeded — permanent refusals; retrying won't help.
		printerr("[IAP] verify rejected: ", code)
		return _OUTCOME_REJECTED
	var data = envelope.get("data", {})
	var already = data is Dictionary and data.get("alreadyExisted", false)
	print("[IAP] verify ok: credited (alreadyExisted=", already, ")")
	return _OUTCOME_OK


func _connect_wallet_signals() -> void:
	if Global.player_identity == null:
		return
	if not Global.player_identity.wallet_connected.is_connected(_on_wallet_connected):
		Global.player_identity.wallet_connected.connect(_on_wallet_connected)
	# If the session was restored synchronously the wallet is already there
	# and the signal won't fire — register + fetch now to cover that case.
	if not _wallet_address().is_empty():
		_async_register_token()
		_async_fetch_balance()
		_async_fetch_history()


func _on_wallet_connected(_address: String, _chain_id: int, _is_guest: bool) -> void:
	_async_register_token()
	_async_fetch_balance()
	_async_fetch_history()


# Public trigger so the credits history view can pull fresh data when shown.
func refresh_history() -> void:
	_async_fetch_history()


# Public: re-fetch the balance from the server and return the fresh value. Used by
# the marketplace return tracker to poll for credit consumption after a purchase.
# Falls back to the cached balance if the fetch fails (so callers never read 0 on
# a transient error). Also refreshes any balance UI via the balance_changed emit.
# gdlint:ignore = async-function-name
func async_refresh_balance() -> int:
	await _async_fetch_balance()
	return _balance


# gdlint:ignore = async-function-name
func _async_register_token() -> void:
	# Register this wallet's appAccountToken so the Apple webhook can resolve the
	# wallet to credit — including for transactions StoreKit redelivers on a
	# later launch that never went through a fresh quote. Idempotent server-side.
	if _wallet_address().is_empty():
		return
	await _async_signed_iap("/credits/iap/register", HTTPClient.METHOD_POST, "{}")


# gdlint:ignore = async-function-name
func _async_fetch_balance() -> void:
	# Reconciles the server-side IAP balance for the signed-in wallet into the
	# local cache. The credits-server endpoint carries the address in the path
	# and returns totals in wei; the IAP share is `totals.nonExpiring`.
	var wallet := _wallet_address()
	if wallet.is_empty():
		return
	var envelope = await _async_signed_iap(
		"/users/" + wallet + "/credits", HTTPClient.METHOD_GET, ""
	)
	# Guard against a wallet switch while the request was in flight.
	if _wallet_address() != wallet:
		return
	if envelope == null:
		return
	var totals = envelope.get("totals", {})
	if not (totals is Dictionary):
		return
	_balance = int(round(float(totals.get("nonExpiring", 0)) / _WEI_PER_MANA))
	balance_changed.emit(_balance)


# gdlint:ignore = async-function-name
func _async_fetch_history() -> void:
	# Builds the history view from the wallet's on-chain credits. credits-server
	# has no per-transaction history endpoint; we derive entries from the IAP
	# credits returned by GET /users/:address/credits. Refunded credits are not
	# returned by this endpoint, so entries are always non-refund here.
	var wallet := _wallet_address()
	if wallet.is_empty():
		return
	var envelope = await _async_signed_iap(
		"/users/" + wallet + "/credits", HTTPClient.METHOD_GET, ""
	)
	if _wallet_address() != wallet:
		return
	if envelope == null:
		return
	var credits = envelope.get("credits", [])
	if not (credits is Array):
		return
	var history: Array = []
	for entry in credits:
		if not (entry is Dictionary):
			continue
		if str(entry.get("creditSource", "")) != "iap":
			continue
		# amount is wei (a string, to avoid int64 overflow); timestamp is in ms.
		var mana := int(round(float(str(entry.get("amount", "0"))) / _WEI_PER_MANA))
		var ts_ms := float(entry.get("timestamp", 0))
		var dt = Time.get_datetime_dict_from_unix_time(int(ts_ms / 1000.0))
		(
			history
			. append(
				{
					"credits": mana,
					"is_refund": false,
					"timestamp": "%04d.%02d.%02d" % [dt.year, dt.month, dt.day],
				}
			)
		)
	_transaction_history = history
	transaction_history_updated.emit()


# gdlint:ignore = async-function-name
func _async_poll_balance_after_purchase() -> void:
	# Keep polling the balance after a purchase until it changes, so the UI picks up
	# the freshly minted credits on its own. Checks immediately first (the credit is
	# often already minted by the time we get here), then every
	# _POST_PURCHASE_POLL_INTERVAL_SEC for up to _POST_PURCHASE_POLL_ATTEMPTS tries.
	var before := _balance
	for i in range(_POST_PURCHASE_POLL_ATTEMPTS):
		await _async_fetch_balance()
		if _balance != before:
			break
		# Don't sleep after the last attempt — we're about to give up.
		if i < _POST_PURCHASE_POLL_ATTEMPTS - 1:
			await get_tree().create_timer(_POST_PURCHASE_POLL_INTERVAL_SEC).timeout
	# Refresh the history once the balance has (likely) settled.
	_async_fetch_history()


# gdlint:ignore = async-function-name
func _async_signed_iap(path: String, method: int, body: String) -> Variant:
	# DCL signed-fetch (ADR-44) call to the IAP backend. Returns the parsed JSON
	# response on any HTTP 2xx, or null on a transport error (non-2xx, timeout,
	# unparseable). Some endpoints wrap their result in {ok, data, ...} and others
	# (GET /users/:address/credits) return the object directly, so callers inspect
	# the fields they expect themselves.
	# Resolve the base URL per call so a runtime environment switch is picked up.
	var url := DclUrls.credits_server() + path
	var response = await Global.async_signed_fetch(url, method, body)
	if response is PromiseError:
		return null
	var json = response.get_string_response_as_json()
	if not (json is Dictionary):
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
