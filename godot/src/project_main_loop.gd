class_name ProjectMainLoop
extends SceneTree

# Environment detection based on version string suffix
var is_dev_version = false
var is_staging_version = false
var is_prod_version = false


func _initialize() -> void:
	var release_string = "org.decentraland.godotexplorer@" + DclGlobal.get_version()

	# Detect environment from version string
	self.is_dev_version = DclGlobal.is_dev()
	self.is_staging_version = DclGlobal.is_staging()
	self.is_prod_version = DclGlobal.is_production()

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


func _before_send(event: SentryEvent) -> SentryEvent:
	# Discard events for dev builds - only prod and staging report to Sentry
	if self.is_dev_version:
		return null

	# if event.message.contains("Bruno"):
	#	# Scrub sensitive information from the event.
	#	event.message = event.message.replace("Bruno", "REDACTED")

	return event
