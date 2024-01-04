extends TextureProgressBar

@onready var animation_player = $AnimationPlayer

func _on_visibility_changed():
	if animation_player == null:
		return
		
		
	if self.visible:
		animation_player.play("spin")
	else:
		animation_player.pause()
