extends Control

var is_enabled = false

func _ready():
	hide()


func _physics_process(_delta):
	if Global.comms.is_voice_chat_enabled():
		if visible == false and Input.is_action_pressed("ia_record_mic"):
			show()
		elif visible and not Input.is_action_pressed("ia_record_mic"):
			hide()
	else:
		if visible:
			hide()
