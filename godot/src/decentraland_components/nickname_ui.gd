@tool
extends MarginContainer

const MAX_CHAR_COUNT = 80
const CHARACTER_WIDTH = 44
const MAX_WIDTH := 1200.0
const TWEEN_DURATION := .25
const MESSAGE_DURATION := 6.0

@export var send_message: String
@export_tool_button("Send message", "Callable")
var send_message_action = func(): async_show_message.bind(send_message).call()

@export var message_container_scene: PackedScene

@export var name_claimed := false:
	set(value):
		name_claimed = value
		if !is_inside_tree():
			return
		hash_container.visible = !value
		checkmark_container.visible = value

@export var mic_enabled := false:
	set(value):
		mic_enabled = value
		if !is_inside_tree():
			return
		mic_enabled_icon.visible = mic_enabled

@export var nickname := "nickname":
	set(value):
		nickname = value
		if !is_inside_tree():
			return
		nickname_label.text = nickname

@export var tag := "xxxx":
	set(value):
		tag = value
		if !is_inside_tree():
			return
		tag_label.text = tag

@export var nickname_color := Color(1, 1, 1):  # Default to white
	set(value):
		nickname_color = value
		if !is_inside_tree():
			return
		nickname_label.add_theme_color_override("font_color", nickname_color)

var message_tween: Tween

@onready var mic_enabled_icon = %MicEnabled
@onready var tag_label = %Tag
@onready var nickname_tag = %NicknameTag
@onready var nickname_label = %Nickname
@onready var hash_container = %Hash
@onready var checkmark_container = %ClaimedCheckmark
@onready var message_clip = %MessageClip


func create_message_container(message: String):
	var message_container = message_container_scene.instantiate()
	var message_label = message_container.get_child(0)
	message_label.text = message
	return message_container


func async_show_message(message: String):
	for child in message_clip.get_children():
		child.queue_free()
	if message == "":
		return
	if message_tween:
		message_tween.kill()

	var message_container = create_message_container(message)
	var message_label = message_container.get_child(0)

	message_clip.custom_minimum_size = Vector2.ZERO
	message_clip.add_child(message_container)

	var container_size = message_container.size
	var width: float = container_size.x
	if width >= MAX_WIDTH:
		width = MAX_WIDTH
		message_container.queue_free()
		message_container = create_message_container(message)
		message_label = message_container.get_child(0)
		message_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		message_label.custom_minimum_size.x = width
		message_clip.add_child(message_container)

	var final_size = Vector2(width, message_container.size.y)
	print(final_size)

	message_tween = create_tween()
	message_tween.tween_property(message_clip, "custom_minimum_size", final_size, TWEEN_DURATION)
	message_tween.tween_property(message_clip, "custom_minimum_size", final_size, MESSAGE_DURATION)
	var subtween = create_tween().set_parallel()
	subtween.tween_property(message_clip, "custom_minimum_size", Vector2.ZERO, TWEEN_DURATION)
	subtween.tween_property(message_clip, "size", Vector2.ZERO, TWEEN_DURATION)

	message_tween.tween_subtween(subtween)
	await message_tween.finished
	message_container.queue_free()
