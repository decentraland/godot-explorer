class_name PlaceDetailsScreenTrackingHandler
extends ScreenTrackingHandler

func track_screen_viewed(item_data: Dictionary):
	var place_id = item_data.get("id", "unknown-id")
	var place_title = item_data.get("title", "unknown-title")
	var orientation = "portrait" if Global.is_orientation_portrait() else "landscape"
	
	var extra_properties = JSON.stringify(
		{
			"place_id": place_id,
			"place_title": place_title,
			"orientation": orientation
		}
	)
	
	Global.metrics.track_screen_viewed("PLACE_DETAILS", extra_properties)
