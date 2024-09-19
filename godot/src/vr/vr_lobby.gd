extends Node3D

@onready var ui_origin: Node3D = %UIOrigin3D
@onready var lobby_ui: Node3D = %LobbyUI
@onready var xr_camera_3d = %XRCamera3D


# gdlint:ignore = async-function-name
func _ready():
	prints("Vr Lobby")
	lobby_ui.scene_node.change_scene.connect(self.change_scene)

	var xr_interface = XRServer.find_interface("OpenXR")
	if xr_interface != null:
		xr_interface.pose_recentered.connect(self.pose_recentered)

	await get_tree().process_frame
	pose_recentered()


func pose_recentered():
	ui_origin.rotation.y = xr_camera_3d.rotation.y


func change_scene(new_scene: String):
	lobby_ui.set_scene(load(new_scene))
