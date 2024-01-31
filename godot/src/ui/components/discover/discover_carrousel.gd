extends VBoxContainer
const DISCOVER_CARROUSEL_ITEM = preload("res://src/ui/components/discover/discover_carrousel_item.tscn")
@onready var h_box_container = $ScrollContainer/HBoxContainer

func _ready():
	# Only to test layout
	add_item()
	add_item()
	add_item()
	add_item()
	add_item()
	add_item()
	add_item()

func add_item():
	var new_item = DISCOVER_CARROUSEL_ITEM.instantiate()
	h_box_container.add_child(new_item)
