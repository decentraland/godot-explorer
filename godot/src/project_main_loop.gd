class_name ProjectMainLoop
extends SceneTree

# godot.log is normally attached on every event. We disable that globally via
# project setting `sentry/options/attach_log=false` and re-add the log only
# for ~1% of sessions, sampled here. We can't toggle attach_log from the init
# callback in this SDK version (1.0.0): _get_global_attachments() runs before
# the callback, so the runtime override never takes effect. add_attachment()
# at scope level works and is honored for every event in the session.
const ATTACH_LOG_SAMPLE_RATE := 0.01

# Environment detection based on version string suffix
var is_dev_version = false
var is_staging_version = false
var is_prod_version = false

var attach_log_sampled := false


func _initialize() -> void:
	var release_string = "org.decentraland.godotexplorer@" + DclGlobal.get_version()

	# Detect environment from version string
	self.is_dev_version = DclGlobal.is_dev()
	self.is_staging_version = DclGlobal.is_staging()
	self.is_prod_version = DclGlobal.is_production()

	self.attach_log_sampled = randf() < ATTACH_LOG_SAMPLE_RATE

	SentrySDK.init(
		func(options: SentryOptions) -> void:
			options.release = release_string
			options.before_send = _before_send

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

			# 1.0 all errors, 0.5 -> 50% errors.
			# for custom sampling rate, use _before_send
			options.sample_rate = 1.0
	)

	# Tag every event so we can filter sampled-vs-unsampled in the Sentry UI.
	SentrySDK.set_tag("attach_log_sampled", str(self.attach_log_sampled))

	if self.attach_log_sampled:
		var log_path: String = ProjectSettings.get_setting(
			"debug/file_logging/log_path", "user://logs/godot.log"
		)
		if FileAccess.file_exists(log_path):
			SentrySDK.add_attachment(SentryAttachment.create_with_path(log_path))

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


func _before_send(event: SentryEvent) -> SentryEvent:
	# Discard events for dev builds - only prod and staging report to Sentry
	if self.is_dev_version:
		return null

	# if event.message.contains("Bruno"):
	#	# Scrub sensitive information from the event.
	#	event.message = event.message.replace("Bruno", "REDACTED")

	return event
