extends Control

@export var generator: CarrouselGenerator = null

@export var title: String = "No title":
	set(new_value):
		%Label_Title.text = new_value
		title = new_value

@onready var scroll_container = %ScrollContainer
@onready var item_container = %HBoxContainer_Items


func _ready():
	if is_instance_valid(generator):
		generator.set_consumer_visible.connect(self.set_visible)
		generator.item_container = item_container

		scroll_container.item_container = item_container
		scroll_container.request.connect(generator.on_request)
		scroll_container.start()


func _on_scroll_container_scroll_ended():
	pass  # Replace with function body.
