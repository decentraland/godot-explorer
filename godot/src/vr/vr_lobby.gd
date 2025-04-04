extends Node3D

@onready var ui_origin: Node3D = %UIOrigin3D
@onready var game_ui: Node3D = %GameUI
@onready var xr_camera_3d = %XRCamera3D


# gdlint:ignore = async-function-name
func _ready():
	prints("Vr Lobby")
	var current_terms_and_conditions_version: int = Global.get_config().terms_and_conditions_version
	if current_terms_and_conditions_version != Global.TERMS_AND_CONDITIONS_VERSION:
		game_ui.set_scene(
			load("res://src/ui/components/terms_and_conditions/terms_and_conditions.tscn")
		)
		game_ui.scene_node.accepted.connect(self.set_lobby_ui)
	else:
		set_lobby_ui()

	var xr_interface = XRServer.find_interface("OpenXR")
	if xr_interface != null:
		xr_interface.pose_recentered.connect(self.pose_recentered)

	await get_tree().process_frame
	pose_recentered()


func set_lobby_ui():
	game_ui.set_scene(load("res://src/ui/components/auth/lobby.tscn"))
	game_ui.scene_node.change_scene.connect(self.change_scene)


func pose_recentered():
	ui_origin.rotation.y = xr_camera_3d.rotation.y


func change_scene(new_scene: String):
	game_ui.set_scene(load(new_scene))
