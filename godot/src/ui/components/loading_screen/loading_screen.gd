extends VBoxContainer

var carrousel_item = preload("res://src/ui/components/carrousel_page_item.tscn")
@onready var h_box_container_pagination = $ColorRect_Background/Control_Discover/VBoxContainer/HBoxContainer_Pagination
@onready var label_title = $ColorRect_Background/Control_Discover/VBoxContainer/HBoxContainer_Content/VBox_Data/Label_Title
@onready var rich_text_label_paragraph = $ColorRect_Background/Control_Discover/VBoxContainer/HBoxContainer_Content/VBox_Data/RichTextLabel_Paragraph
@onready var texture_rect_image = $ColorRect_Background/Control_Discover/VBoxContainer/HBoxContainer_Content/Control_Image/TextureRect_Image

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

func _on_texture_rect_right_arrow_gui_input(event):
	if event is InputEventMouseButton:
		if event.button_index == MOUSE_BUTTON_LEFT:
			if event.pressed:
				if (selected_data < len(mockedData)-1):
					selected_data = selected_data + 1
				else:
					selected_data = 0
				update_view()

func _on_texture_rect_left_arrow_gui_input(event):
	if event is InputEventMouseButton:
		if event.button_index == MOUSE_BUTTON_LEFT:
			if event.pressed:
				if (selected_data > 0):
					selected_data = selected_data - 1
				else:
					selected_data = len(mockedData)-1
				update_view()
				
	
func update_view():	
	for child in h_box_container_pagination.get_children():
		child.unselect()
	h_box_container_pagination.get_child(selected_data).select()
	label_title.text = mockedData[selected_data].title
	rich_text_label_paragraph.text = mockedData[selected_data].paragraph
	
	
	#This is expensive, but idk how iterate the array and create variables to save preload resources yet. 
	var route:String = mockedData[selected_data].imageSource
	var texture = load(route)
	texture_rect_image.texture = texture
