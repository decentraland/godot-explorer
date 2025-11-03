class_name JumpInScreenTrackingHandler
extends ScreenTrackingHandler

func track_screen_viewed(item_data: Dictionary):
	var place_id = item_data.get("id", "unknown-id")
	var orientation = "portrait" if Global.is_orientation_portrait() else "landscape"
	
	var extra_properties = JSON.stringify(
		{
			"place_id": place_id,
			"orientation": orientation
		}
	)
	
	#Global.metrics.track_screen_viewed("JUMP_IN", extra_properties)
	print("JUMP_IN")
	print(extra_properties)
