@tool
extends HBoxContainer

var carrousel_item = preload("res://src/ui/components/carrousel_page_item.tscn")
# Called when the node enters the scene tree for the first time.
func _ready():
	pass

func populate(quantity:int):
	for i in range(quantity):
		var instantiated_carrousel_item = carrousel_item.instantiate()
		add_child(instantiated_carrousel_item)

func select(item_number):	
	for child in get_children():
		child.unselect()
	get_child(item_number).select()
