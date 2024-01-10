extends VBoxContainer

var carrousel_item = preload("res://src/ui/components/carrousel_page_item.tscn")
@onready var h_box_container_pagination = $ColorRect_Background/Control_Discover/VBoxContainer/HBoxContainer_Pagination

const mockedData = [{
	"title": "Genesis Plaza, heart of city",
	"paragraph": "Genesis Plaza is built and maintained by the Decentraland Foundation but is still in many ways a community project. Around here you'll find several teleports that can take you directly to special scenes marked as points of interest.",
	"imageSource": "res://assets/ui/NPCs_Aisha.png"
	},
	{
	"title": "Title 2",
	"paragraph": "Paragraph 2.",
	"imageSource": "res://assets/ui/NPCs_Robot.png"
	}
]

var selected_data = 0
	
func _ready():
	for data in mockedData:
		var instantiated_carrousel_item = carrousel_item.instantiate()
		h_box_container_pagination.add_child(instantiated_carrousel_item)
			
	update_view()
	
	
func update_view():	
	for child in h_box_container_pagination.get_children():
		child.unselect()
	h_box_container_pagination.get_child(selected_data).select()


func _on_texture_rect_gui_input(event):
	pass # Replace with function body.
