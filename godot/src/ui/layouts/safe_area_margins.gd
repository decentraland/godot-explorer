@tool
class_name SafeAreaMargins
extends Resource

## Shared HUD inset rectangle. Create ONE .tres and assign it to the `margin_profile`
## of every SafeMarginContainer that frames the HUD, so all of them align to the same
## rectangle and the values are edited in a single place (no per-container duplication).
## When set, it overrides each container's `min_margin_*`.
##
## Values are orientation-aware: `left/top/right/bottom` are used in landscape, and the
## `portrait_*` set in portrait (so the HUD can collapse to smaller margins when only the
## chat is on screen). SafeMarginContainer re-applies on Global.orientation_changed.

@export var left: int = 0
@export var top: int = 0
@export var right: int = 0
@export var bottom: int = 0

@export_group("Portrait")
@export var portrait_left: int = 0
@export var portrait_top: int = 0
@export var portrait_right: int = 0
@export var portrait_bottom: int = 0
