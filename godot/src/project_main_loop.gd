class_name ProjectMainLoop
extends SceneTree

# Sample which sessions upload godot.log. Without this, every event re-uploads
# the entire log file (default attach_log=true) and tight error loops blow up
# the Sentry attachment quota.
const ATTACH_LOG_SAMPLE_RATE := 0.01

# Substring patterns for messages classified as Sentry noise. These all
# originate in Godot internals, GPU drivers, or third-party crates
# (livekit-rust); we can't act on them and they fire in tight loops,
# dominating our quota.
const NOISE_PATTERNS := [
	"VK_SUCCESS",
	"vkWaitForFences",
	"QueuePresentKHR",
	"Uniforms supplied",
	"p_mipmap",
	"det == 0",
	"!is_inside_tree",
	"err != OK",
	"Bones array",
	"Skin bind",
	"must be a normalized",
	"Mouse is not supported",
	"utf16 surrogate",
	"ClientMessagesHandler",
	"StreamProtocol",
	'Condition "active"',
]
# Keep this fraction of noise events as a canary — if the shape or volume of
# engine/driver errors shifts we want to notice, but 100% is wasted quota.
const NOISE_KEEP_RATE := 0.05

# Error-sampling rate used until the `sentry-sample-rate` feature flag loads —
# also the effective rate when the fetch fails or the flag is absent. The bff
# can raise it (currently serves 1.0) or lower it to 0.0 as a kill switch.
const DEFAULT_SENTRY_SAMPLE_RATE := 0.1

# Environment detection based on version string suffix
var is_dev_version = false
var is_staging_version = false
var is_prod_version = false

var attach_log_sampled := false

# Remote error-sampling rate from the `sentry-sample-rate` feature flag — lets
# us throttle Sentry volume without shipping a build. Enforced in _before_send
# because the flag fetch resolves after SentrySDK.init already ran.
var sentry_sample_rate := DEFAULT_SENTRY_SAMPLE_RATE


func _initialize() -> void:
	# Skip Sentry init when telemetry is disabled at build time
	# (e.g. CI desktop builds compiled with the `disable_telemetry` cargo feature).
	if DclGlobal.is_telemetry_disabled():
		return

	# Clean semver (`{version}+{build}`) so Sentry parses release.version / release.build.
	# Commit hash and env are carried separately via `dist` / `environment` below.
	var release_string = "org.decentraland.godotexplorer@" + DclGlobal.get_sentry_release()

	# Detect environment from version string
	self.is_dev_version = DclGlobal.is_dev()
	self.is_staging_version = DclGlobal.is_staging()
	self.is_prod_version = DclGlobal.is_production()

	self.attach_log_sampled = randf() < ATTACH_LOG_SAMPLE_RATE

	SentrySDK.init(
		func(options: SentryOptions) -> void:
			options.release = release_string
			options.before_send = _before_send
			options.attach_log = self.attach_log_sampled
			# Disambiguate builds that share a release string (cross-platform
			# CI runs, debug vs release of the same commit).
			options.dist = "%s-%s" % [OS.get_name(), DclGlobal.get_commit_hash()]
			options.send_default_pii = false

			# Set environment based on build type
			# production: report to production env
			# staging: report to staging env
			# dev: no sentry report (handled in _before_send)
			if self.is_prod_version:
				options.environment = "production"
			elif self.is_staging_version:
				options.environment = "staging"
			else:
				options.environment = "development"
				options.debug = true

			# Keep the SDK-level rate at 1.0 — the effective rate is the remote
			# `sentry-sample-rate` feature flag, enforced in _before_send once
			# the flags load.
			options.sample_rate = 1.0

			# Clamp the logger's own limiter. The addon defaults (20 events per
			# 10s, same source line re-reported every 1s) let a single device
			# emit ~172k events/day when an engine ERR_FAIL_COND fires inside a
			# per-frame loop, which is what drained the quota. These values cap
			# a device at ~5 events/min (~7k/day) and re-report a given source
			# line at most once a minute; _before_send still filters on top.
			options.logger_limits.events_per_frame = 2
			options.logger_limits.repeated_error_window_ms = 60000
			options.logger_limits.throttle_events = 5
			options.logger_limits.throttle_window_ms = 60000
	)

	# Tag every event so we can filter sampled-vs-unsampled in the Sentry UI.
	SentrySDK.set_tag("attach_log_sampled", str(self.attach_log_sampled))

	# Static tags that never change after process start.
	SentrySDK.set_tag("platform", OS.get_name())
	SentrySDK.set_tag("build_type", "debug" if OS.is_debug_build() else "release")
	SentrySDK.set_tag("gpu_vendor", RenderingServer.get_video_adapter_vendor())
	SentrySDK.set_tag("locale", OS.get_locale())

	var graphics_ctx := {
		"name": RenderingServer.get_video_adapter_name(),
		"vendor": RenderingServer.get_video_adapter_vendor(),
		"version": RenderingServer.get_video_adapter_api_version(),
	}
	SentrySDK.set_context("graphics", graphics_ctx)

	# Add Sentry tags for staging and development builds (for filtering)
	if self.is_staging_version or self.is_dev_version:
		var branch_name = DclGlobal.get_branch_name()
		var commit_message = DclGlobal.get_commit_message()

		if not branch_name.is_empty():
			SentrySDK.set_tag("branch_name", branch_name)

		if not commit_message.is_empty():
			SentrySDK.set_tag("commit_message", commit_message)

		# Only add commit hash for staging (not for development)
		if self.is_staging_version:
			var commit_hash = DclGlobal.get_commit_hash()
			if not commit_hash.is_empty():
				SentrySDK.set_tag("commit_hash", commit_hash)


## Called by FeatureFlags when the remote flags load.
func set_sentry_sample_rate(rate: float) -> void:
	sentry_sample_rate = clampf(rate, 0.0, 1.0)


func _before_send(event: SentryEvent) -> SentryEvent:
	# Discard events for dev builds - only prod and staging report to Sentry
	if self.is_dev_version:
		return null

	# Remote throttle (`sentry-sample-rate` feature flag): 1.0 keeps every
	# event, 0.0 drops them all.
	if randf() >= sentry_sample_rate:
		return null

	if randf() >= NOISE_KEEP_RATE:
		var msg := _event_text(event)
		if not msg.is_empty():
			for pattern in NOISE_PATTERNS:
				if pattern in msg:
					return null

	# if event.message.contains("Bruno"):
	#	# Scrub sensitive information from the event.
	#	event.message = event.message.replace("Bruno", "REDACTED")

	return event


# SentryGodotLogger builds engine/Rust errors with `add_exception()` and never
# calls `set_message()`, so `event.message` is empty for every event the noise
# filter is meant to catch. Read the exception value first and fall back to
# `message` for events captured directly via SentrySDK.capture_message().
func _event_text(event: SentryEvent) -> String:
	if event.get_exception_count() > 0:
		var value := event.get_exception_value(0)
		if not value.is_empty():
			return value
	return event.message
