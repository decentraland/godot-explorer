@tool

class_name NetworkInspectorRequestEntryControl
extends Control

signal click

@export var selected: bool = false:
	set(value):
		selected = value
		await async_wait_ready()
		refresh_color()

@export var status: String = "":
	set(value):
		status = value
		await async_wait_ready()
		$HBoxContainer/Status.text = value

@export var method: String = "":
	set(value):
		method = value
		await async_wait_ready()
		$HBoxContainer/Method.text = value

@export var domain: String = "":
	set(value):
		domain = value
		await async_wait_ready()
		$HBoxContainer/Domain.text = value.substr(0, 40)

@export var initiator: String = "":
	set(value):
		initiator = value
		await async_wait_ready()
		$HBoxContainer/Initiator.text = value.substr(0, 40)

@export var start_time: String = "":
	set(value):
		start_time = value
		await async_wait_ready()
		$HBoxContainer/StartTime.text = value

@export var duration: String = "":
	set(value):
		duration = value
		await async_wait_ready()
		$HBoxContainer/Duration.text = value

@export var bytes_size: String = "":
	set(value):
		bytes_size = value
		await async_wait_ready()
		$HBoxContainer/Size.text = value

@export var regular_color: Color = Color("#2E2E32"):  # #232327:
	set(value):
		regular_color = value
		await async_wait_ready()
		refresh_color()

@export var hover_color: Color = Color("#353B48"):
	set(value):
		hover_color = value
		await async_wait_ready()
		refresh_color()

@export var selected_color: Color = Color("#5d5d77"):
	set(value):
		selected_color = value
		await async_wait_ready()
		refresh_color()

var hovered: bool = false


func _ready():
	refresh_color()


func _on_mouse_exited():
	hovered = false
	refresh_color()


func _on_mouse_entered():
	hovered = true
	refresh_color()


func async_wait_ready():
	if not is_node_ready():
		await ready


func refresh_color():
	if selected:
		$ColorRect.color = selected_color
	elif hovered:
		$ColorRect.color = hover_color
	else:
		$ColorRect.color = regular_color


func _on_gui_input(event):
	if event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT:
		self.click.emit()
