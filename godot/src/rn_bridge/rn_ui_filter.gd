class_name RnUiFilter
extends RefCounted
## Hides every Godot UI node except the ones that stay native.

const EMOTE_WHEEL := "EmoteWheel"


# Hide, never free: explorer.gd keeps @onready refs to these nodes. Scene UI, the joystick,
# the on-screen gamepad, the camera-look catcher, the crosshair, the pointer tooltip and the version/FPS labels stay native. UI root
# keeps focus or the player ignores movement input.
static func hide_non_native(ui_root: Control) -> void:
	for child in ui_root.get_children():
		if child.name == "SafeAreaHud":
			_hide_hud_except_emotes(child)
		elif child is CanvasItem and child.visible and not _is_native(String(child.name)):
			child.hide()
	if not ui_root.has_focus():
		ui_root.grab_focus()


# The emote button and wheel stay native: hide the rest of the interactive HUD and Show-UI button.
static func _hide_hud_except_emotes(safe_area: Node) -> void:
	for group in safe_area.get_children():
		if not group is CanvasItem:
			continue
		if group.name != "InteractableHUD":
			group.hide()
			continue
		for widget in group.get_children():
			if widget is CanvasItem and widget.visible and widget.name != EMOTE_WHEEL:
				widget.hide()


static func _is_native(node_name: String) -> bool:
	return (
		node_name == "SceneUIContainer"
		or node_name == "SafeAreaControls"
		or node_name == "VirtualJoystick_Left"
		or node_name == "MobileCameraInput"
		or node_name == "HBoxContainer_VersionFPS"
		or node_name == "Label_Crosshair"
		or node_name == "Control_PointerTooltip"
	)
