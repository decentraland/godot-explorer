extends Control

@export var player: Player = null

var avatar: Avatar = null

@onready var emote_wheel_container = %EmoteWheelContainer

func _ready():
	avatar = player.avatar
	for child in emote_wheel_container.get_children():
		if child is EmoteWheelItem:
			child.play_emote.connect(self._on_play_emote)

func _physics_process(delta):
	if Input.is_action_just_pressed("ia_open_emote_wheel"):
		show()

func _on_play_emote(emote_id):
	self.hide()
	if avatar:
		avatar.play_emote(emote_id)
		avatar.broadcast_avatar_animation(emote_id)
