class_name ProjectMainLoop
extends SceneTree

var is_dev_version = false


func _initialize() -> void:
	var release_string = "org.decentraland.godotexplorer@" + DclGlobal.get_version()
	self.is_dev_version = false  # release_string.contains("dev")
	SentrySDK.init(
		func(options: SentryOptions) -> void:
			if OS.is_debug_build() or self.is_dev_version:
				options.environment = "debug"
				options.debug = true

			options.release = release_string
			options.before_send = _before_send
			if self.is_dev_version:
				options.environment = "staging"
			else:
				options.environment = "production"

			# 1.0 all errors, 0.5 -> 50% errors.
			# for custom sampling rate, use _before_send
			options.sample_rate = 1.0
	)


func _before_send(event: SentryEvent) -> SentryEvent:
	# Discard event if running in a develop build.
	if self.is_dev_version:
		return null

	# if event.message.contains("Bruno"):
	#	# Scrub sensitive information from the event.
	#	event.message = event.message.replace("Bruno", "REDACTED")

	return event
