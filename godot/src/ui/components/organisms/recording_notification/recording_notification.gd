extends Control

var is_enabled = false

@onready var animation_player: AnimationPlayer = $AnimationPlayer


func _ready():
	visibility_changed.connect(_on_visibility_changed)
	hide()
	_on_visibility_changed()


func _on_visibility_changed() -> void:
	if is_visible_in_tree():
		animation_player.play()
	else:
		animation_player.pause()


func _physics_process(_delta):
	if Global.comms.is_voice_chat_enabled():
		if visible == false and Input.is_action_pressed("ia_record_mic"):
			show()
		elif visible and not Input.is_action_pressed("ia_record_mic"):
			hide()
	else:
		if visible:
			hide()
