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

# Raw bytes, before base64. The proxy rejects the whole ticket above this, so an
# oversized image is dropped rather than allowed to sink the report.
const MAX_EVIDENCE_BYTES := 3 * 1024 * 1024

const JPEG_QUALITY := 0.85

# Matches the Figma character counter.
const DESCRIPTION_MAX_LENGTH := 300


## Files a bug report. `images` may be empty; only the first is attached, because
## the proxy's `evidence` field holds a single image.
##
## Returns {ok: bool, id: String, error: String}. Never throws.
static func async_submit(
	issue_type_uuid: String, description: String, images: Array = []
) -> Dictionary:
	if issue_type_uuid.is_empty():
		return {"ok": false, "id": "", "error": "missing issue type"}

	var trimmed := description.strip_edges()
	if trimmed.is_empty():
		return {"ok": false, "id": "", "error": "missing description"}

	# Encoded once and shared: the Sentry copy matters most precisely when the
	# ticket has to go without the image, so re-encoding here would both double
	# the work on the UI thread and defeat the reason for attaching it.
	var jpeg_bytes := _encode_image(images)

	# Before the POST: the returned link is an input to the description. Returns
	# "" whenever Sentry is unavailable, and the report is filed regardless.
	var diagnostics_link := SentryUserFeedback.submit(trimmed, jpeg_bytes)

	var payload := {
		"ticket_attributes": _build_attributes(issue_type_uuid, trimmed, diagnostics_link)
	}

	var evidence := _build_evidence(jpeg_bytes)
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


# Every value must be a String — the proxy rejects non-string attribute values.
static func _build_attributes(
	issue_type_uuid: String, description: String, diagnostics_link: String
) -> Dictionary:
	var device := _collect_device_info()
	var attributes := {
		"_default_title_": "Bug Report: %s" % _label_for_uuid(issue_type_uuid),
		"_default_description_": _compose_description(description, diagnostics_link),
		"Issue Type": issue_type_uuid,
		"Operating System": device["os"],
		"Graphic Card": device["gpu"],
		"RAM": device["ram"],
		"Client version": String(DclGlobal.get_version()),
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
static func _compose_description(description: String, diagnostics_link: String) -> String:
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
	var diagnostics := diagnostics_link if not diagnostics_link.is_empty() else "unavailable"
	lines.append("Internal diagnostics: %s" % diagnostics)
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


# The first image only, as JPEG bytes; empty when there is nothing to attach or
# the encode fails. Split out from _build_evidence so the same bytes can also go
# to Sentry, which is the only place an over-cap image survives.
static func _encode_image(images: Array) -> PackedByteArray:
	if images.is_empty():
		return PackedByteArray()
	if images[0] == null or not (images[0] is Image):
		return PackedByteArray()
	var image: Image = images[0]
	if image.is_empty():
		return PackedByteArray()

	var bytes: PackedByteArray = image.save_jpg_to_buffer(JPEG_QUALITY)
	if bytes.is_empty():
		push_warning("BugReportService: could not encode the attached image")
	return bytes


# Returns {} when there is nothing to attach or the encode is too large — an
# oversized image must not take the whole ticket down with it. It still reaches
# Sentry via SentryUserFeedback, so it is dropped from the ticket, not lost.
static func _build_evidence(bytes: PackedByteArray) -> Dictionary:
	if bytes.is_empty():
		return {}
	if bytes.size() > MAX_EVIDENCE_BYTES:
		push_warning(
			(
				"BugReportService: attachment is %d bytes (max %d) — filing without it"
				% [bytes.size(), MAX_EVIDENCE_BYTES]
			)
		)
		return {}

	return {"content_type": "image/jpeg", "data": Marshalls.raw_to_base64(bytes)}


static func _label_for_uuid(uuid: String) -> String:
	for category in BugReportCategories.CATEGORIES:
		if category["uuid"] == uuid:
			return category["label"]
	return "Other"
