class_name RnUiFilter
extends RefCounted
## VIBE-CODED SPIKE: hides every Godot UI node except the ones that stay native.


# Hide, never free: explorer.gd keeps @onready refs to these nodes. Scene UI, the joystick,
# the on-screen gamepad, the camera-look catcher and the version/FPS labels stay native. UI root
# keeps focus or the player ignores movement input.
static func hide_non_native(ui_root: Control) -> void:
	for child in ui_root.get_children():
		if child is CanvasItem and child.visible and not _is_native(String(child.name)):
			child.hide()
	if not ui_root.has_focus():
		ui_root.grab_focus()


static func _is_native(node_name: String) -> bool:
	return (
		node_name == "SceneUIContainer"
		or node_name == "SafeAreaControls"
		or node_name == "VirtualJoystick_Left"
		or node_name == "MobileCameraInput"
		or node_name == "HBoxContainer_VersionFPS"
	)
