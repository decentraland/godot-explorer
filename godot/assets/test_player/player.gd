extends CharacterBody3D

@onready var mount_camera := $Mount
@onready var camera: Camera3D = $Mount/Camera3D
@onready var animation_player: AnimationPlayer = $PlayerVisuals/AnimationPlayer
@onready var direction: Vector3 = Vector3(0, 0, 0)
@onready var visuals = $PlayerVisuals

var first_person: bool = true
var _mouse_position = Vector2(0.0, 0.0)

@export var vertical_sens: float = 0.5
@export var horizontal_sens: float = 0.5


func _ready():
	var tween_out = create_tween()
	tween_out.tween_property(camera, "position", Vector3(0.5, 0, 4), 0.25).set_ease(
		Tween.EASE_IN_OUT
	)

	first_person = false

	visuals.show()
	visuals.set_rotation(Vector3(0, 0, 0))

	floor_snap_length = 0.2

	# Fix the Idle animation


#	var idle_anim := animation_player.get_animation("Idle")
#	for i in range(idle_anim.get_track_count()):
#		var original_path: NodePath = idle_anim.track_get_path(i)
#		var bone_name: StringName = original_path.get_name(original_path.get_name_count() - 1)
#		idle_anim.track_set_path(i, NodePath("Armature/Skeleton3D:" + bone_name))


func _input(event):
	# Receives mouse motion
	if event is InputEventMouseMotion && Input.get_mouse_mode() == Input.MOUSE_MODE_CAPTURED:
		_mouse_position = event.relative
		rotate_y(deg_to_rad(-_mouse_position.x) * horizontal_sens)
		visuals.rotate_y(deg_to_rad(_mouse_position.x) * horizontal_sens)
		mount_camera.rotate_x(deg_to_rad(-_mouse_position.y) * vertical_sens)
		mount_camera.rotation.x = clamp(mount_camera.rotation.x, deg_to_rad(-60), deg_to_rad(5))

	# Release mouse
	if event is InputEventKey:
		if event.keycode == KEY_TAB:
			Input.set_mouse_mode(Input.MOUSE_MODE_VISIBLE)

	# Toggle first or third person camera
	if event is InputEventMouseButton:
		if event.button_index == MOUSE_BUTTON_WHEEL_DOWN:
			if first_person == true:
				first_person = false
				var tween_out = create_tween()
				tween_out.tween_property(camera, "position", Vector3(0.5, 0, 4), 0.25).set_ease(
					Tween.EASE_IN_OUT
				)
				visuals.show()
				visuals.set_rotation(Vector3(0, 0, 0))

		if event.button_index == MOUSE_BUTTON_WHEEL_UP:
			if first_person == false:
				first_person = true
				var tween_in = create_tween()
				tween_in.tween_property(camera, "position", Vector3(0, 0, -0.2), 0.25).set_ease(
					Tween.EASE_IN_OUT
				)
				visuals.hide()


const WALK_SPEED = 5.0
const RUN_SPEED = 12.0
const GRAVITY := 55.0
const JUMP_VELOCITY_0 := 12.0


func _physics_process(delta: float) -> void:
	var input_dir := Input.get_vector("ia_left", "ia_right", "ia_forward", "ia_backward")
	direction = (transform.basis * Vector3(input_dir.x, 0, input_dir.y)).normalized()

	if not is_on_floor():
		if Input.is_action_pressed("double_gravity"):
			velocity.y -= GRAVITY * delta * .5
		else:
			velocity.y -= GRAVITY * delta
	elif Input.is_action_just_pressed("ia_jump"):
		velocity.y = JUMP_VELOCITY_0

	if direction:
		if Input.is_action_pressed("ia_walk"):
			if animation_player.current_animation != "Walk":
				animation_player.play("Walk")
			velocity.x = direction.x * WALK_SPEED
			velocity.z = direction.z * WALK_SPEED
		else:
			if animation_player.current_animation != "Run":
				animation_player.play("Run")

			velocity.x = direction.x * RUN_SPEED
			velocity.z = direction.z * RUN_SPEED

		visuals.look_at(direction + position)

	else:
		if animation_player.current_animation != "Idle":
			animation_player.play("Idle")
		velocity.x = move_toward(velocity.x, 0, WALK_SPEED)
		velocity.z = move_toward(velocity.z, 0, WALK_SPEED)

	move_and_slide()
