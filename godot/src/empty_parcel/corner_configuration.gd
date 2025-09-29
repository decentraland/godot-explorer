class_name CornerConfiguration
extends Resource

enum ParcelState {
	EMPTY,    # Another empty parcel
	NOTHING,  # Out of bounds
	LOADED    # Loaded scene parcel
}

# Stores the state of each adjacent parcel
var north: ParcelState = ParcelState.NOTHING
var south: ParcelState = ParcelState.NOTHING
var east: ParcelState = ParcelState.NOTHING
var west: ParcelState = ParcelState.NOTHING
var northwest: ParcelState = ParcelState.NOTHING
var northeast: ParcelState = ParcelState.NOTHING
var southwest: ParcelState = ParcelState.NOTHING
var southeast: ParcelState = ParcelState.NOTHING

func get_edges_with_cliffs() -> Array:
	var edges = []
	# Add edges that are out of bounds only (not loaded parcels)
	if north == ParcelState.NOTHING:
		edges.append("north")
	if south == ParcelState.NOTHING:
		edges.append("south")
	if east == ParcelState.NOTHING:
		edges.append("east")
	if west == ParcelState.NOTHING:
		edges.append("west")
	return edges

func has_any_out_of_bounds_neighbor() -> bool:
	return (
		north == ParcelState.NOTHING or south == ParcelState.NOTHING or
		east == ParcelState.NOTHING or west == ParcelState.NOTHING
	)
