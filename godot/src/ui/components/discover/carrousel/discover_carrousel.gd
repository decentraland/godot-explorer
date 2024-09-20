extends Control

@export var generator: CarrouselGenerator = null
@export var with_search: bool = false

@export var title: String = "No title":
	set(new_value):
		%Label_Title.text = new_value
		title = new_value

@onready var scroll_container = %ScrollContainer
@onready var item_container = %HBoxContainer_Items
@onready var label_error = $VBoxContainer/Label_Error
@onready var label_not_found = $VBoxContainer/Label_NotFound
@onready var h_box_container_loading = $VBoxContainer/HBoxContainer_Loading

@onready var timer_search_bounce = $MarginContainer/HBoxContainer/TimerSearchBounce
@onready var line_edit_search = $MarginContainer/HBoxContainer/LineEdit_Search
@onready var button_search_hide = $MarginContainer/HBoxContainer/Button_SearchHide
@onready var button_search = $MarginContainer/HBoxContainer/Button_Search

var _last_search_text: String = ""

func _ready():
	if is_instance_valid(generator):
		generator.report_loading_status.connect(self._on_report_loading_status)
		generator.item_container = item_container

		scroll_container.item_container = item_container
		scroll_container.request.connect(generator.on_request)
		scroll_container.start()

	button_search.visible = with_search
	
func _on_report_loading_status(status: CarrouselGenerator.LoadingStatus) -> void: 
	if status == CarrouselGenerator.LoadingStatus.Loading:
		h_box_container_loading.show()
		
		scroll_container.hide()
		label_not_found.hide()
		label_error.hide()
	elif status == CarrouselGenerator.LoadingStatus.OkWithResults:
		scroll_container.show()
		
		h_box_container_loading.hide()
		label_not_found.hide()
		label_error.hide()
	elif status == CarrouselGenerator.LoadingStatus.OkWithoutResults:
		label_not_found.show()
		
		scroll_container.hide()
		h_box_container_loading.hide()
		label_error.hide()
	else:
	#elif not ok:
		h_box_container_loading.hide()
		
		scroll_container.hide()
		label_not_found.hide()
		label_error.show()
		
func _on_scroll_container_scroll_ended():
	pass  # Replace with function body.

func _on_timer_search_bounce_timeout():
	if line_edit_search.text != _last_search_text:
		_last_search_text = line_edit_search.text
		generator.search_param = line_edit_search.text
		scroll_container.restart()
		return
	
func _on_line_edit_search_text_changed(_new_text):
	timer_search_bounce.start()

func _on_button_search_pressed():
	if not button_search_hide.visible:
		button_search_hide.show()
		line_edit_search.show()
		line_edit_search.text = ""
		line_edit_search.call_deferred("grab_focus")

func _on_button_search_hide_pressed():
	button_search_hide.hide()
	line_edit_search.hide()
	line_edit_search.text = ""
