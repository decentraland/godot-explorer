@tool
extends MarginContainer

const MAX_CHAR_COUNT = 80
const CHARACTER_WIDTH = 44
const MAX_WIDTH := 1200.0
const TWEEN_DURATION := .25
const MESSAGE_DURATION := 2.5

@export var name_claimed := false:
	set(value):
		name_claimed = value
		if !is_inside_tree(): return
		hash_container.visible = !value
		checkmark_container.visible = value

@export var mic_enabled := false:
	set(value):
		mic_enabled = value
		if !is_inside_tree(): return
		mic_enabled_icon.visible = mic_enabled

@export var nickname := "nickname":
	set(value):
		nickname = value
		if !is_inside_tree(): return
		nickname_label.text = nickname

@export var tag := "xxxx":
	set(value):
		tag = value
		if !is_inside_tree(): return
		tag_label.text = tag

@export var nickname_color := Color(1, 1, 1):  # Default to white
	set(value):
		nickname_color = value
		if !is_inside_tree(): return
		nickname_label.add_theme_color_override("font_color", nickname_color)

@onready var mic_enabled_icon = %MicEnabled
@onready var tag_label = %Tag
@onready var nickname_tag = %NicknameTag
@onready var nickname_label = %Nickname
@onready var hash_container = %Hash
@onready var checkmark_container = %ClaimedCheckmark
@onready var message_clip = %MessageClip
@onready var message_container = %MessageContainer
@onready var message_label = %MessageText

var message_tween : Tween

func show_message(message: String):
	if message_tween:
		message_tween.kill()
	message_tween = create_tween()
	message_label.custom_minimum_size = Vector2.ZERO
	message_label.size = Vector2.ZERO
	message_label.text = message if message.length() < MAX_CHAR_COUNT else '%s...' % message.substr(0, MAX_CHAR_COUNT-3)

	# We use estimate sizes to avoid waiting for resize
	var width : float = max(nickname_tag.size.x, message.length() * CHARACTER_WIDTH) 
	var height : float
	if width > MAX_WIDTH:
		width = MAX_WIDTH
		height = 250
		message_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		message_label.custom_minimum_size.x = width
		message_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_LEFT
	else:
		height = 75
		message_label.autowrap_mode = TextServer.AUTOWRAP_OFF
		message_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER

	message_clip.visible = true
	message_clip.custom_minimum_size = Vector2.ZERO
	message_tween.tween_property(message_clip,"custom_minimum_size",Vector2(width,height),TWEEN_DURATION)
	message_tween.tween_property(message_clip,"custom_minimum_size",Vector2(width,height),MESSAGE_DURATION)
	var subtween = create_tween().set_parallel()
	subtween.tween_property(message_clip,"custom_minimum_size",Vector2.ZERO,TWEEN_DURATION)
	subtween.tween_property(message_clip,"size",Vector2.ZERO,TWEEN_DURATION)
	message_tween.tween_subtween(subtween)
	await message_tween.finished
	message_clip.visible = false
