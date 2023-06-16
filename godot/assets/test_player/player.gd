extends CharacterBody3D

const WALK_SPEED = 5.0
const RUN_SPEED = 10.0
const JUMP_VELOCITY = 50.0

@onready var mount_camera := get_node("Mount")
@onready var camera := get_node("Mount/Camera3D")
@onready var animation_player = $Visuals/mixamo_base/AnimationPlayer

@onready var direction: Vector3 = Vector3(0,0,0)

@onready var visuals = $Visuals
@export var vertical_sens:float = 0.5
@export var horizontal_sens:float = 0.5

var first_person : bool = true
var _mouse_position = Vector2(0.0, 0.0)
var captured : bool = true

func _ready():
	first_person = false
	var tween_out = create_tween()
	tween_out.tween_property(camera, "position", Vector3(0.5,0,4), 0.25 ).set_ease(Tween.EASE_IN_OUT)
	visuals.show()
	visuals.set_rotation(Vector3(0,0,0))
	floor_snap_length = 0.2

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
				tween_out.tween_property(camera, "position", Vector3(0.5,0,4), 0.25 ).set_ease(Tween.EASE_IN_OUT)
				visuals.show()
				visuals.set_rotation(Vector3(0,0,0))
				
		if event.button_index == MOUSE_BUTTON_WHEEL_UP:
			if first_person == false:
				first_person = true
				var tween_in = create_tween()
				tween_in.tween_property(camera, "position", Vector3(0,0,-0.2), 0.25 ).set_ease(Tween.EASE_IN_OUT)
				visuals.hide()
		

var _tmp_floor: bool
var _floor = null
func _physics_process(delta: float) -> void:
	_tmp_floor = is_on_floor()
	
	
	# Add the gravity.
	if not _tmp_floor:
		velocity.y -= 100 * delta
	else:
		var s = str(get_slide_collision_count())
		
		for i in range(0, get_slide_collision_count()):
			s = s + " = " + str(get_slide_collision(i))
			
		print(s)
#		var slide = get_slide_collision(0)
#
#		var query := PhysicsRayQueryParameters3D.create(self.position, self.position + (10.0*Vector3.DOWN), 0xffffffff, [self])
#		var result := get_world_3d().direct_space_state.intersect_ray(query)
#		print(result)
		
	# Handle Jump.
	if Input.is_action_just_pressed("jump") and _tmp_floor:
		apply_floor_snap()
		velocity.y = JUMP_VELOCITY

	# Get the input direction and handle the movement/deceleration.
	# As good practice, you should replace UI actions with custom gameplay actions.
	var input_dir := Input.get_vector("move_left", "move_right", "move_up", "move_down")
	direction = (transform.basis * Vector3(input_dir.x, 0, input_dir.y)).normalized()
	if direction:
		if Input.is_action_pressed("walk"):
			if animation_player.current_animation != "walking":
				animation_player.play("walking")
			velocity.x = direction.x * WALK_SPEED
			velocity.z = direction.z * WALK_SPEED
		else:
			if animation_player.current_animation != "running":
				animation_player.play("running")
				
			velocity.x = direction.x * RUN_SPEED
			velocity.z = direction.z * RUN_SPEED
		visuals.look_at(direction + position)
	
	else:
		if animation_player.current_animation != "idle":
			animation_player.play("idle")
		velocity.x = move_toward(velocity.x, 0, WALK_SPEED)
		velocity.z = move_toward(velocity.z, 0, WALK_SPEED)
#
#	if is_on_wall() and is_on_floor() and velocity.x > 0:
#		position.y += 0.25
	
	move_and_slide()
	
	if is_on_floor():
		var floor := get_slide_collision(0)
		if floor != null:
			print(floor.get_position(0))
#	if is_on_floor():
#		print(" floor angle " , get_floor_angle(), " and normal ", get_floor_normal())
#		velocity += col_velocity
