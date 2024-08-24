extends Node3D

@export var follow_speed : Curve

@onready var lobby_ui: Node3D = %LobbyUI
@onready var ui_origin: Node3D = %UIOrigin3D

@onready var xr_camera_3d: XRCamera3D = %XRCamera3D

# Called when the node enters the scene tree for the first time.
func _ready():
	prints("Vr Lobby")
	lobby_ui.scene_node.change_scene.connect(self.change_scene)


func change_scene(new_scene: String):
	lobby_ui.set_scene(load(new_scene))


func _process(delta):
	# Get the camera direction (horizontal only)
	var camera_dir := xr_camera_3d.global_transform.basis.z
	camera_dir.y = 0.0
	camera_dir = camera_dir.normalized()

	# Get the loading screen direction
	var loading_screen_dir := ui_origin.global_transform.basis.z

	# Get the angle
	var angle := loading_screen_dir.signed_angle_to(camera_dir, Vector3.UP)
	if angle == 0:
		return

	# Do rotation based on the curve
	ui_origin.global_transform.basis = ui_origin.global_transform.basis.rotated(
			Vector3.UP * sign(angle),
			follow_speed.sample_baked(abs(angle) / PI) * delta
	).orthonormalized()
