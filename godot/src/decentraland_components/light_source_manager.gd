class_name DclLightSourceManager
extends Node

## Central ticker for scene-authored LightSource components.
##
## Replaces per-light _process: one iteration per tick instead of N nodes
## running GDScript every frame. Recomputes light range/budget state when:
##   - the player reference position changed (throttled to UPDATE_INTERVAL), or
##   - any light requested a recompute (created/removed/settings/transform).

const UPDATE_INTERVAL: float = 0.25

var _update_timer: float = 0.0


func _process(delta: float) -> void:
	# Fades advance every frame (only lights in transition do work).
	DclLightSourceComponent.tick_fades(delta)

	_update_timer -= delta

	var due: bool = _update_timer <= 0.0
	if not due and not DclLightSourceComponent.is_recompute_pending():
		return

	if due:
		_update_timer = UPDATE_INTERVAL

	var reference_position := Vector3.ZERO
	var has_reference := false

	var explorer := Global.get_explorer()
	if explorer != null and is_instance_valid(explorer.player):
		reference_position = explorer.player.global_position
		has_reference = true

	var cam_pos := Vector3.ZERO
	var cam_forward := Vector3.FORWARD
	var cam_available := false

	if is_instance_valid(Global.player_camera_node):
		cam_pos = Global.player_camera_node.global_position
		cam_forward = -Global.player_camera_node.global_transform.basis.z.normalized()
		cam_available = true

	DclLightSourceComponent.consume_recompute_pending()
	DclLightSourceComponent.tick(
		reference_position, has_reference, cam_pos, cam_forward, cam_available
	)
