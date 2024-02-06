extends Node

@export var avatar: Avatar = null

# Avoid multiple animations being executed
var animation_key_pressed = false


func _input(event):
	# Receives mouse motion
	if not Global.is_mobile() && event:
		# Release mouse
		if event is InputEventKey:
			# Play emotes
			if event.is_command_or_control_pressed():
				if event.keycode >= KEY_0 and event.keycode <= KEY_9:
					if event.pressed:
						if animation_key_pressed == false:
							var id := avatar.play_emote_by_index(event.keycode - KEY_0)
							avatar.broadcast_avatar_animation(id)
							animation_key_pressed = true
					else:
						animation_key_pressed = false
