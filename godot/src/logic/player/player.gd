extends CharacterBody3D

@onready var mount_camera := $Mount
@onready var camera: Camera3D = $Mount/Camera3D
@onready var direction: Vector3 = Vector3(0, 0, 0)
@onready var avatar := $Avatar

var first_person: bool = true
var _mouse_position = Vector2(0.0, 0.0)
var _touch_position = Vector2(0.0, 0.0)
var captured: bool = true

var is_on_air: bool

@export var vertical_sens: float = 0.5
@export var horizontal_sens: float = 0.5

var body_shape: String = "urn:decentraland:off-chain:base-avatars:BaseFemale"
var wearables: PackedStringArray = [
	"urn:decentraland:off-chain:base-avatars:f_sweater",
	"urn:decentraland:off-chain:base-avatars:f_jeans",
	"urn:decentraland:off-chain:base-avatars:bun_shoes",
	"urn:decentraland:off-chain:base-avatars:standard_hair",
	"urn:decentraland:off-chain:base-avatars:f_eyes_01",
	"urn:decentraland:off-chain:base-avatars:f_eyebrows_00",
	"urn:decentraland:off-chain:base-avatars:f_mouth_00"
]
var eyes_color: Color = Color(0.3, 0.2235294133424759, 0.99)
var hair_color: Color = Color(0.5960784554481506, 0.37254902720451355, 0.21568627655506134)
var skin_color: Color = Color(0.4901960790157318, 0.364705890417099, 0.27843138575553894)
var emotes: Array = []


func _ready():
	camera.current = true
	if is_on_floor():
		is_on_air = false

	if captured:
		Input.set_mouse_mode(Input.MOUSE_MODE_CAPTURED)

	first_person = false
	var tween_out = create_tween()
	tween_out.tween_property(camera, "position", Vector3(0.5, 0, 4), 0.25).set_ease(
		Tween.EASE_IN_OUT
	)

	first_person = false
	avatar.show()

	floor_snap_length = 0.2

	avatar.update_avatar(
		"https://peer.decentraland.org/content",
		"Godot User",
		body_shape,
		eyes_color,
		hair_color,
		skin_color,
		wearables,
		emotes
	)


func _input(event):
	# Receives touchscreen motion
	if Global.is_mobile:
		if event is InputEventScreenDrag:
			_touch_position = event.relative
			rotate_y(deg_to_rad(-_touch_position.x) * horizontal_sens)
			avatar.rotate_y(deg_to_rad(_touch_position.x) * horizontal_sens)
			mount_camera.rotate_x(deg_to_rad(-_touch_position.y) * vertical_sens)
			if first_person:
				mount_camera.rotation.x = clamp(
					mount_camera.rotation.x, deg_to_rad(-60), deg_to_rad(90)
				)
			else:
				mount_camera.rotation.x = clamp(
					mount_camera.rotation.x, deg_to_rad(-70), deg_to_rad(45)
				)

	# Receives mouse motion
	if not Global.is_mobile && event:
		if event is InputEventMouseMotion && Input.get_mouse_mode() == Input.MOUSE_MODE_CAPTURED:
			_mouse_position = event.relative
			rotate_y(deg_to_rad(-_mouse_position.x) * horizontal_sens)
			avatar.rotate_y(deg_to_rad(_mouse_position.x) * horizontal_sens)
			mount_camera.rotate_x(deg_to_rad(-_mouse_position.y) * vertical_sens)

		if first_person:
			mount_camera.rotation.x = clamp(
				mount_camera.rotation.x, deg_to_rad(-60), deg_to_rad(90)
			)
		else:
			mount_camera.rotation.x = clamp(
				mount_camera.rotation.x, deg_to_rad(-70), deg_to_rad(45)
			)

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
					avatar.show()
					avatar.set_rotation(Vector3(0, 0, 0))

			if event.button_index == MOUSE_BUTTON_WHEEL_UP:
				if first_person == false:
					first_person = true
					var tween_in = create_tween()
					tween_in.tween_property(camera, "position", Vector3(0, 0, -0.2), 0.25).set_ease(
						Tween.EASE_IN_OUT
					)
					avatar.hide()


const WALK_SPEED = 2.0
const RUN_SPEED = 6.0
const GRAVITY := 55.0
const JUMP_VELOCITY_0 := 12.0


func _physics_process(delta: float) -> void:
	var input_dir := Input.get_vector("ia_left", "ia_right", "ia_forward", "ia_backward")
	direction = (transform.basis * Vector3(input_dir.x, 0, input_dir.y)).normalized()
	var _floor: bool = is_on_floor()

	if _floor:
		if not _floor == is_on_air:
#			particles_jump.emitting = true
			is_on_air = _floor
	else:
		if not _floor == is_on_air:
#			particles_land.emitting = true
			is_on_air = _floor

	if not _floor:
#		particles_move.emitting = false
		if Input.is_action_pressed("double_gravity"):
			velocity.y -= GRAVITY * delta * .5
		else:
			velocity.y -= GRAVITY * delta

	elif Input.is_action_just_pressed("ia_jump"):
		velocity.y = JUMP_VELOCITY_0

	if direction:
#		if is_on_floor():
#			particles_move.emitting = true
#		else:
#			particles_move.emitting = false

		if Input.is_action_pressed("ia_walk"):
#			if animation_player.current_animation != "Walk":
#				animation_player.play("Walk")
			avatar.set_walking()
			velocity.x = direction.x * WALK_SPEED
			velocity.z = direction.z * WALK_SPEED
		else:
#			if animation_player.current_animation != "Run":
#				animation_player.play("Run")

			avatar.set_running()
			velocity.x = direction.x * RUN_SPEED
			velocity.z = direction.z * RUN_SPEED

		avatar.look_at(direction + position)

	else:
#		particles_move.emitting = false
#		if animation_player.current_animation != "Idle":
#			animation_player.play("Idle")
		avatar.set_idle()
		velocity.x = move_toward(velocity.x, 0, WALK_SPEED)
		velocity.z = move_toward(velocity.z, 0, WALK_SPEED)

	move_and_slide()
