extends Node3D

@export var skip_process: bool = false
@onready var animation_player = $AnimationPlayer
@onready var label_3d_name = $Label3D_Name

var last_position: Vector3 = Vector3.ZERO
var target_position: Vector3 = Vector3.ZERO
var t: float = 0.0
var target_distance: float = 0.0

var first_position = false


func set_target(target: Transform3D) -> void:
	if not first_position:
		first_position = true
		self.global_transform = target
		last_position = target.origin
		return

	target_distance = target_position.distance_to(target.origin)

	last_position = target_position
	target_position = target.origin

	self.global_rotation = target.basis.get_euler()
	self.global_position = last_position

	t = 0


func _process(delta):
	if skip_process:
		return

	if t < 2:
		t += 10 * delta
		if t < 1:
			if t > 1.0:
				t = 1.0

			self.global_position = last_position.lerp(target_position, t)
			if target_distance > 0:
				if target_distance > 0.6:
					set_running()
				else:
					set_walking()

		elif t > 1.5:
			self.set_idle()


func set_walking():
	if animation_player.current_animation != "Walk":
		animation_player.play("Walk")


func set_running():
	if animation_player.current_animation != "Run":
		animation_player.play("Run")


func set_idle():
	if animation_player.current_animation != "Idle":
		animation_player.play("Idle")


func update_avatar(
	_base_url: String,
	avatar_name: String,
	_body_shape: String,
	_eyes: Color,
	_hair: Color,
	_skin: Color,
	_wearables: PackedStringArray,
	_emotes: Array
):
	label_3d_name.text = avatar_name
