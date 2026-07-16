class_name GameLoopResult
extends RefCounted

## Typed outcome of a Game Loop scenario — replaces an untyped {passed, detail} Dictionary.
## `detail` is the human-readable line that flows into the machine-parseable RESULT log
## the runner prints (`[GAMELOOP] RESULT scenario=N status=... detail=...`).

var passed: bool
var detail: String


func _init(p_passed: bool = false, p_detail: String = "") -> void:
	passed = p_passed
	detail = p_detail


## Convenience constructor for a passing scenario.
static func ok(p_detail: String) -> GameLoopResult:
	return GameLoopResult.new(true, p_detail)


## Convenience constructor for a failing scenario.
static func fail(p_detail: String) -> GameLoopResult:
	return GameLoopResult.new(false, p_detail)
