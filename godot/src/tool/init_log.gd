class_name InitLog
extends RefCounted

## Per-component "[INIT] <name>" boot trace, emitted as each autoload / subsystem
## comes up so the wiring can be verified through the unified debug channel.
##
## Static (no instance) so it can be called before the `Global` autoload is ready
## — `UiSounds` initializes before `Global` — and so the whole set toggles from a
## single flag.
##
## TODO: set ENABLED = false once the boot-wiring verification is no longer needed.
## These are info-level and currently print on every startup, production included.
const ENABLED := true


static func emit(component: String) -> void:
	if ENABLED:
		print("[INIT] ", component)
