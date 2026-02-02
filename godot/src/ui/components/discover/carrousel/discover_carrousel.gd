extends Control

@export var generator: CarrouselGenerator = null
@export var with_search: bool = false

@export var title: String = "No title":
	set(new_value):
		%Label_Title.text = new_value
		title = new_value

var _last_search_text: String = ""

@onready var scroll_container = %ScrollContainer
@onready var item_container = %HBoxContainer_Items
@onready var label_error = $VBoxContainer/Label_Error
@onready var label_not_found = $VBoxContainer/Label_NotFound
@onready var h_box_container_loading = $VBoxContainer/HBoxContainer_Loading


func _ready():
	if is_instance_valid(generator):
		generator.report_loading_status.connect(self._on_report_loading_status)
		generator.item_container = item_container

		scroll_container.item_container = item_container
		scroll_container.request.connect(generator.on_request)
		scroll_container.start()


func _on_report_loading_status(status: CarrouselGenerator.LoadingStatus) -> void:
	if status == CarrouselGenerator.LoadingStatus.LOADING:
		h_box_container_loading.show()

		scroll_container.hide()
		label_not_found.hide()
		label_error.hide()
		show()
	elif status == CarrouselGenerator.LoadingStatus.OK_WITH_RESULTS:
		scroll_container.show()

		h_box_container_loading.hide()
		label_not_found.hide()
		label_error.hide()
		show()
	elif status == CarrouselGenerator.LoadingStatus.OK_WITHOUT_RESULTS:
		label_not_found.show()

		scroll_container.hide()
		h_box_container_loading.hide()
		label_error.hide()
		hide()
	else:
		#elif not ok:
		h_box_container_loading.hide()

		scroll_container.hide()
		label_not_found.hide()
		label_error.show()
		hide()


func set_search_param(new_search_param: String):
	generator.search_param = new_search_param
	scroll_container.restart()
