class_name EventsDetailScreenTrackingHandler
extends ScreenTrackingHandler


func track_screen_viewed(item_data: Dictionary):
	var event_id = item_data.get("id", "unknown-id")
	var event_status = "live" if item_data.get("live", false) else "upcoming"
	var event_tags = "trending" if item_data.get("trending", false) else "none"
	var orientation = "portrait" if Global.is_orientation_portrait() else "landscape"

	var extra_properties = JSON.stringify(
		{
			"event_id": event_id,
			"event_status": event_status,
			"event_tags": event_tags,
			"orientation": orientation
		}
	)

	Global.metrics.track_screen_viewed("EVENT_DETAILS", extra_properties)
