class_name BugReportService
extends RefCounted

## Submits native bug reports as Intercom tickets (issue #2652).
##
## Talks to the Decentraland intercom-proxy, which holds the Intercom workspace
## token and forwards ticket creation — the client never calls Intercom directly.
## The contract mirrors the Unity Explorer client (`IntercomTicketClient.cs`), so
## mobile and desktop tickets are shaped identically.
##
##     POST https://intercom-proxy.decentraland.<env>/intercom/tickets
##     Origin: https://play.decentraland.<env>      # allowlisted; 403 otherwise
##     x-identity-auth-chain-*                      # signed fetch, metadata "{}"
##
## The body accepts ONLY `ticket_attributes` and `evidence` at the top level, and
## every attribute name must be declared on the Bug Report ticket type. An extra
## or misspelled key gets the entire ticket rejected, so nothing here is
## speculative — attributes match Unity's exactly.

# Raw bytes, before base64. The proxy rejects the whole ticket if one image is over
# this, so an oversized image is shrunk or dropped rather than sinking the report.
const MAX_EVIDENCE_BYTES := 3 * 1024 * 1024

const MAX_EVIDENCE_COUNT := 3

# Raw bytes across all images. The proxy caps the whole BODY at 4.5MB, and base64
# grows data by 4/3: 3MiB raw is ~4.2MB encoded, which leaves room for the
# attributes.
const TOTAL_EVIDENCE_BYTES := 3 * 1024 * 1024

# Intercom's "Platform" list attribute: 1 Desktop, 2 Mobile. This client only
# ships on mobile, so it is always Mobile.
const PLATFORM_MOBILE := 2

const JPEG_QUALITY := 0.85

# Matches the Figma character counter.
const DESCRIPTION_MAX_LENGTH := 300


## Files a bug report with up to MAX_EVIDENCE_COUNT screenshots (may be empty).
##
## Each shot is `{bytes, image}`, the shape BugReportModal keeps: `bytes` is the
## JPEG encoded when it was captured or picked, `image` its decoded preview.
## Screenshots that fit their share of the body are sent as-is, so the usual
## submit does no image work (PR #2779 review). Only an oversized one is
## re-encoded from `image`, one frame per image so the spinner keeps animating
## (PR #2906 review).
##
## Returns {ok: bool, id: String, error: String}. Never throws.
static func async_submit(
	issue_type_uuid: String, description: String, shots: Array[Dictionary] = []
) -> Dictionary:
	if issue_type_uuid.is_empty():
		return {"ok": false, "id": "", "error": "missing issue type"}

	var trimmed := description.strip_edges()
	if trimmed.is_empty():
		return {"ok": false, "id": "", "error": "missing description"}

	# Before the POST: the returned links are inputs to the description. They are
	# "" whenever Sentry is unavailable, and the report is filed regardless.
	var images: Array[PackedByteArray] = []
	for shot in shots:
		images.append(shot.get("bytes", PackedByteArray()))
	var sentry_links := SentryUserFeedback.submit(trimmed, images)

	var payload := {"ticket_attributes": _build_attributes(issue_type_uuid, trimmed, sentry_links)}

	var evidence := await _async_build_evidence(shots)
	if not evidence.is_empty():
		payload["evidence"] = evidence

	var url := String(DclUrls.intercom_tickets())
	var body := JSON.stringify(payload)

	# The proxy signs `{}` and leaves the body unsigned, unlike most DCL services.
	var response = await Global.async_signed_fetch(
		url, HTTPClient.METHOD_POST, body, "{}", {"Origin": String(DclUrls.intercom_origin())}
	)

	if response is PromiseError:
		var message: String = response.get_error()
		push_warning("BugReportService: request failed: %s" % message)
		return {"ok": false, "id": "", "error": message}

	var json = response.get_string_response_as_json()
	if json == null or typeof(json) != TYPE_DICTIONARY:
		push_warning("BugReportService: unexpected response body")
		return {"ok": false, "id": "", "error": "unexpected response"}

	# The proxy echoes Intercom's ticket object; only the id matters. Its absence
	# means the ticket wasn't created even if the transport succeeded.
	var ticket_id := str(json.get("id", ""))
	if ticket_id.is_empty():
		var err := str(json.get("error", "ticket not created"))
		push_warning("BugReportService: %s" % err)
		return {"ok": false, "id": "", "error": err}

	return {"ok": true, "id": ticket_id, "error": ""}


# Every value is a String except Platform, which the proxy expects as an int.
static func _build_attributes(
	issue_type_uuid: String, description: String, sentry_links: Dictionary
) -> Dictionary:
	var device := _collect_device_info()
	var attributes := {
		"_default_title_": "Bug Report: %s" % _label_for_uuid(issue_type_uuid),
		"_default_description_": _compose_description(description, sentry_links),
		"Issue Type": issue_type_uuid,
		"Operating System": device["os"],
		"Graphic Card": device["gpu"],
		"RAM": device["ram"],
		"Client version": String(DclGlobal.get_version()),
		# A raw int, not an option-id string like Issue Type: the intercom-proxy
		# contract (issue #2842) defines Platform as 1 Desktop / 2 Mobile.
		"Platform": PLATFORM_MOBILE,
	}

	# Omitted rather than sent empty: Intercom leaves an absent attribute unset,
	# while "" renders as a filled-in blank. Same rule the Unity client applies.
	var sdk_version := _current_scene_sdk_version()
	if not sdk_version.is_empty():
		attributes["SDK version"] = sdk_version

	return attributes


# `runtimeVersion` of the scene the reporter is standing in ("7" for SDK7),
# mirroring Unity's SceneSdkVersion. Empty in the lobby, where there is no scene.
static func _current_scene_sdk_version() -> String:
	var fetcher = Global.scene_fetcher
	if fetcher == null:
		return ""
	var scene_data = fetcher.get_current_scene_data()
	if scene_data == null or scene_data.scene_entity_definition == null:
		return ""
	return String(scene_data.scene_entity_definition.get_runtime_version())


# Mirrors Unity's ComposeTicketDescription so both clients read the same in
# Intercom. `Internal diagnostics` is a Sentry deep link to the event carrying the
# log tail and screenshot; it falls back to "unavailable" — the same string Unity
# emits when its Sentry step fails — whenever SentryUserFeedback returns nothing,
# which is every dev build, since _before_send discards those events.
# `Sentry feedback` is Godot-only: the Feedback page filtered to the reporter (see
# SentryUserFeedback), omitted rather than "unavailable" when there is none.
static func _compose_description(description: String, sentry_links: Dictionary) -> String:
	var lines := [description, "", "---"]
	# Only in-world. `last_parcel_position` is persisted spawn config (config_data.gd),
	# not a live position — explorer.gd writes it as the player moves and reads it back
	# to pick a spawn. In the lobby it therefore still holds the previous session's
	# parcel, or the (72,-10) default on a fresh install, and reporting a parcel the
	# player is demonstrably not standing in is worse than reporting none. Omitted
	# rather than blanked, matching how the optional ticket attributes behave.
	if Global.get_explorer() != null:
		var position = Global.get_config().last_parcel_position
		if position != null:
			lines.append("Coordinates: %d,%d" % [position.x, position.y])
	var event_url := str(sentry_links.get("event_url", ""))
	lines.append(
		"Internal diagnostics: %s" % (event_url if not event_url.is_empty() else "unavailable")
	)
	var feedback_url := str(sentry_links.get("feedback_url", ""))
	if not feedback_url.is_empty():
		lines.append("Sentry feedback: %s" % feedback_url)
	return "\n".join(lines)


# Ported from the Google-Form flow this replaces, which gathered the same fields.
static func _collect_device_info() -> Dictionary:
	var os_version := OS.get_name()
	var ram := ""
	var brand := ""
	var model := ""

	var info: Dictionary = {}
	if DclAndroidPlugin.is_available():
		var android = Engine.get_singleton("dcl-godot-android")
		if android != null:
			info = android.getMobileDeviceInfo()
	elif DclIosPlugin.is_available():
		var ios = Engine.get_singleton("DclGodotiOS")
		if ios != null:
			info = ios.get_mobile_device_info()

	if not info.is_empty():
		brand = str(info.get("device_brand", ""))
		model = str(info.get("device_model", ""))
		os_version = str(info.get("os_version", os_version))
		var total_ram := int(info.get("total_ram_mb", -1))
		if total_ram > 0:
			ram = "%d MB" % total_ram

	# Brand/model have no attribute of their own on the ticket type, so they ride
	# along in Operating System — the only field that distinguishes a mobile
	# ticket from a desktop one.
	var device := " ".join([brand, model]).strip_edges()
	if not device.is_empty():
		os_version = "%s (%s)" % [os_version, device]

	return {
		"os": os_version,
		"gpu": RenderingServer.get_video_adapter_name(),
		"ram": ram,
	}


# One evidence entry per screenshot, in order. The body budget is shared out as
# we go: each image gets the remaining bytes divided by the images still to come,
# so one that comes in small or gets dropped leaves more room for the rest. An
# image over its share is re-encoded smaller, and dropped only when even that
# fails, so it can't take the whole ticket down with it. The originals still
# reach Sentry via SentryUserFeedback, so a dropped image is missing from the
# ticket but not lost.
static func _async_build_evidence(shots: Array[Dictionary]) -> Array:
	var present: Array[Dictionary] = []
	for shot in shots:
		var bytes: PackedByteArray = shot.get("bytes", PackedByteArray())
		if not bytes.is_empty():
			present.append(shot)
	if present.size() > MAX_EVIDENCE_COUNT:
		present.resize(MAX_EVIDENCE_COUNT)

	var evidence: Array = []
	var remaining_bytes := TOTAL_EVIDENCE_BYTES
	for i in present.size():
		var budget := mini(MAX_EVIDENCE_BYTES, remaining_bytes / (present.size() - i))
		var bytes: PackedByteArray = present[i]["bytes"]
		var fitted := bytes
		if fitted.size() > budget:
			# Yield first: each re-encode is a few full-size JPEG encodes on the
			# main thread, and the spinner should paint between them.
			var tree := Engine.get_main_loop() as SceneTree
			if tree != null:
				await tree.process_frame
			var image: Image = present[i].get("image")
			if image == null:
				image = ImagePickerService.decode(bytes)
			fitted = BugReportCapture.encode_within(image, budget)
		if fitted.is_empty():
			push_warning(
				(
					"BugReportService: attachment is %d bytes (budget %d) — filing without it"
					% [bytes.size(), budget]
				)
			)
			continue
		remaining_bytes -= fitted.size()
		evidence.append({"content_type": "image/jpeg", "data": Marshalls.raw_to_base64(fitted)})
	return evidence


static func _label_for_uuid(uuid: String) -> String:
	for category in BugReportCategories.CATEGORIES:
		if category["uuid"] == uuid:
			return category["label"]
	return "Other"
