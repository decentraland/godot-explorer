class_name ProjectMainLoop
extends SceneTree

# Sample which sessions upload godot.log. Without this, every event re-uploads
# the entire log file (default attach_log=true) and tight error loops blow up
# the Sentry attachment quota.
const ATTACH_LOG_SAMPLE_RATE := 0.01

# Text prefixes stamped by the Rust tracing layer (lib/src/tools/godot_logger.rs):
# every event it forwards arrives as "[Rust:{target}] {msg} ({file}:{line})".
# Scene JS console.error is funnelled through op_error (lib/src/dcl/js/mod.rs)
# so it always carries the dcl::js target. These nest, so they must be tested
# most-specific first: scene, then our crate, then any dependency.
const RUST_SCENE_PREFIX := "[Rust:dclgodot::dcl::js]"
const RUST_APP_PREFIX := "[Rust:dclgodot"
const RUST_PREFIX := "[Rust:"

# Event source taxonomy, reported as the `source` tag so the Sentry UI can split
# engine vs. our Rust vs. dependency vs. scene content in one click.
const SOURCE_CRASH := "crash"
const SOURCE_CAPTURE := "capture"
const SOURCE_SCENE := "scene"
const SOURCE_RUST_APP := "rust_app"
const SOURCE_RUST_DEP := "rust_dep"
const SOURCE_ENGINE := "engine"
# Structured events we author from GDScript (sentry_seeder.gd), marked with an
# `event_kind` tag: one issue per crashed scene, and the previous run's Android
# exit reason. Bounded at the call site, so exempt from the remote rate below.
const SOURCE_SCENE_CRASH := "scene_crash"
const SOURCE_EXIT_REASON := "exit_reason"

# Fraction of each source kept once `sentry-error-events` is on, applied on top
# of the remote `sentry-sample-rate`. Sources we author and can act on stay at
# 1.0. The unbounded ones - scene content we do not write, third-party crates,
# and engine/driver errors we cannot fix - keep a canary slice so a shift in
# shape or volume is still visible without paying for the firehose.
const SOURCE_KEEP_RATE := {
	SOURCE_CRASH: 1.0,
	SOURCE_CAPTURE: 1.0,
	SOURCE_SCENE_CRASH: 1.0,
	SOURCE_EXIT_REASON: 1.0,
	SOURCE_RUST_APP: 1.0,
	SOURCE_RUST_DEP: 0.05,
	SOURCE_SCENE: 0.01,
	SOURCE_ENGINE: 0.01,
}

# Keep rate for a source not listed above - i.e. a bug in _classify.
const UNKNOWN_SOURCE_KEEP_RATE := 0.01

# Sources that skip the remote `sentry-sample-rate` roll (0.0 still drops them:
# the kill switch stays total). Crashes are a fraction of a percent of volume,
# and the two structured sources cap themselves - at most ten scene crashes per
# session and one event per exit reason per launch - so sampling them would
# only randomly hide the events these tags exist for.
const REMOTE_RATE_EXEMPT := [SOURCE_CRASH, SOURCE_SCENE_CRASH, SOURCE_EXIT_REASON]

# Sampling rate used until the `sentry-sample-rate` feature flag loads — also
# the effective rate when the fetch fails or the flag is absent. 1.0 because
# crash-only mode (below) is the volume control now: pre-flag traffic is just
# crashes and NodeGuard messages, and sampling those would randomly drop the
# events we care about most (early-session crashes). The bff flag remains the
# remote kill switch (0.0 drops everything).
const DEFAULT_SENTRY_SAMPLE_RATE := 1.0

# Environment detection based on version string suffix
var is_dev_version = false
var is_staging_version = false
var is_prod_version = false

var attach_log_sampled := false

# Remote error-sampling rate from the `sentry-sample-rate` feature flag — lets
# us throttle Sentry volume without shipping a build. Enforced in _before_send
# because the flag fetch resolves after SentrySDK.init already ran.
var sentry_sample_rate := DEFAULT_SENTRY_SAMPLE_RATE

# When false (default), only crashes reach Sentry as events: exception events
# below FATAL — the engine/Rust error firehose captured by SentryGodotLogger —
# are dropped in _before_send. The `sentry-error-events` feature flag turns
# ERROR reporting back on remotely (e.g. to debug a release for a day) without
# shipping a build. Errors keep flowing as breadcrumbs either way (the logger's
# breadcrumb mask is untouched), so crash events retain the recent-error trail.
var sentry_error_events_enabled := false


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
			# (sentry-godot 2.x moved these under godot_logger; set explicitly
			# so an SDK bump can never silently reset them to the defaults.)
			options.godot_logger.limits.events_per_frame = 2
			options.godot_logger.limits.repeated_error_window_ms = 60000
			options.godot_logger.limits.throttle_events = 5
			options.godot_logger.limits.throttle_window_ms = 60000

			# Android ANRs: the SDK default, pinned so it cannot flip. Below
			# Android 11 this is a 5s main-thread watchdog; from 11 on the OS
			# exit record is read on the next launch (ANR v2), so an ANR that
			# kills the app is still reported.
			options.android.enable_anr_detection = true
	)

	# Tag every event so we can filter sampled-vs-unsampled in the Sentry UI.
	SentrySDK.set_tag("attach_log_sampled", str(self.attach_log_sampled))

	# Static tags that never change after process start.
	SentrySDK.set_tag("platform", OS.get_name())
	SentrySDK.set_tag("build_type", "debug" if OS.is_debug_build() else "release")
	SentrySDK.set_tag("gpu_vendor", RenderingServer.get_video_adapter_vendor())
	SentrySDK.set_tag("locale", OS.get_locale())
	# Also inside `dist`, but a tag is what the issue view can filter/group by.
	var commit_hash := DclGlobal.get_commit_hash()
	if not commit_hash.is_empty():
		SentrySDK.set_tag("commit_hash", commit_hash)
	# Desktop only: on Android OS.get_memory_info() reports -1 for every field,
	# and SentrySeeder sets this same tag from the native plugin instead.
	var physical_bytes: int = OS.get_memory_info().get("physical", -1)
	if physical_bytes > 0:
		SentrySDK.set_tag("total_ram_mb", str(physical_bytes / 1048576))

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


## Called by FeatureFlags when the remote flags load.
func set_sentry_sample_rate(rate: float) -> void:
	sentry_sample_rate = clampf(rate, 0.0, 1.0)


## Called by FeatureFlags when the remote flags load.
func set_sentry_error_events_enabled(enabled: bool) -> void:
	sentry_error_events_enabled = enabled


func _before_send(event: SentryEvent) -> SentryEvent:
	# Discard events for dev builds - only prod and staging report to Sentry.
	if self.is_dev_version:
		return null

	# These two are tested first, and in this order, both deliberately. The addon
	# routes its on_crash hook through this same callback, and a crashpad crash
	# event carries no exception values - so is_crash() must be tested before the
	# exception count, or every native crash would classify as an explicit
	# capture. Neither needs the event text, so the classifier never runs here.
	if event.is_crash() or event.level == SentrySDK.LEVEL_FATAL:
		return _keep(event, SOURCE_CRASH)
	# Structured events built by SentrySeeder carry an `event_kind` tag. They have
	# no exception either, so this must run before the shape test below or they
	# would land in SOURCE_CAPTURE and roll the remote rate.
	var event_kind := event.get_tag("event_kind")
	if event_kind == "scene_crash":
		return _keep(event, SOURCE_SCENE_CRASH)
	if event_kind == "exit_reason":
		return _keep(event, SOURCE_EXIT_REASON)
	# An event with no exception came from an explicit SentrySDK.capture_* call:
	# deliberate instrumentation, today only NodeGuard (node_guard.gd), which caps
	# itself at one report per site per session. Note this is a shape test, not an
	# intent test - any future capture_message inherits the same bypass.
	if event.get_exception_count() == 0:
		return _keep(event, SOURCE_CAPTURE)

	# Everything below is the SentryGodotLogger firehose, off by default and
	# re-enabled remotely by `sentry-error-events`. Errors keep flowing as
	# breadcrumbs either way, so a later crash still carries the recent-error
	# trail. Gating here rather than after classification means the text work
	# never runs at all in the default configuration.
	if not sentry_error_events_enabled:
		return null

	return _keep(event, _classify(event))


## Rolls the remote rate and then the per-source rate, and tags the survivor.
## Every set_tag lives here so tagging cannot drift out of step with dropping.
##
## Note randf() runs on whichever thread raised the error - the addon's logger
## does not marshal to the main thread - so draws can correlate across threads.
## That is fine for sampling; do not build anything on them being independent.
func _keep(event: SentryEvent, source: String) -> SentryEvent:
	# The remote kill switch has to be total, crashes included.
	if sentry_sample_rate <= 0.0:
		return null
	# Crashes and the self-bounded structured sources bypass the remote rate:
	# sampling them randomly drops the events we most need (see REMOTE_RATE_EXEMPT).
	if source not in REMOTE_RATE_EXEMPT and randf() >= sentry_sample_rate:
		return null
	var keep_rate: float = SOURCE_KEEP_RATE.get(source, UNKNOWN_SOURCE_KEEP_RATE)
	if keep_rate < 1.0 and randf() >= keep_rate:
		return null
	event.set_tag("source", source)
	return event


## Buckets a logger event by its text. SentryGodotLogger leaves `message` empty
## and puts the error in exception 0, and SentryEvent (sentry-godot 2.x) still
## exposes no exception type, culprit or stack frames - so that one string is
## everything there is to work with.
##
## Anything without a Rust prefix is an engine print. That includes GDScript
## push_error: it expands to ERR_PRINT -> ERR_HANDLER_ERROR, indistinguishable
## from a C++ engine error without serializing the whole event. Splitting those
## out needs a marker at the call site, not a richer test here.
func _classify(event: SentryEvent) -> String:
	var text := _event_text(event)
	if text.begins_with(RUST_SCENE_PREFIX):
		return SOURCE_SCENE
	if text.begins_with(RUST_APP_PREFIX):
		return SOURCE_RUST_APP
	if text.begins_with(RUST_PREFIX):
		return SOURCE_RUST_DEP
	return SOURCE_ENGINE


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
