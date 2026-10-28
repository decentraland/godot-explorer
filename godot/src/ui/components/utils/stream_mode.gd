class_name StreamMode
extends RefCounted

## Settings > Dev Tools "Stream Mode" (staging builds only): hides the on-screen gamepad
## (joypad + movement joystick) and the version label for video recording, while the controls
## keep taking touches. `visibility_layer = 0` drops a subtree from rendering but leaves it
## `visible` for input, and none of the explorer's show/hide/modulate juggling touches it.


static func is_enabled(explorer: Explorer) -> bool:
	return explorer.joypad.visibility_layer == 0


static func set_enabled(explorer: Explorer, enabled: bool) -> void:
	var layer := 0 if enabled else 1
	for node: CanvasItem in [explorer.joypad, explorer.virtual_joystick, explorer.label_version]:
		node.visibility_layer = layer
