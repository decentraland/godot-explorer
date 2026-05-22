class_name AttestationService
extends Node

# Platform attestation orchestrator. Exchanges native attestation artifacts
# (Apple App Attest / Google Play Integrity) for a server-issued session
# token from mobile-bff. The token is cached on disk and reused until the
# server-controlled expiry (~48h) before re-attesting.
#
# Dispatch + signal pattern:
#   async_get_valid_jwt()  →  cache hit?  →  return immediately.
#                             cache miss  →  _kick() (idempotent)
#                                         →  await jwt_refreshed
#                                         →  return token.
#
# Multiple concurrent callers coalesce on a single attestation cycle (one
# HTTP roundtrip even with N awaiters).
#
# Boot:
#   - _ready loads any cached session from disk.
#   - Waits for EULA acceptance (poll every EULA_POLL_INTERVAL_SEC).
#   - Once accepted, dispatches a kick to pre-warm the session if absent.
#
# Retry: failed cycles back off [1,2,5,10,30]s capped indefinite. Deliberate
# soft-fail — never blocks UI, never emits a terminal failure signal. Callers
# awaiting jwt_refreshed wait until success or hang forever on permanently
# unsupported devices (those callers should pre-check is_supported()).
#
# Persistence:
#   - user://attest_session.json  : {token, expires_at, platform}. Plaintext.
#     Bearer secret — TODO: move to Keychain/Keystore via native plugin.
#   - Nothing else. Per mobile-bff PR #54, iOS uses fresh App Attest keys
#     per session (no key_id persistence).
#
# Server contract (mobile-bff PR #54):
#   POST {bff}/attest/ios/challenge → {challenge, expires_at}
#   POST {bff}/attest/session       → {token, expires_at}
#     iOS     : header x-attest-platform: ios + JSON body
#               {key_id, attestation_object, challenge}.
#     Android : headers x-attest-platform: android + x-attest-integrity-token:
#               <PI JWS>, raw body (empty; the bound nonce is SHA256("")).
#
# Logging: every state transition, plugin call, HTTP roundtrip and persistence
# op is logged under the `[Attestation:<STATE>]` prefix so a full
# `cargo run -- run --target ios/android` session traces the entire flow.
# Bearer secrets (session token, PI JWS) are masked to <prefix>…(len=N).

# ---------------- signals ----------------

# Fires once per successful cycle. All awaiters of async_get_valid_jwt() wake
# with the same payload, so N concurrent callers cost one attestation.
signal jwt_refreshed(token: String, expires_at_unix: int)

# ---------------- enums ----------------

enum State { IDLE, WAITING_EULA, ATTESTING, BACKOFF }

# ---------------- constants ----------------

const SESSION_PATH := "user://attest_session.json"
const EULA_POLL_INTERVAL_SEC := 1.0
# Exponential backoff (seconds) between cycle attempts. Indexes past the last
# entry stay at 30s.
const RETRY_BACKOFF_SEC := [1.0, 2.0, 5.0, 10.0, 30.0]
# Treat tokens within EXPIRY_MARGIN_SEC of expiry as expired (clock skew +
# roundtrip buffer). Server TTL is ~48h so the margin is negligible.
const EXPIRY_MARGIN_SEC := 60
# Bearer-secret prefix length in logs: enough to distinguish session tokens
# (which share the `<b64url>.<b64url>` layout) without leaking the secret.
const _LOG_PREFIX_LEN := 12
# DEBUG override for the mobile-bff base URL while PR #54 endpoints
# (/attest/ios/challenge, /attest/session) aren't deployed to prod
# (mobile-bff.decentraland.org). Pilot deployment lives at test-auth.
# Set to "" to fall back to DclUrls.mobile_bff().
const _DEBUG_BFF_OVERRIDE := "https://test-auth.dclregenesislabs.xyz"

# ---------------- state ----------------

var _state: int = State.IDLE
var _cached_token: String = ""
var _cached_expires_at: int = 0
var _ios_plugin: Object = null
var _android_plugin: Object = null

# ---------------- public API ----------------


# Returns a valid session token. Synchronous cache hit when possible;
# otherwise dispatches a cycle (or piggybacks an in-flight one) and awaits
# jwt_refreshed. Returns "" immediately on unsupported platforms — callers
# should branch on the return value rather than awaiting forever.
func async_get_valid_jwt() -> String:
	if has_valid_session():
		var remaining: int = _cached_expires_at - int(Time.get_unix_time_from_system())
		_log(
			(
				"async_get_valid_jwt: cache HIT token=%s remaining=%ds"
				% [_secret_prefix(_cached_token), remaining]
			)
		)
		return _cached_token
	if not is_supported():
		_log("async_get_valid_jwt: unsupported platform → returning empty string")
		return ""
	_log("async_get_valid_jwt: cache MISS, dispatching kick (state=%s)" % _state_name(_state))
	_kick()
	_log("async_get_valid_jwt: awaiting jwt_refreshed signal...")
	var result: Array = await jwt_refreshed
	_log("async_get_valid_jwt: signal received → token=%s" % _secret_prefix(str(result[0])))
	return str(result[0])


# Fast sync check: is there a non-expired token in memory? Does not touch
# disk or network. Returns false within EXPIRY_MARGIN_SEC of expiry.
func has_valid_session() -> bool:
	if _cached_token.is_empty():
		return false
	return Time.get_unix_time_from_system() < _cached_expires_at - EXPIRY_MARGIN_SEC


# Invalidates the cached session and forces a fresh attestation cycle.
# Use after a downstream call (e.g. sign-message) reports the session as
# rejected — the server may have rotated its HMAC secret.
func async_force_reattest() -> String:
	_log("async_force_reattest: invalidating cache + kicking new cycle")
	_clear_session()
	if not is_supported():
		_log("async_force_reattest: unsupported platform → returning empty")
		return ""
	_kick()
	_log("async_force_reattest: awaiting jwt_refreshed signal...")
	var result: Array = await jwt_refreshed
	_log("async_force_reattest: signal received → token=%s" % _secret_prefix(str(result[0])))
	return str(result[0])


# True iff this device's OS/plugin combination can produce attestation
# artifacts. Desktop, iOS simulator, and Android without Play Services
# return false.
func is_supported() -> bool:
	if OS.get_name() == "iOS":
		return _ios_plugin != null and _ios_plugin.attestation_is_supported()
	if OS.get_name() == "Android":
		return _android_plugin != null
	return false


# ---------------- boot orchestration ----------------


func _ready() -> void:
	var platform := OS.get_name()
	var ios_singleton_present: bool = Engine.has_singleton("DclGodotiOS")
	var android_singleton_present: bool = Engine.has_singleton("dcl-godot-android")
	if platform == "iOS" and ios_singleton_present:
		_ios_plugin = Engine.get_singleton("DclGodotiOS")
	elif platform == "Android" and android_singleton_present:
		_android_plugin = Engine.get_singleton("dcl-godot-android")

	var bff_url := _bff_url()
	var session_abs := ProjectSettings.globalize_path(SESSION_PATH)
	_log(
		(
			"_ready: platform=%s ios_singleton=%s android_singleton=%s"
			% [platform, ios_singleton_present, android_singleton_present]
		)
	)
	_log("_ready: mobile_bff=%s" % bff_url)
	_log("_ready: session_path=%s" % session_abs)

	_load_session()

	var plugin_loaded: bool = _ios_plugin != null or _android_plugin != null
	var supported: bool = is_supported()
	_log(
		(
			"_ready: plugin_loaded=%s supported=%s has_session=%s"
			% [plugin_loaded, supported, has_valid_session()]
		)
	)
	if not supported:
		_log("_ready: platform not supported → skipping boot dispatch")
		return
	_async_boot_dispatch()


# Proactive boot warmup: waits for EULA, then kicks the FSM if we don't
# already have a usable session. Concurrent boot calls are absorbed by the
# WAITING_EULA state guard (_kick early-returns when state != IDLE).
func _async_boot_dispatch() -> void:
	if has_valid_session():
		var remaining: int = _cached_expires_at - int(Time.get_unix_time_from_system())
		_log("boot_dispatch: cached session valid (remaining=%ds), no kick" % remaining)
		return
	if not _is_eula_accepted():
		_log("boot_dispatch: waiting for EULA (poll every %.1fs)" % EULA_POLL_INTERVAL_SEC)
		_state = State.WAITING_EULA
		var t0 := Time.get_ticks_msec()
		while not _is_eula_accepted():
			await get_tree().create_timer(EULA_POLL_INTERVAL_SEC).timeout
		_state = State.IDLE
		_log("boot_dispatch: EULA accepted after %dms → kicking" % (Time.get_ticks_msec() - t0))
	else:
		_log("boot_dispatch: EULA already accepted → kicking immediately")
	_kick()


# Mirrors analytics_controller.gd::setup() — the canonical "EULA accepted
# on a prior run or this session" check used across the app.
func _is_eula_accepted() -> bool:
	if Global == null:
		return false
	var cfg = Global.get_config()
	if cfg == null:
		return false
	return cfg.terms_and_conditions_version == Global.TERMS_AND_CONDITIONS_VERSION


# ---------------- FSM core ----------------


# Idempotent kick. Starts a cycle iff IDLE. Skipped during WAITING_EULA
# (boot dispatch owns the kick), ATTESTING (cycle in progress), and BACKOFF
# (cycle is between attempts but still alive — its loop will retry).
func _kick() -> void:
	if _state != State.IDLE:
		_log("_kick: SKIPPED (state=%s — cycle already in flight)" % _state_name(_state))
		return
	if not is_supported():
		_log("_kick: SKIPPED (platform not supported)")
		return
	_log("_kick: starting cycle")
	_async_run_cycle()


# One attestation cycle with indefinite backoff. Exits cleanly when a token
# is obtained and jwt_refreshed is emitted. State alternates ATTESTING ↔
# BACKOFF until success, then returns to IDLE.
func _async_run_cycle() -> void:
	_state = State.ATTESTING
	var cycle_t0 := Time.get_ticks_msec()
	var attempt: int = 0
	while true:
		var attempt_t0 := Time.get_ticks_msec()
		_log(
			(
				"cycle: attempt #%d starting (cycle_elapsed=%dms)"
				% [attempt + 1, attempt_t0 - cycle_t0]
			)
		)
		var result: Dictionary = await _async_try_once()
		var attempt_ms: int = Time.get_ticks_msec() - attempt_t0
		if result.get("ok", false):
			var token: String = str(result["token"])
			var exp: int = int(result["expires_at"])
			_cached_token = token
			_cached_expires_at = exp
			_save_session(token, exp)
			_state = State.IDLE
			var seconds_left: int = exp - int(Time.get_unix_time_from_system())
			_log(
				(
					"cycle: SUCCESS attempt #%d in %dms (cycle_total=%dms) → token=%s remaining=%ds"
					% [
						attempt + 1,
						attempt_ms,
						Time.get_ticks_msec() - cycle_t0,
						_secret_prefix(token),
						seconds_left,
					]
				)
			)
			jwt_refreshed.emit(token, exp)
			return
		var idx: int = min(attempt, RETRY_BACKOFF_SEC.size() - 1)
		var delay: float = RETRY_BACKOFF_SEC[idx]
		push_warning(
			(
				"[Attestation:%s] cycle: FAIL attempt #%d in %dms (%s) → retry in %.1fs"
				% [
					_state_name(_state),
					attempt + 1,
					attempt_ms,
					str(result.get("error", "unknown")),
					delay,
				]
			)
		)
		_state = State.BACKOFF
		await get_tree().create_timer(delay).timeout
		_state = State.ATTESTING
		attempt += 1


# Single attestation attempt. Returns {ok: true, token, expires_at} or
# {ok: false, error}. Platform-dispatched.
func _async_try_once() -> Dictionary:
	var platform := OS.get_name()
	if platform == "iOS":
		return await _async_attest_ios()
	if platform == "Android":
		return await _async_attest_android()
	return {"ok": false, "error": "unsupported platform: %s" % platform}


# ---------------- iOS attestation ----------------


# Fresh App Attest key per session (PR #54 design). Sequence:
#   challenge → generateKey → attestKey(SHA256(challenge)) → POST /attest/session.
func _async_attest_ios() -> Dictionary:
	if _ios_plugin == null or not _ios_plugin.attestation_is_supported():
		return {"ok": false, "error": "iOS App Attest unsupported"}

	var bff := _bff_url()

	# 1. Server challenge.
	var t0: int = Time.get_ticks_msec()
	_log("iOS 1/4: POST %s/attest/ios/challenge" % bff)
	var ch_resp := await _async_post_json(bff + "/attest/ios/challenge", "", {})
	if ch_resp.get("__error", false):
		_log("iOS 1/4: FAIL → %s" % JSON.stringify(ch_resp))
		return {"ok": false, "error": "challenge: " + JSON.stringify(ch_resp)}
	var challenge_b64u := str(ch_resp.get("challenge", ""))
	if challenge_b64u.is_empty():
		_log("iOS 1/4: FAIL → challenge field empty")
		return {"ok": false, "error": "challenge: empty"}
	_log(
		(
			"iOS 1/4: OK in %dms (challenge_len=%d expires_at=%s)"
			% [
				Time.get_ticks_msec() - t0,
				challenge_b64u.length(),
				str(ch_resp.get("expires_at", "?")),
			]
		)
	)

	# 2. Fresh DCAppAttestService key.
	t0 = Time.get_ticks_msec()
	_log("iOS 2/4: DCAppAttestService.generateKey() (Secure Enclave)")
	_ios_plugin.attestation_generate_key()
	var key_result: Array = await _ios_plugin.attestation_key_generated
	var key_id := str(key_result[0])
	var key_err := str(key_result[1])
	if not key_err.is_empty() or key_id.is_empty():
		_log("iOS 2/4: FAIL in %dms (%s)" % [Time.get_ticks_msec() - t0, key_err])
		return {"ok": false, "error": "generateKey: " + key_err}
	_log(
		(
			"iOS 2/4: OK in %dms (key_id=%s len=%d)"
			% [Time.get_ticks_msec() - t0, _secret_prefix(key_id), key_id.length()]
		)
	)

	# 3. attestKey(key_id, SHA256(challenge_bytes)).
	t0 = Time.get_ticks_msec()
	var challenge_bytes := _b64url_decode(challenge_b64u)
	if challenge_bytes.is_empty():
		_log("iOS 3/4: FAIL → challenge decode produced empty bytes")
		return {"ok": false, "error": "challenge: decode empty"}
	var hasher := HashingContext.new()
	hasher.start(HashingContext.HASH_SHA256)
	hasher.update(challenge_bytes)
	var cdh := hasher.finish()
	_log(
		(
			"iOS 3/4: attestKey(key_id, cdh) — challenge_bytes=%d cdh=%s"
			% [challenge_bytes.size(), _bytes_hex_prefix(cdh, 8)]
		)
	)
	_ios_plugin.attestation_attest_key(key_id, cdh)
	var attest_result: Array = await _ios_plugin.attestation_attest_completed
	var attestation_object_b64u := str(attest_result[0])
	var attest_err := str(attest_result[1])
	if not attest_err.is_empty() or attestation_object_b64u.is_empty():
		_log("iOS 3/4: FAIL in %dms (%s)" % [Time.get_ticks_msec() - t0, attest_err])
		return {"ok": false, "error": "attestKey: " + attest_err}
	_log(
		(
			"iOS 3/4: OK in %dms (attestation_object_len=%d)"
			% [Time.get_ticks_msec() - t0, attestation_object_b64u.length()]
		)
	)

	# 4. Exchange for a session token.
	t0 = Time.get_ticks_msec()
	var body := (
		JSON
		. stringify(
			{
				"key_id": key_id,
				"attestation_object": attestation_object_b64u,
				"challenge": challenge_b64u,
			}
		)
	)
	_log("iOS 4/4: POST %s/attest/session (body_len=%d)" % [bff, body.length()])
	var session_resp := await _async_post_json(
		bff + "/attest/session", body, {"x-attest-platform": "ios"}
	)
	var elapsed: int = Time.get_ticks_msec() - t0
	var result := _session_response_to_result(session_resp)
	if result.get("ok", false):
		_log(
			(
				"iOS 4/4: OK in %dms (token=%s expires_at=%s)"
				% [
					elapsed,
					_secret_prefix(str(result["token"])),
					str(session_resp.get("expires_at", "?")),
				]
			)
		)
	else:
		_log("iOS 4/4: FAIL in %dms (%s)" % [elapsed, str(result.get("error", "?"))])
	return result


# ---------------- Android attestation ----------------


# Play Integrity binds the token to the request body via requestHash (PR #54).
# We send an empty body and bind the nonce to SHA256(""). Server SHA256s the
# received body and compares to the requestHash inside the verified token.
#
# Play Integrity 1.4.0+ rejects standard base64 with +/= for the nonce field
# (returns IntegrityErrorCode -13 NONCE_IS_NOT_BASE64) — must be base64url
# without padding.
func _async_attest_android() -> Dictionary:
	if _android_plugin == null:
		return {"ok": false, "error": "Android plugin missing"}

	var hasher := HashingContext.new()
	hasher.start(HashingContext.HASH_SHA256)
	var body_hash := hasher.finish()
	var request_hash_b64 := _b64url(body_hash)
	_log(
		(
			'Android 1/2: requestPlayIntegrityToken(request_hash=%s) — empty body, hash of SHA256("")'
			% request_hash_b64
		)
	)

	var t0: int = Time.get_ticks_msec()
	_android_plugin.requestPlayIntegrityToken(request_hash_b64)
	var result: Array = await _android_plugin.play_integrity_token_ready
	var pi_token := str(result[0])
	var pi_err := str(result[1])
	if not pi_err.is_empty() or pi_token.is_empty():
		_log("Android 1/2: FAIL in %dms (%s)" % [Time.get_ticks_msec() - t0, pi_err])
		return {"ok": false, "error": "playIntegrity: " + pi_err}
	_log(
		(
			"Android 1/2: OK in %dms (token=%s len=%d)"
			% [Time.get_ticks_msec() - t0, _secret_prefix(pi_token), pi_token.length()]
		)
	)

	var bff := _bff_url()
	var headers := {
		"x-attest-platform": "android",
		"x-attest-integrity-token": pi_token,
	}
	t0 = Time.get_ticks_msec()
	_log("Android 2/2: POST %s/attest/session (empty body)" % bff)
	var session_resp := await _async_post_json(bff + "/attest/session", "", headers)
	var elapsed: int = Time.get_ticks_msec() - t0
	var session_result := _session_response_to_result(session_resp)
	if session_result.get("ok", false):
		_log(
			(
				"Android 2/2: OK in %dms (token=%s expires_at=%s)"
				% [
					elapsed,
					_secret_prefix(str(session_result["token"])),
					str(session_resp.get("expires_at", "?")),
				]
			)
		)
	else:
		_log("Android 2/2: FAIL in %dms (%s)" % [elapsed, str(session_result.get("error", "?"))])
	return session_result


# ---------------- helpers ----------------


# Resolves the mobile-bff base URL. Honors _DEBUG_BFF_OVERRIDE for piloting
# against test-auth (PR #54 not in prod yet); otherwise routes through the
# normal DclUrls.mobile_bff() that respects dclenv.
func _bff_url() -> String:
	if not _DEBUG_BFF_OVERRIDE.is_empty():
		return _DEBUG_BFF_OVERRIDE
	return str(DclUrls.mobile_bff())


# Normalize a /attest/session response into the result shape used by the
# cycle. Both platforms share this — the success body is identical.
func _session_response_to_result(resp: Dictionary) -> Dictionary:
	if resp.get("__error", false):
		return {"ok": false, "error": "session: " + JSON.stringify(resp)}
	var token := str(resp.get("token", ""))
	if token.is_empty():
		return {"ok": false, "error": "session: missing token"}
	var exp := _parse_iso_to_unix(str(resp.get("expires_at", "")))
	if exp <= 0:
		return {
			"ok": false,
			"error": "session: bad expires_at (raw=%s)" % str(resp.get("expires_at", "")),
		}
	return {"ok": true, "token": token, "expires_at": exp}


# Re-uses Global.http_requester so we inherit the shared retry / 429-backoff
# stack. Returns the parsed JSON dict, or a dict with __error=true on
# transport failure or non-2xx response.
func _async_post_json(url: String, body: String, headers: Dictionary) -> Dictionary:
	var merged := {
		"Content-Type": "application/json",
		"Accept": "application/json",
	}
	for k in headers:
		merged[k] = headers[k]
	_log("HTTP POST %s (body_len=%d headers=%s)" % [url, body.length(), str(merged.keys())])
	var http_t0: int = Time.get_ticks_msec()
	var promise: Promise = Global.http_requester.request_json(
		url, HTTPClient.METHOD_POST, body, merged
	)
	var raw: Variant = await PromiseUtils.async_awaiter(promise)
	var http_ms: int = Time.get_ticks_msec() - http_t0
	if raw is PromiseError:
		var err_text: String = raw.get_error()
		_log("HTTP POST %s FAILED in %dms: %s" % [url, http_ms, err_text])
		var parsed = JSON.parse_string(err_text)
		if parsed is Dictionary:
			parsed["__error"] = true
			return parsed
		return {"error": err_text, "__error": true}
	if raw == null:
		_log("HTTP POST %s returned null in %dms" % [url, http_ms])
		return {"error": "null response", "__error": true}
	var json = raw.get_string_response_as_json()
	if json is Dictionary:
		_log(
			(
				"HTTP POST %s OK in %dms (response_keys=%s)"
				% [url, http_ms, str((json as Dictionary).keys())]
			)
		)
		return json
	_log("HTTP POST %s OK in %dms but unparseable body" % [url, http_ms])
	return {"error": "unparseable response", "__error": true}


# ---------------- persistence ----------------


func _load_session() -> void:
	if not FileAccess.file_exists(SESSION_PATH):
		_log("_load_session: no file at %s" % SESSION_PATH)
		return
	var f := FileAccess.open(SESSION_PATH, FileAccess.READ)
	if f == null:
		push_warning("[Attestation] _load_session: file exists but open failed")
		return
	var raw := f.get_as_text()
	f.close()
	var parsed = JSON.parse_string(raw)
	if not parsed is Dictionary:
		push_warning(
			"[Attestation] _load_session: file contents not a JSON dict (raw_len=%d)" % raw.length()
		)
		return
	var token := str(parsed.get("token", ""))
	var exp := int(parsed.get("expires_at", 0))
	if token.is_empty() or exp <= 0:
		push_warning(
			(
				"[Attestation] _load_session: invalid fields token_empty=%s exp=%d"
				% [token.is_empty(), exp]
			)
		)
		return
	_cached_token = token
	_cached_expires_at = exp
	var remaining: int = exp - int(Time.get_unix_time_from_system())
	_log(
		(
			"_load_session: loaded token=%s remaining=%ds platform=%s"
			% [_secret_prefix(token), remaining, str(parsed.get("platform", "?"))]
		)
	)


func _save_session(token: String, expires_at_unix: int) -> void:
	var f := FileAccess.open(SESSION_PATH, FileAccess.WRITE)
	if f == null:
		push_warning("[Attestation] could not persist session to %s" % SESSION_PATH)
		return
	(
		f
		. store_string(
			(
				JSON
				. stringify(
					{
						"token": token,
						"expires_at": expires_at_unix,
						"platform": OS.get_name().to_lower(),
					}
				)
			)
		)
	)
	f.close()
	_log(
		(
			"_save_session: wrote token=%s expires_at=%d to %s"
			% [_secret_prefix(token), expires_at_unix, SESSION_PATH]
		)
	)


func _clear_session() -> void:
	var had_token: bool = not _cached_token.is_empty()
	_cached_token = ""
	_cached_expires_at = 0
	if FileAccess.file_exists(SESSION_PATH):
		DirAccess.remove_absolute(ProjectSettings.globalize_path(SESSION_PATH))
		_log("_clear_session: removed %s (had_token=%s)" % [SESSION_PATH, had_token])
	else:
		_log("_clear_session: no file to remove (had_token=%s)" % had_token)


# ---------------- encoding ----------------


func _b64url(bytes: PackedByteArray) -> String:
	return (
		Marshalls
		. raw_to_base64(bytes)
		. replace("+", "-")
		. replace("/", "_")
		. trim_suffix("=")
		. trim_suffix("=")
	)


func _b64url_decode(s: String) -> PackedByteArray:
	var normalized := s.replace("-", "+").replace("_", "/")
	while normalized.length() % 4 != 0:
		normalized += "="
	return Marshalls.base64_to_raw(normalized)


# Server returns expires_at as ISO-8601 with milliseconds + Z suffix
# (e.g. "2026-05-22T15:30:00.000Z"). Time.get_unix_time_from_datetime_string
# treats no-timezone strings as UTC, so strip both before parsing.
func _parse_iso_to_unix(iso: String) -> int:
	if iso.is_empty():
		return 0
	var s := iso
	var z_idx := s.find("Z")
	if z_idx >= 0:
		s = s.substr(0, z_idx)
	var dot_idx := s.find(".")
	if dot_idx >= 0:
		s = s.substr(0, dot_idx)
	return int(Time.get_unix_time_from_datetime_string(s))


# ---------------- logging helpers ----------------


# Single entry point so every log line carries the current FSM state — makes
# it trivial to grep through `cargo run` output for one specific phase.
func _log(msg: String) -> void:
	print("[Attestation:%s] %s" % [_state_name(_state), msg])


func _state_name(s: int) -> String:
	match s:
		State.IDLE:
			return "IDLE"
		State.WAITING_EULA:
			return "WAITING_EULA"
		State.ATTESTING:
			return "ATTESTING"
		State.BACKOFF:
			return "BACKOFF"
	return "?"


# Masks bearer secrets: returns first _LOG_PREFIX_LEN chars + total length.
# Confirms "we have a token here" without putting the credential in logs.
func _secret_prefix(s: String) -> String:
	if s.is_empty():
		return "<empty>"
	if s.length() <= _LOG_PREFIX_LEN:
		return "<short:%d>" % s.length()
	return "%s…(len=%d)" % [s.substr(0, _LOG_PREFIX_LEN), s.length()]


# Hex-dump the first N bytes of a buffer for compact log lines. Used for
# log-friendly identification of hashes (e.g. cdh prefix) without dumping
# 64 hex chars per entry.
func _bytes_hex_prefix(bytes: PackedByteArray, n: int) -> String:
	var out := ""
	var lim: int = min(n, bytes.size())
	for i in range(lim):
		out += "%02x" % bytes[i]
	if bytes.size() > n:
		out += "…"
	out += "(len=%d)" % bytes.size()
	return out
