extends TextureProgressBar

@export var speed_scale := 1.0
@export var show_background := false

@onready var animation_player = $AnimationPlayer


func _ready():
	animation_player.speed_scale = speed_scale
	%BackgroundCircle.visible = show_background


func _on_visibility_changed():
	if animation_player == null:
		return

	if self.visible:
		animation_player.play("spin")
	else:
		animation_player.pause()
