## Visibility-grid culling driver.
##
## Rebuilds the cell grid + PVS on every scene-set load (the same boundary
## the floating-islands feature uses — N parcels load/unload as a batch via
## `Global.scene_runner.loading_complete`). Per-frame update delegates to
## the Rust hot path (DclVisibilityGridRust) to keep CPU < 1ms/frame on GP.
##
## Off when `Global.cli.visibility_grid_enabled == false` (--no-visibility-grid).
class_name VisibilityCulling
extends Node

const GRID_SCRIPT := preload("res://src/tools/visibility_grid.gd")

var _grid: Node = null


func _ready() -> void:
	if not Global.cli.visibility_grid_enabled:
		set_process(false)
		return
	Global.scene_runner.loading_complete.connect(_on_loading_complete)


func _on_loading_complete(_session_id: int) -> void:
	if _grid != null:
		_grid = null
	var scene_root: Node = Global.scene_runner
	if scene_root == null:
		return
	_grid = GRID_SCRIPT.new()
	_grid.build_from_scene_tree(scene_root)


func _process(_delta: float) -> void:
	if _grid == null:
		return
	var camera: Camera3D = Global.player_camera_node
	if camera == null:
		return
	_grid.update_visibility(camera)
