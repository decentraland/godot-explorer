class_name DeepLinkRouter
extends RefCounted

## Centralized deep link processing and path routing.
##
## Handles incoming deep links from iOS signals, Android intents, and
## --fake-deeplink CLI args. Parses the URL, updates Global.deep_link_obj/url,
## and emits the appropriate signal based on the URL path.

signal deep_link_received
signal deep_link_jump
signal deep_link_open_event(event_id: String)
signal deep_link_open_place(place_id: String)
## A `?signin=` token was parked because no in-process auth was pending (#2644). The lobby
## listens so it can redeem it when the deep link lands after its own _ready; when it lands
## before, lobby._ready reads the parked token directly. Both go through
## take_pending_signin_identity_id(), which only hands it out once.
signal deep_link_signin_parked

# How long a parked `?signin=` token stays redeemable. Only meant to cover the boot it
# arrived on (lobby._ready lands a couple of seconds in); the cap exists so a token nobody
# consumed can't be replayed on an unrelated later visit to the lobby and surface a bogus
# "Authentication failed". See _handle_signin_deep_link.
const PENDING_SIGNIN_MAX_AGE_MS: int = 5 * 60 * 1000

# Parked `?signin=` identity id + when it was parked. Read through
# take_pending_signin_identity_id() only; lobby.gd is the single consumer.
var _pending_signin_identity_id: String = ""
var _pending_signin_parked_at_ms: int = 0


## Parse and store a deep link URL, then emit deep_link_received.
## Called from _notification(FOCUS_IN) and iOS deeplink_received signal.
func process_deep_link(url: String) -> void:
	if url.is_empty():
		return

	# Consume receivedUrl from the iOS plugin so that _notification(FOCUS_IN)
	# doesn't re-read the same URL via get_deeplink_url() on the next resume.
	if DclIosPlugin.is_available():
		DclIosPlugin.get_deeplink_args()

	Global.deep_link_url = url
	Global.deep_link_obj = DclParseDeepLink.parse_decentraland_link(url)
	print("[DEEPLINK] process_deep_link params: ", Global.deep_link_obj.params)

	# Apply rust-log from deeplink params
	var rust_log_value = Global.deep_link_obj.params.get("rust-log", "")
	if not rust_log_value.is_empty():
		print("[DEEPLINK] Found rust-log param: ", rust_log_value)
		DclGlobal.set_rust_log_filter(rust_log_value)

	# Pulse transport params (pulse-server / pulse / dual-channel / livekit); the
	# shared helper no-ops on builds without the use_pulse feature.
	Global._apply_comms_deeplink_params(Global.deep_link_obj)

	Global._apply_optimized_content_base_url(Global.deep_link_obj)

	# QA affordance, non-production only: mint a brand-new guest so the FTUE is reachable
	# again on a device whose native anchor survives reinstall.
	Global._capture_debug_guest_rotate(Global.deep_link_obj)

	# Before any routing decision: the token has to survive whichever branch below consumes
	# the deeplink (#2670).
	Global._capture_campaign_token(Global.deep_link_obj)

	# `skip-gltf` toggle has to be set BEFORE any scene's GLTF_CONTAINER
	# component dirty-set is processed by `update_gltf_container`. The
	# bench runner's `_apply_deeplink_overrides` runs too late — by then
	# the first scene's GLTFs are already instantiated. Apply here, in
	# the deeplink router, which lands before any scene starts loading.
	var skip_gltf_value = Global.deep_link_obj.params.get("skip-gltf", "")
	if not skip_gltf_value.is_empty():
		Global.cli.set_skip_gltf_load(skip_gltf_value.to_lower() in ["true", "1", "yes"])
		print("[DEEPLINK] skip-gltf=", Global.cli.get_skip_gltf_load())

	var kill_sky_value = Global.deep_link_obj.params.get("kill-sky", "")
	if not kill_sky_value.is_empty():
		Global.cli.set_kill_sky(kill_sky_value.to_lower() in ["true", "1", "yes"])
		print("[DEEPLINK] kill-sky=", Global.cli.get_kill_sky())

	# Touch-feedback debug overlay (issue #2562): off by default, enabled on demand.
	var touch_feedback_value: String = Global.deep_link_obj.params.get("touch-feedback", "")
	if not touch_feedback_value.is_empty():
		var touch_feedback_enable: bool = touch_feedback_value.to_lower() in ["true", "1", "yes"]
		TouchFeedback.set_enabled(touch_feedback_enable)
		print("[DEEPLINK] touch-feedback=", touch_feedback_enable)

	# Opt-in gate for deleting an UPGRADED (email-linked) guest. Sticky-on for the
	# session; only takes effect on a NON-production build (see
	# Global.is_upgraded_deletion_enabled() + account_deletion_popup.gd).
	var upgraded_deletion_value = Global.deep_link_obj.params.get("enable-upgraded-deletion", "")
	if upgraded_deletion_value.to_lower() in ["true", "1", "yes"]:
		Global._enable_upgraded_deletion = true
		print("[DEEPLINK] enable-upgraded-deletion=true")

	# Genesis Plaza profiling benchmark (issue #1862). The CLI path spawns the
	# runner from Global._ready, but on mobile the deep link is not parsed by
	# then — spawn here once the deeplink lands and only if no runner exists.
	if Global.deep_link_obj.gp_benchmark and Global.get_node_or_null("GPBenchmarkRunner") == null:
		# Flip bench_mode BEFORE spawning the runner: this fires earlier than
		# DG's deferred _init_dynamic_graphics_manager (07.4xx vs 07.7xx on A54)
		# and before lobby.gd's first-launch HW bench trigger, so both honor it.
		Global.cli.bench_mode = true
		print("[DEEPLINK] bench_mode=true (gp-benchmark deeplink)")
		print("[DEEPLINK] Spawning GP benchmark runner")
		var gp_runner = load("res://src/tools/gp_benchmark_runner.gd").new()
		gp_runner.set_name("GPBenchmarkRunner")
		Global.add_child(gp_runner)

	if Global.deep_link_obj.safe_margin_debug:
		Global.set_safe_margin_debug_enable(true)

	# Returning from the in-app marketplace webview: the web fires a
	# decentraland://open?iap_enabled=true[&urn=<urn>] deep link to bring the app back. The
	# native side dismisses the SFSafariViewController directly, which never fires the
	# tracker's webview_closed signal — so drive the post-return handling here (restore the
	# landscape the portrait-only IAP view took away, then poll for the purchase against the
	# pre-purchase baseline and refresh the balance).
	#
	# `urn` is OPTIONAL — the web leaves it out whenever it has no purchased item in context
	# — yet it used to be the only thing that triggered ANY of this. A urn-less return then
	# did nothing at all: the app stayed in the forced portrait, where the backpack's
	# landscape-only back button is hidden and the menu's portrait bottom bar has already
	# been freed (clean_orientation.gd, when the menu was built in landscape), leaving no way
	# out of the backpack; and the tracker stayed armed forever, so the purchase never
	# surfaced either. So recognise the return by the TRACKER's own state, and use the
	# `iap_enabled`/`urn` params only to decide whether the link is ours to swallow.
	var return_urn: String = str(Global.deep_link_obj.params.get("urn", ""))
	var iap_marker: String = str(Global.deep_link_obj.params.get("iap_enabled", ""))
	var is_marketplace_return: bool = not return_urn.is_empty() or not iap_marker.is_empty()
	if is_marketplace_return or MarketplaceTracker.is_awaiting_return():
		print("[DEEPLINK] marketplace return — driving tracker return handling")
		# A no-op unless a round-trip is actually in flight, so a duplicate delivery (iOS
		# fires application:openURL: twice for a single tap) or a stale link is harmless.
		MarketplaceTracker.notify_marketplace_return()
	if is_marketplace_return:
		# Swallow it so it doesn't fall through to the "/open" routing below and pop the
		# jump-in panel (scene title + placeholders). A deep link that merely arrived while
		# the tracker was armed is NOT ours, so it keeps routing normally.
		_clear_deep_link()
		return

	# Trigger avatar impostor benchmark
	var bench_param = Global.deep_link_obj.params.get("benchmark", "")
	if bench_param == "avatar-impostors":
		print("[DEEPLINK] Triggering avatar impostor benchmark")
		if Global.player_identity.get_profile_or_null() == null:
			Global.player_identity.set_default_profile()
		Global.set_meta("avatar_impostor_benchmark_auto_quit", true)
		Global.get_tree().change_scene_to_file.call_deferred(
			"res://src/tools/avatar_impostor_benchmark.tscn"
		)
		_clear_deep_link()
		return

	# Ignore WalletConnect callbacks (decentraland://walletconnect)
	if Global.deep_link_obj.is_walletconnect_callback:
		print("[DEEPLINK] Ignoring WalletConnect callback")
		return

	# Check for environment change — requires restart (sign out back to lobby)
	if Global._check_dclenv_change():
		return

	# Handle signin deep link for mobile auth flow
	if Global.deep_link_obj.is_signin_request():
		_handle_signin_deep_link(Global.deep_link_obj.signin_identity_id)
		_clear_deep_link()
	else:
		deep_link_received.emit.call_deferred()


## Route the current deep link based on its path.
## Called after the explorer or menu is ready to act on the deep link.
func route() -> void:
	# Only process deep links on real mobile devices (not emulation/desktop)
	if not Global.is_mobile() or Global.is_virtual_mobile():
		return

	# Skip if no pending deep link (already consumed or none received)
	if Global.deep_link_url.is_empty():
		return

	# Ignore WalletConnect callbacks
	if Global.deep_link_obj.is_walletconnect_callback:
		_clear_deep_link()
		return

	var path: String = Global.deep_link_obj.path
	# Normalize: strip trailing slashes, treat root as empty
	path = path.rstrip("/")

	match path:
		"/jump", "/open":
			# If location or realm is provided, teleport; otherwise open jump panel
			if (
				Global.deep_link_obj.is_location_defined()
				or not Global.deep_link_obj.realm.is_empty()
				or not Global.deep_link_obj.preview.is_empty()
			):
				_route_teleport()
			elif Global.deep_link_obj.params.is_empty():
				deep_link_jump.emit()
			# else: config-only params (multiplayer_debug, pulse, rust-log, scene-stats,
			# …) were already applied in process_deep_link — a link with no navigation
			# target must not pop an empty jump-in panel over Discover.
		"/events":
			var event_id: String = Global.deep_link_obj.params.get("id", "")
			if not event_id.is_empty():
				deep_link_open_event.emit(event_id)
			else:
				Global.open_discover.emit()
		"/places":
			var place_id: String = Global.deep_link_obj.params.get("id", "")
			if not place_id.is_empty():
				deep_link_open_place.emit(place_id)
			else:
				Global.open_discover.emit()
		_:
			# "/mobile", "", or any other path -> existing teleport behavior
			_route_teleport()

	_clear_deep_link()


func _route_teleport() -> void:
	var realm = Global.deep_link_obj.preview
	if realm.is_empty():
		realm = Global.deep_link_obj.realm
	var location: Vector2i = Global.deep_link_obj.location
	var has_location := Global.deep_link_obj.is_location_defined()

	# The deeplink target is captured above and consumed now — clear realm/location on the shared
	# deep_link_obj so a later explorer boot (e.g. teleporting away after a private-world block)
	# can't re-read this stale realm and re-trigger the modal (#2569 review, iOS). preview is left
	# untouched: it drives preview/hot-reload mode with its own lifecycle.
	Global.deep_link_obj.realm = ""
	Global.deep_link_obj.location = Vector2i.MAX

	# World realm without explicit location → join_world, skip ban pre-check (deferred post-loading)
	if not realm.is_empty() and Realm.is_dcl_ens(realm) and not has_location:
		Global.async_join_world(realm)
	elif has_location:
		if realm.is_empty():
			realm = DclUrls.main_realm()
		Global.async_teleport_to(location, realm)
	elif not realm.is_empty():
		Global.async_teleport_to(Vector2i.ZERO, realm)


## Redeem a `decentraland://open?signin=<identityId>` token.
##
## The token is self-contained — complete_mobile_connect_account fetches the identity by id
## — so the only question is who drives the UI while that fetch runs.
func _handle_signin_deep_link(identity_id: String) -> void:
	# Checked ahead of the pending branch: abort_try_connect_account clears
	# pending_mobile_auth, but start_mobile_connect_account's spawn is not abortable and sets
	# it again on resolve, so a cancel during the browser-opening window leaves it true.
	if Global.player_identity.was_mobile_auth_cancelled():
		print("[DEEPLINK] Ignoring signin token: the user cancelled this sign-in")
		return

	if Global.player_identity.has_pending_mobile_auth():
		# Warm resume: this same process opened the browser, so the lobby is alive with the
		# AUTH_BROWSER_OPEN spinner up and its auth signals connected. Complete right here.
		Global.player_identity.complete_mobile_connect_account(identity_id)
		return

	# Already signed in, so this cannot be the cold start we rescue. Parking it leaves a token
	# with no consumer until sign_out() swaps in a fresh lobby, whose _ready redeems it — the
	# user asks to sign out and is signed back in as whoever the link belongs to.
	if not Global.player_identity.get_address_str().is_empty():
		print("[DEEPLINK] Ignoring signin token: a wallet is already connected")
		return

	# Cold start (#2644). The OS killed the process during the browser hop, taking the
	# in-memory pending flag with it, and the deep link came back to a fresh boot. This used
	# to `printerr` and drop the token, stranding the user on ACCOUNT_HOME after a sign-in
	# they had already completed — ~26% of logins that hit a cold start survived it.
	#
	# Don't complete it here: on a cold start this runs from Global._notification, before the
	# first scene exists, so wallet_connected/profile_changed could fire into a lobby that
	# hasn't connected them yet. Park the token and let the lobby redeem it on its own terms.
	print("[DEEPLINK] signin token arrived with no pending auth (cold start) — parking it")
	_pending_signin_identity_id = identity_id
	_pending_signin_parked_at_ms = Time.get_ticks_msec()
	deep_link_signin_parked.emit.call_deferred()


## Consume a parked `?signin=` identity id. Returns "" when there is none or it went stale
## (see PENDING_SIGNIN_MAX_AGE_MS). Single-shot: the token is cleared on read, so the two
## lobby entry points (its _ready and the deep_link_signin_parked signal) can both call
## this and only whichever gets there first acts on it.
func take_pending_signin_identity_id() -> String:
	var identity_id := _pending_signin_identity_id
	_pending_signin_identity_id = ""
	if identity_id.is_empty():
		return ""

	var age_ms := Time.get_ticks_msec() - _pending_signin_parked_at_ms
	if age_ms > PENDING_SIGNIN_MAX_AGE_MS:
		print("[DEEPLINK] Dropping parked signin token, %ss old" % (age_ms / 1000))
		return ""
	return identity_id


func _clear_deep_link() -> void:
	# Only clear the URL flag, not deep_link_obj.
	# deep_link_obj is still needed by scene_fetcher (preview mode)
	# and other systems that check deep link parameters.
	Global.deep_link_url = ""
