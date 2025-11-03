class_name ScreenTrackingHandler
extends Resource

## Base class for screen tracking handlers
## Allows tracking logic to be independent of the component
##
## Extend this script to create your own tracking implementation
## and pass it as an export var to EventDetailWrapper

## Function that will be called when a screen is displayed
## @param screen_name: String - Screen name (e.g., “EVENT_DETAILS”)
## @param item_data: Dictionary - Complete event/item data


func track_screen_viewed(_item_data: Dictionary):
	push_error("ScreenTrackingHandler.track_screen_viewed() must be overridden in a derived class.")
