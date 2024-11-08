extends Node

var dirty_ids: Array[int] = []
var requests: Dictionary = {}
var request_selected_id: int = -1

@onready var requests_v_box_container = $Requests/ScrollContainer/VBoxContainer

@onready var request_body_text_edit = $Control/TabContainer/RequestBody/TextEdit
@onready var request_headers_text_edit = $Control/TabContainer/RequestHeaders/TextEdit

@onready var response_body_text_edit = $Control/TabContainer/ResponseBody/TextEdit
@onready var response_headers_text_edit = $Control/TabContainer/ResponseHeaders/TextEdit

@onready var label_url = $Control/TabContainer/General/Label_Url
@onready var label_method = $Control/TabContainer/General/Label_Method
@onready var label_ok = $Control/TabContainer/General/Label_OK
@onready var label_status = $Control/TabContainer/General/Label_Status


func _ready():
	Global.network_inspector.request_changed.connect(self.on_request_changed)

	for i in range(1, Global.network_inspector.get_request_count()):
		dirty_ids.push_back(i)


func on_request_changed(id: int):
	if not dirty_ids.has(id):
		dirty_ids.push_back(id)


func _on_timer_timeout():
	if dirty_ids.is_empty():
		return

	_update()


func _update():
	for id in dirty_ids:
		var request_control: NetworkInspectorRequestEntryControl = requests.get(id, null)
		if request_control == null:
			request_control = (
				load("res://src/ui/components/debug_panel/network_inspector/request_entry.tscn")
				. instantiate()
			)
			request_control.click.connect(self.on_request_click.bind(id))

			requests_v_box_container.add_child(request_control)

			requests[id] = request_control

		var request_data := Global.network_inspector.get_request(id)

		if is_instance_valid(request_data):
			request_control.method = request_data.method
			request_control.domain = request_data.url
			request_control.start_time = str(roundf(request_data.requested_at / 10.0) / 100.0)
			request_control.initiator = str(request_data.requested_by)

			if request_data.response_received:
				if request_data.response_error.is_empty():
					request_control.status = str(request_data.response_status_code)
					request_control.duration = str(
						roundf(
							(
								(
									request_data.response_payload_received_at
									- request_data.requested_at / 10.0
								)
								/ 100.0
							)
						)
					)
				else:
					request_control.status = "ERROR"
			else:
				request_control.status = "PEND"

	dirty_ids.clear()


func on_request_click(id: int):
	if requests.get(request_selected_id) != null:
		requests.get(request_selected_id).selected = false

	request_selected_id = id
	requests.get(request_selected_id).selected = true

	var request_data := Global.network_inspector.get_request(id)

	if request_data.response_received:
		if request_data.response_error.is_empty():
			response_headers_text_edit.text = JSON.stringify(request_data.response_headers, "  ")
			response_body_text_edit.text = request_data.get_request_body().substr(0, 4096)
			label_ok.text = "yes"
			label_status.text = str(request_data.response_status_code)
		else:
			label_ok.text = request_data.response_error
			label_status.text = "ERROR"
	else:
		label_status.text = "PEND"

	request_headers_text_edit.text = JSON.stringify(request_data.request_headers, "  ")
	request_body_text_edit.text = request_data.get_request_body().substr(0, 4096)

	label_url.text = request_data.url
	label_method.text = request_data.method
