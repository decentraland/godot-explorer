extends Area3D

## Detects when the player teleports inside a collider and temporarily disables
## the collider's physics layer until the player exits.

## Tracks disabled colliders: Node -> original_collision_layer
var _disabled_colliders: Dictionary = {}


func _ready():
	collision_layer = 0  # Don't need to be detected
	collision_mask = 0xFFFFFFFF  # Detect all layers so we can track colliders after disabling CL_PHYSICS
	monitoring = true
	monitorable = false
	if not body_exited.is_connected(_on_body_exited):
		body_exited.connect(_on_body_exited)


## Called after teleport to detect overlapping colliders
func check_stuck():
	var space_state = get_world_3d().direct_space_state
	if not space_state:
		return

	# Get the collision shape from child node
	var collision_shape = get_node("CollisionShape3D_Body")

	var query = PhysicsShapeQueryParameters3D.new()
	query.shape = collision_shape.shape
	query.transform = global_transform * collision_shape.transform
	query.collision_mask = 2  # CL_PHYSICS
	query.collide_with_bodies = true
	query.collide_with_areas = false

	var results = space_state.intersect_shape(query, 32)

	for result in results:
		var collider = result.get("collider")
		if collider:
			_disable_collider(collider)


func _disable_collider(collider: Node):
	if collider in _disabled_colliders:
		return

	_disabled_colliders[collider] = collider.collision_layer
	# Disable CL_PHYSICS bit (layer 2, bit index 1)
	collider.collision_layer = collider.collision_layer & ~2

	if not collider.tree_exiting.is_connected(_on_collider_removed):
		collider.tree_exiting.connect(_on_collider_removed.bind(collider))


func _enable_collider(collider: Node):
	if collider not in _disabled_colliders:
		return

	# Restore original collision layer
	collider.collision_layer = _disabled_colliders[collider]
	_disabled_colliders.erase(collider)


func _on_body_exited(body: Node3D):
	if body in _disabled_colliders:
		_enable_collider(body)


func _on_collider_removed(collider: Node):
	_disabled_colliders.erase(collider)
