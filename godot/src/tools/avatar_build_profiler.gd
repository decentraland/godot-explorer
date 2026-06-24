class_name AvatarBuildProfiler
extends RefCounted

static var enabled: bool = false

static var _phases: Dictionary = {}
static var _last_usec: int = 0


static func begin() -> void:
	if not enabled:
		return
	_phases = {}
	_last_usec = Time.get_ticks_usec()


static func mark(phase: String) -> void:
	if not enabled:
		return
	var now := Time.get_ticks_usec()
	_phases[phase] = int(_phases.get(phase, 0)) + (now - _last_usec)
	_last_usec = now


static func finish() -> Dictionary:
	var result := _phases.duplicate()
	_phases = {}
	return result
