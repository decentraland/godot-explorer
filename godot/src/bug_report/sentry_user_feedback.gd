class_name SentryUserFeedback
extends RefCounted

## Files the reporter's message with Sentry User Feedback and returns a deep link
## for the Intercom ticket (issue #2652).
##
## Mirrors the Unity client (`SentryUserFeedbackService`): capture an event
## carrying the log tail, then a feedback entry associated with it.
##
## Unlike Unity we link to that EVENT rather than to the feedback entry. Unity
## reaches past its own facade (`HubAdapter`) to recover the feedback id; the
## Godot SDK has no equivalent — `capture_feedback()` returns nothing by
## documented design — so the feedback id is unobtainable here. The feedback
## hangs off the event, one hop from the link.
##
## Everything here is synchronous on purpose: attachments are SDK-global, so the
## window between adding and clearing them must stay as short as possible.

const DEFAULT_LOG_PATH := "user://logs/godot.log"

# Unity ships 64KB; ours is larger because the Rust/engine logging in this client
# is considerably chattier. Note Godot's FileLogger buffers, so the last few lines
# of the session may not be on disk yet.
const LOG_TAIL_BYTES := 128 * 1024

# NOT "godot.log": `options.attach_log` already uploads a file by that name for
# the 1% of sessions it samples (project_main_loop.gd), and two same-named
# attachments on one event are impossible to tell apart in the Sentry UI.
const LOG_ATTACHMENT_NAME := "bug_report_tail.log"
const LOG_CONTENT_TYPE := "text/plain"

const SCREENSHOT_ATTACHMENT_NAME := "screenshot.jpg"
const SCREENSHOT_CONTENT_TYPE := "image/jpeg"

const EVENT_MESSAGE := "Bug report diagnostics"

## Read by ProjectMainLoop._before_send, which exempts user-initiated reports from
## the remote sample-rate throttle: a sampled-away event would leave the Intercom
## ticket linking to something that was never sent.
##
## _before_send compares against these values as LITERALS — it cannot reference
## this class without breaking Godot's main-loop resolution at startup — so any
## change here must be mirrored there by hand.
const CATEGORY_TAG_KEY := "category"
const CATEGORY_TAG_VALUE := "FEEDBACK"

# Org and project are deployment values, not secrets — the same pair appears in
# every Sentry URL. Project `godot-explorer` is DSN project id 4510187688361984.
const ISSUE_URL_TEMPLATE := "https://dcl-regenesis-labs.sentry.io/issues/?query=%s"


## Returns a deep link to the event carrying the diagnostics, or "" when Sentry is
## disabled or dropped the event.
##
## Total by design: every branch returns a String and nothing raises. A bug report
## must still reach Intercom when diagnostics fail, which is what the Unity client
## does too — the caller falls back to "unavailable".
static func submit(message: String, jpeg_bytes: PackedByteArray) -> String:
	if not SentrySDK.is_enabled():
		return ""
	if message.strip_edges().is_empty():
		return ""

	var event_id := _capture_event(jpeg_bytes)
	if event_id.is_empty():
		return ""

	var feedback := SentryFeedback.new()
	feedback.set_message(message)
	feedback.set_associated_event_id(event_id)
	# No contact_email: the form has no email field by design. Identity already
	# rides on the event — sentry_seeder.gd pins user id/username, realm and
	# parcel onto everything we send — so the name is for legibility only.
	var reporter := _reporter_name()
	if not reporter.is_empty():
		feedback.set_name(reporter)
	SentrySDK.capture_feedback(feedback)

	return ISSUE_URL_TEMPLATE % event_id


# Attachments are SDK-global rather than per-event, so any event captured between
# the first add and the clear inherits them. Hence: no `await` in this call path,
# and `clear_attachments()` runs unconditionally right after the capture, before
# any return. If a second caller ever adds attachments this needs save/restore.
static func _capture_event(jpeg_bytes: PackedByteArray) -> String:
	var tail := _read_log_tail()
	if not tail.is_empty():
		var log_attachment := SentryAttachment.create_with_bytes(tail, LOG_ATTACHMENT_NAME)
		log_attachment.set_content_type(LOG_CONTENT_TYPE)
		SentrySDK.add_attachment(log_attachment)

	# The proxy rejects an image over its 3MB cap and drops the whole ticket with
	# it, so the screenshot rides to Sentry as well and survives there even when
	# the ticket has to go without it. Same reasoning as Unity's
	# SelectEvidenceImage.
	if not jpeg_bytes.is_empty():
		var shot := SentryAttachment.create_with_bytes(jpeg_bytes, SCREENSHOT_ATTACHMENT_NAME)
		shot.set_content_type(SCREENSHOT_CONTENT_TYPE)
		SentrySDK.add_attachment(shot)

	var event := SentrySDK.create_event()
	event.set_message(EVENT_MESSAGE)
	event.set_level(SentrySDK.LEVEL_INFO)
	event.set_tag(CATEGORY_TAG_KEY, CATEGORY_TAG_VALUE)

	var raw_id := SentrySDK.capture_event(event)
	SentrySDK.clear_attachments()

	return _normalize_event_id(raw_id)


# Sentry emits ids in both the dashed (36-char) and simple (32-hex) UUID forms,
# while event search and `set_associated_event_id()` both want the dashless one.
# Returns "" for the nil id, which is what a dropped event yields — on dev builds
# _before_send discards everything, so this path is load-bearing, not defensive.
static func _normalize_event_id(raw: String) -> String:
	var normalized := raw.replace("-", "").strip_edges().to_lower()
	# Matches the nil id in either form; a literal of 32 zeros would not.
	if normalized.lstrip("0").is_empty():
		return ""
	return normalized


static func _read_log_tail() -> PackedByteArray:
	var path := str(ProjectSettings.get_setting("debug/file_logging/log_path", DEFAULT_LOG_PATH))
	var file := FileAccess.open(path, FileAccess.READ)
	# Absent on a first run and lockable on some platforms. Never push_error here:
	# SentryGodotLogger captures those, so complaining would recurse.
	if file == null:
		return PackedByteArray()

	var length := file.get_length()
	if length <= 0:
		file.close()
		return PackedByteArray()

	var truncated := length > LOG_TAIL_BYTES
	if truncated:
		file.seek(length - LOG_TAIL_BYTES)
	var bytes := file.get_buffer(mini(length, LOG_TAIL_BYTES))
	file.close()

	# A tail cut at a byte offset starts mid-line, and possibly mid-UTF8 sequence.
	# Drop through the first newline so the attachment opens cleanly.
	if truncated:
		var newline := bytes.find(10)
		if newline >= 0 and newline + 1 < bytes.size():
			bytes = bytes.slice(newline + 1)

	return bytes


static func _reporter_name() -> String:
	if Global.player_identity == null:
		return ""
	var profile = Global.player_identity.get_profile_or_null()
	if profile == null:
		return ""
	return profile.get_name()
