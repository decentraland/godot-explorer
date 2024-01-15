extends VBoxContainer

@onready var pagination = $ColorRect_Background/Control_Discover/VBoxContainer/Pagination
@onready var animation_player = $AnimationPlayer
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
var swipe_threshold = 120


func _ready():
	if Global.is_mobile:
		Global.config.ui_scale = 2.5
	var pages_quantity:int = mockedData.size()
	pagination.populate(pages_quantity)
	update_view()

func _on_texture_rect_right_arrow_gui_input(event):
	if event is InputEventScreenTouch:
		if event.pressed:
			nextPage()

func _on_texture_rect_left_arrow_gui_input(event):
	if event is InputEventScreenTouch:
		if event.pressed:
			prevPage()

func update_view():	
	animation_player.play("fadeOutContent")
	pagination.select(selected_data)
	animation_player.play("fadeInContent")
	
	label_title.text = mockedData[selected_data].title
	rich_text_label_paragraph.text = mockedData[selected_data].paragraph
	var route:String = mockedData[selected_data].imageSource
	var texture = load(route)
	texture_rect_image.texture = texture

var start_swipe_position:Vector2
func _on_control_discover_gui_input(event):
	if event is InputEventScreenTouch:
		if event.pressed:
			start_swipe_position = event.position
			
	if event is InputEventScreenTouch:
		if !event.pressed:
			var end_swipe_position = event.position
			var swipe_distance = end_swipe_position - start_swipe_position
			if abs(swipe_distance.x) > swipe_threshold:
				if swipe_distance.x > 0:
					nextPage()
				else:
					prevPage()
		
func prevPage():
	if selected_data > 0:
		selected_data = selected_data - 1
	else:
		selected_data = len(mockedData) - 1
	update_view()
	
func nextPage():
	if selected_data < len(mockedData) - 1:
		selected_data = selected_data + 1
	else:
		selected_data = 0
	update_view()
