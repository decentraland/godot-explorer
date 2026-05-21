class_name AttestationServiceImpl
extends Node

# Platform attestation service — produces the headers consumed by the mobile-bff
# /v1/attest/check endpoint (and any sign-message proxy that wants to gate on
# attestation). Two flows, picked by OS.get_name():
#
#   iOS:     one-time enrollment (App Attest key + register with backend),
#            then per-request CBOR assertion signed over SHA256(body || nonce).
#   Android: stateless. Per-request Play Integrity token bound to
#            base64url(SHA256(body)) as the nonce field.
#
# Lifecycle:
#   - Instantiated by Global._ready as a child Node.
#   - On boot: if `_is_validated()` (persisted marker present) → no-op.
#   - Otherwise: poll Global.get_config().terms_and_conditions_version every 1s
#     until EULA is accepted, then run validation against the backend. On
#     success → persist the marker so future launches skip. On failure → retry
#     with exponential backoff [1,2,5,10,30]s, capped at 30s indefinitely.
#
# Persistence:
#   - iOS key_id: `user://attest_ios_key_id.txt`. Plaintext is fine — the key_id
#     is not a secret; the private half is sealed in the Secure Enclave.
#   - Validated marker: `user://attest_validated.txt`. Presence means "this
#     install passed /v1/attest/check at least once".

const KEY_ID_PATH := "user://attest_ios_key_id.txt"
const VALIDATED_PATH := "user://attest_validated.txt"
const NONCE_LEN := 16
const ENROLL_TIMEOUT_SEC := 30
const ATTEST_CHECK_URL := "https://test-auth.dclregenesislabs.xyz"
const EULA_POLL_INTERVAL_SEC := 1.0
# Exponential backoff (seconds) for validation retries once EULA is accepted.
# Indexes past the last entry stay at 30s.
const RETRY_BACKOFF_SEC := [1.0, 2.0, 5.0, 10.0, 30.0]

# Set to true to skip reading/writing the persisted key_id so every session
# re-runs the full iOS enrollment ceremony. Useful while iterating on the
# server-side verifier.
const _DEBUG_DISABLE_PERSIST := false

var _enrollment_lock: bool = false
var _validating: bool = false
var _ios_plugin: Object = null
var _android_plugin: Object = null


func _ready() -> void:
	var platform := OS.get_name()
	if platform == "iOS":
		if Engine.has_singleton("DclGodotiOS"):
			_ios_plugin = Engine.get_singleton("DclGodotiOS")
	elif platform == "Android":
		if Engine.has_singleton("dcl-godot-android"):
			_android_plugin = Engine.get_singleton("dcl-godot-android")
	if _is_validated():
		return
	if not is_supported():
		return
	async_validate_when_eula_accepted()


# Returns true if this install already passed /v1/attest/check at least once,
# so no further attestation work is needed for the remaining lifetime of this
# install (until user data is wiped).
func _is_validated() -> bool:
	return FileAccess.file_exists(VALIDATED_PATH)


func _mark_validated() -> void:
	var f := FileAccess.open(VALIDATED_PATH, FileAccess.WRITE)
	if f == null:
		push_warning("[Attestation] could not persist validated marker to %s" % VALIDATED_PATH)
		return
	f.store_string("ok")
	f.close()


# Mirrors `analytics_controller.gd::setup()` — the canonical "EULA accepted on a
# prior run OR just-accepted in this session" check used across the app.
func _is_eula_accepted() -> bool:
	if Global == null:
		return false
	var cfg = Global.get_config()
	if cfg == null:
		return false
	return cfg.terms_and_conditions_version == Global.TERMS_AND_CONDITIONS_VERSION


# Polls EULA acceptance every EULA_POLL_INTERVAL_SEC. Once accepted, runs the
# validation flow with exponential backoff on failure. Persists the validated
# marker on first success and exits the loop. Safe to call multiple times — a
# second concurrent invocation is a no-op.
func async_validate_when_eula_accepted() -> void:
	if _validating:
		return
	_validating = true
	while not _is_eula_accepted():
		await get_tree().create_timer(EULA_POLL_INTERVAL_SEC).timeout
	var attempt: int = 0
	while true:
		var ok: bool = await _async_try_validate()
		if ok:
			_mark_validated()
			_validating = false
			return
		var idx: int = min(attempt, RETRY_BACKOFF_SEC.size() - 1)
		var delay: float = RETRY_BACKOFF_SEC[idx]
		await get_tree().create_timer(delay).timeout
		attempt += 1


# Runs one validation round: produces attestation headers + POSTs to
# /v1/attest/check. Returns true iff the backend responds with `ok: true`.
func _async_try_validate() -> bool:
	var body := PackedByteArray()
	var headers := await async_get_attestation_headers(body, ATTEST_CHECK_URL)
	if headers.is_empty():
		push_warning("[Attestation] no headers produced; cannot validate")
		return false
	var resp: Dictionary = await _async_post_json(
		ATTEST_CHECK_URL + "/v1/attest/check", "", headers
	)
	var ok: bool = bool(resp.get("ok", false))
	var platform: String = str(resp.get("platform", "?"))
	var elapsed: String = str(resp.get("elapsed_ms", "?"))
	if ok:
		print("[Attestation] validated platform=%s elapsed_ms=%s" % [platform, elapsed])
	else:
		var code: String = str(resp.get("code", "?"))
		var err: String = str(resp.get("error", ""))
		push_warning(
			(
				"[Attestation] validation failed platform=%s code=%s elapsed_ms=%s error=%s"
				% [platform, code, elapsed, err]
			)
		)
	return ok


# True if this device's OS/plugin combination can produce attestation
# headers. On unsupported platforms (desktop dev, iOS simulator, Android
# without Play Services) returns false — callers should either skip the
# attest call or report the platform as unsupported.
func is_supported() -> bool:
	if OS.get_name() == "iOS":
		return _ios_plugin != null and _ios_plugin.attestation_is_supported()
	if OS.get_name() == "Android":
		return _android_plugin != null
	return false


# Force a fresh iOS enrollment, ignoring any persisted key_id. Useful when
# the server side reports our key as unknown (server restart wiped the
# in-memory store; user data was wiped; Secure Enclave reset). Callers
# typically invoke this on a 401 ATTESTATION_IOS_KEY_NOT_REGISTERED.
# Returns the new key_id on success, "" on failure.
func async_force_reenroll_ios(backend_url: String) -> String:
	if OS.get_name() != "iOS":
		return ""
	print("[Attestation] forcing iOS re-enrollment")
	_clear_ios_key_id()
	return await _async_enroll_ios(backend_url)


# Returns the headers to attach to a request whose body the caller has
# already finalized. Empty dict means "couldn't produce headers" — callers
# can still send the request unauthenticated and let the backend decide
# (the /v1/attest/check endpoint always responds 200, reporting the failure).
#
# `backend_url` is only used for the iOS enrollment ceremony (one-time per
# install). Per-request assertion generation is fully local.
func async_get_attestation_headers(body_bytes: PackedByteArray, backend_url: String) -> Dictionary:
	var platform := OS.get_name()
	if platform == "iOS":
		return await _async_get_ios_headers(body_bytes, backend_url)
	if platform == "Android":
		return await _async_get_android_headers(body_bytes)
	push_warning("[Attestation] platform=%s not supported; sending unauthenticated" % platform)
	return {}


# ---------------- iOS ----------------


func _async_get_ios_headers(body_bytes: PackedByteArray, backend_url: String) -> Dictionary:
	if _ios_plugin == null:
		push_warning("[Attestation] iOS plugin singleton missing")
		return {}
	if not _ios_plugin.attestation_is_supported():
		push_warning("[Attestation] App Attest unsupported on this device")
		return {}

	var key_id := _load_ios_key_id()
	if key_id.is_empty():
		key_id = await _async_enroll_ios(backend_url)
		if key_id.is_empty():
			return {}

	var crypto := Crypto.new()
	var nonce_bytes := crypto.generate_random_bytes(NONCE_LEN)
	var hasher := HashingContext.new()
	hasher.start(HashingContext.HASH_SHA256)
	hasher.update(body_bytes)
	hasher.update(nonce_bytes)
	var client_data_hash := hasher.finish()

	_ios_plugin.attestation_generate_assertion(key_id, client_data_hash)
	var result: Array = await _ios_plugin.attestation_assertion_completed
	var assertion_b64u := str(result[0])
	var error := str(result[1])
	if not error.is_empty() or assertion_b64u.is_empty():
		push_warning("[Attestation] iOS generateAssertion failed: %s" % error)
		# Wipe and re-enroll on the next call — a corrupted/invalidated key
		# (user reinstalled, Secure Enclave reset) won't recover on retry.
		if "key" in error.to_lower() and "found" in error.to_lower():
			_clear_ios_key_id()
		return {}

	return {
		"x-attest-platform": "ios",
		"x-attest-key-id": key_id,
		"x-attest-assertion": assertion_b64u,
		"x-attest-nonce": _b64url(nonce_bytes),
	}


func _async_enroll_ios(backend_url: String) -> String:
	if _enrollment_lock:
		push_warning("[Attestation] iOS enrollment already in progress; skipping concurrent call")
		return ""
	_enrollment_lock = true

	var key_id: String = ""
	var ok := false
	# The whole ceremony is wrapped so we always release the lock.
	while true:
		# 1. Server challenge.
		var challenge_resp := await _async_post_json(
			backend_url + "/v1/attest/ios/challenge", "{}", {}
		)
		if challenge_resp.get("__error", false):
			push_warning("[Attestation] iOS /challenge failed: " + JSON.stringify(challenge_resp))
			break
		var challenge_b64u := str(challenge_resp.get("challenge", ""))
		if challenge_b64u.is_empty():
			push_warning("[Attestation] iOS /challenge returned no challenge field")
			break

		# 2. DCAppAttestService.generateKey().
		_ios_plugin.attestation_generate_key()
		var key_result: Array = await _ios_plugin.attestation_key_generated
		var new_key_id := str(key_result[0])
		var key_error := str(key_result[1])
		if not key_error.is_empty() or new_key_id.is_empty():
			push_warning("[Attestation] iOS generateKey failed: %s" % key_error)
			break

		# 3. attestKey(keyId, SHA256(challenge_bytes)).
		var challenge_bytes := _b64url_decode(challenge_b64u)
		if challenge_bytes.is_empty():
			push_warning("[Attestation] iOS challenge bytes empty after decode")
			break
		var hasher := HashingContext.new()
		hasher.start(HashingContext.HASH_SHA256)
		hasher.update(challenge_bytes)
		var cdh := hasher.finish()

		_ios_plugin.attestation_attest_key(new_key_id, cdh)
		var attest_result: Array = await _ios_plugin.attestation_attest_completed
		var attestation_object_b64u := str(attest_result[0])
		var attest_error := str(attest_result[1])
		if not attest_error.is_empty() or attestation_object_b64u.is_empty():
			push_warning("[Attestation] iOS attestKey failed: %s" % attest_error)
			break

		# 4. POST /attest/ios/register with the attestation object.
		var register_body := (
			JSON
			. stringify(
				{
					"key_id": new_key_id,
					"attestation_object": attestation_object_b64u,
					"challenge": challenge_b64u,
				}
			)
		)
		var register_resp := await _async_post_json(
			backend_url + "/v1/attest/ios/register", register_body, {}
		)
		if register_resp.get("__error", false) or not register_resp.get("registered", false):
			push_warning("[Attestation] iOS /register failed: " + JSON.stringify(register_resp))
			break

		_save_ios_key_id(new_key_id)
		key_id = new_key_id
		ok = true
		break

	_enrollment_lock = false
	if ok:
		print("[Attestation] iOS enrollment OK: key_id_prefix=%s" % key_id.substr(0, 8))
	return key_id


func _load_ios_key_id() -> String:
	if _DEBUG_DISABLE_PERSIST:
		return ""
	if not FileAccess.file_exists(KEY_ID_PATH):
		return ""
	var f := FileAccess.open(KEY_ID_PATH, FileAccess.READ)
	if f == null:
		return ""
	var s := f.get_as_text().strip_edges()
	f.close()
	return s


func _save_ios_key_id(key_id: String) -> void:
	if _DEBUG_DISABLE_PERSIST:
		return
	var f := FileAccess.open(KEY_ID_PATH, FileAccess.WRITE)
	if f == null:
		push_warning("[Attestation] could not persist key_id to %s" % KEY_ID_PATH)
		return
	f.store_string(key_id)
	f.close()


func _clear_ios_key_id() -> void:
	if FileAccess.file_exists(KEY_ID_PATH):
		DirAccess.remove_absolute(ProjectSettings.globalize_path(KEY_ID_PATH))


# ---------------- Android ----------------


func _async_get_android_headers(body_bytes: PackedByteArray) -> Dictionary:
	if _android_plugin == null:
		push_warning("[Attestation] Android plugin singleton missing")
		return {}

	# Play Integrity requires URL-safe base64 *without* padding for the nonce
	# field (PlayIntegrity 1.4.0 returns IntegrityErrorCode -13
	# NONCE_IS_NOT_BASE64 for standard base64 with +/=). The backend's
	# decodeFlexibleBase64() normalizes either form back to bytes for the
	# hash comparison, so this end of the chain is what needs the fix.
	var hasher := HashingContext.new()
	hasher.start(HashingContext.HASH_SHA256)
	hasher.update(body_bytes)
	var body_hash := hasher.finish()
	var request_hash_b64 := _b64url(body_hash)

	_android_plugin.requestPlayIntegrityToken(request_hash_b64)
	var result: Array = await _android_plugin.play_integrity_token_ready
	var token := str(result[0])
	var error := str(result[1])
	if not error.is_empty() or token.is_empty():
		push_warning("[Attestation] Play Integrity failed: %s" % error)
		return {}

	return {
		"x-attest-platform": "android",
		"x-attest-integrity-token": token,
	}


# ---------------- helpers ----------------


# Re-uses Global.http_requester so we get the same retry / 429-backoff stack
# the rest of the auth flow goes through. The body is sent verbatim; the
# attestation challenge endpoint takes "{}" so there's no body-hashing
# concern for /challenge or /register.
func _async_post_json(url: String, body: String, headers: Dictionary) -> Dictionary:
	var merged := {
		"Content-Type": "application/json",
		"Accept": "application/json",
	}
	for k in headers:
		merged[k] = headers[k]
	var promise: Promise = Global.http_requester.request_json(
		url, HTTPClient.METHOD_POST, body, merged
	)
	var raw: Variant = await PromiseUtils.async_awaiter(promise)
	if raw is PromiseError:
		var err_text: String = raw.get_error()
		var parsed = JSON.parse_string(err_text)
		if parsed is Dictionary:
			parsed["__error"] = true
			return parsed
		return {"error": err_text, "__error": true}
	if raw == null:
		return {"error": "null response", "__error": true}
	var json = raw.get_string_response_as_json()
	if json is Dictionary:
		return json
	return {"error": "unparseable response", "__error": true}


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
