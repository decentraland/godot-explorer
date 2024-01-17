# This components resize the current control that is attached to match the display_safe_area
# It is useful in Mobile, when there is a part of the screen that it should not have UI there
extends Control


func _ready():
	if OS.has_feature("mobile"):  # Only mobile! Not use this in a "simulated mobile"
		var safe_area = DisplayServer.get_display_safe_area()
		set_size(safe_area.size)
		set_position(safe_area.position)
