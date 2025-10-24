extends Node

const DEFAULT_TRANSITION_TIME = 0.35  # in seconds

var global_virtual_camera_transform: Transform3D
var last_virtual_camera_entity_node = null
var last_camera_reached = true
var transition_start_transform: Transform3D
var transition_time_counter: float = 0.0

@onready var global_virtual_camera = Camera3D.new()


func _ready():
	add_child(global_virtual_camera)
	global_virtual_camera.clear_current()
	global_virtual_camera.cull_mask = 0x7fff


# in the process fn, this node has the responsability of transitioning the camera between player one and virtual one
#  virtuals can change between scenes or in the same scene, computed as desired_target
#  desired_target can be removed from the scene tree, so global_virtual_camera could be orphan at this point
func _process(delta: float) -> void:
	var current_scene_id = Global.scene_runner.get_current_parcel_scene_id()
	var scene_virtual_camera = Global.scene_runner.get_scene_virtual_camera(current_scene_id)

	var desired_target = null
	var look_at_entity_node: Node3D = null
	var transition_speed: float = 0.0
	var transition_time: float = 0.0

	if is_instance_valid(scene_virtual_camera) and scene_virtual_camera.entity_id >= 0:
		desired_target = Global.scene_runner.get_scene_entity_node_or_null_3d(
			current_scene_id, scene_virtual_camera.entity_id
		)

		if scene_virtual_camera.look_at_entity_id >= 0:
			if scene_virtual_camera.look_at_entity_id == 1:
				look_at_entity_node = Global.player_camera_node.get_parent_node_3d().get_parent()  # todo: it should be the feet
			else:
				look_at_entity_node = Global.scene_runner.get_scene_entity_node_or_null_3d(
					current_scene_id, scene_virtual_camera.look_at_entity_id
				)

		transition_speed = scene_virtual_camera.transition_speed
		transition_time = scene_virtual_camera.transition_time
		Global.scene_runner.raycast_use_cursor_position = true
	else:
		transition_time = DEFAULT_TRANSITION_TIME

	# if desired_target == null -> it should go to Global.player_camera_node
	#  - immediately reparent to self conserving the last global transform annoted
	#  - once is next to Global.player_camera_node, set Global.player_camera_node.make_current()

	# when desired_target == some node -> it should travel to the node
	# - starting from get_viewport().get_camera_3d().position setting globally to global_virtual_camera and making global_virtual_camera.make_current()
	# - once is next to the node, reparent the global_virtual_camera

	# always use the scene_virtual_camera.transition_speed and time (fallbacking to default if == 0)
	# when goes to the player (desired_target == null) use the default time

	# when desired_target != null and look_at_entity_node != null, compute the rotation (looking at)

	if desired_target != last_virtual_camera_entity_node:
		last_camera_reached = false
		if desired_target == null:
			# Going back to player camera - reparent to self conserving global transform
			if global_virtual_camera.get_parent() != self:
				global_virtual_camera.reparent(self)
		else:
			# Switching to virtual camera - start from current viewport camera position
			var current_camera = get_viewport().get_camera_3d()
			if current_camera != global_virtual_camera and is_instance_valid(current_camera):
				global_virtual_camera.global_transform = current_camera.global_transform

			# Make the global virtual camera current
			global_virtual_camera.make_current()

			var explorer = Global.get_explorer()
			if is_instance_valid(explorer):
				explorer.player.avatar.show()

		# Reset transition counter and store start transform
		transition_time_counter = 0.0
		transition_start_transform = global_virtual_camera.global_transform
		last_virtual_camera_entity_node = desired_target
	# If desired = target, and transition_time_counter == 1.0, so we've already are attached
	elif last_camera_reached:
		if is_instance_valid(look_at_entity_node):
			global_virtual_camera.look_at(look_at_entity_node.global_position)
		return

	# Determine effective transition time (fallback to default if both are 0)
	var effective_transition_time: float
	if transition_speed == 0.0 and transition_time == 0.0:
		effective_transition_time = DEFAULT_TRANSITION_TIME
		transition_time_counter = effective_transition_time
	elif transition_time > 0.0:
		effective_transition_time = transition_time
	else:
		# Calculate time from speed and distance
		var target_transform: Transform3D
		if desired_target != null:
			target_transform = desired_target.global_transform
		else:
			target_transform = Global.player_camera_node.global_transform

		var distance = transition_start_transform.origin.distance_to(target_transform.origin)
		effective_transition_time = (
			distance / transition_speed if transition_speed > 0.0 else DEFAULT_TRANSITION_TIME
		)

	# Increment time counter
	transition_time_counter += delta

	# Calculate constant-speed interpolation factor
	var t = clamp(transition_time_counter / effective_transition_time, 0.0, 1.0)

	# Perform smooth transition
	if desired_target == null:
		# Transitioning to player camera
		var player_camera_transform = Global.player_camera_node.global_transform
		global_virtual_camera.global_transform = transition_start_transform.interpolate_with(
			player_camera_transform, t
		)

		# Check if transition is complete - once is next to Global.player_camera_node, set Global.player_camera_node.make_current()
		if t >= 1.0:
			Global.player_camera_node.make_current()
			global_virtual_camera.clear_current()
			last_camera_reached = true
			Global.scene_runner.raycast_use_cursor_position = false
			var explorer = Global.get_explorer()
			if is_instance_valid(explorer):
				explorer.reset_cursor_position()
				if explorer.player.camera.get_camera_mode() == Global.CameraMode.FIRST_PERSON:
					explorer.player.avatar.hide()
	else:
		# Transitioning to virtual camera target
		var target_transform = desired_target.global_transform

		global_virtual_camera.global_transform = transition_start_transform.interpolate_with(
			target_transform, t
		)

		# once is next to the node, reparent the global_virtual_camera
		if t >= 1.0:
			if global_virtual_camera.get_parent() != desired_target:
				global_virtual_camera.reparent(desired_target)
				global_virtual_camera.transform = Transform3D.IDENTITY
				last_camera_reached = true

		# when desired_target != null and look_at_entity_node != null, compute the rotation (looking at)
		if is_instance_valid(look_at_entity_node):
			global_virtual_camera.look_at(look_at_entity_node.global_position)
