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
	'Condition "active"',
]
# Keep this fraction of noise events as a canary — if the shape or volume of
# engine/driver errors shifts we want to notice, but 100% is wasted quota.
const NOISE_KEEP_RATE := 0.05

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
			options.attach_log = self.attach_log_sampled

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

	if randf() >= NOISE_KEEP_RATE:
		var msg: String = event.message
		if not msg.is_empty():
			for pattern in NOISE_PATTERNS:
				if pattern in msg:
					return null

	# if event.message.contains("Bruno"):
	#	# Scrub sensitive information from the event.
	#	event.message = event.message.replace("Bruno", "REDACTED")

	return event
