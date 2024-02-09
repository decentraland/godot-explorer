extends Control


func _ready():
	hide()


func _physics_process(_delta):
	if visible == false and Input.is_action_pressed("ia_record_mic"):
		show()
	elif visible and not Input.is_action_pressed("ia_record_mic"):
		hide()
