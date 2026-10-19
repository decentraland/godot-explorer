extends TextureProgressBar

@export var speed_scale = 1.0
@export var show_background = false:
	set(value):
		show_background = value
		tint_under = Color.WHITE if value else Color.TRANSPARENT

@onready var animation_player = $AnimationPlayer


func _ready():
	animation_player.speed_scale = speed_scale
	tint_under = Color.WHITE if show_background else Color.TRANSPARENT
	# Autoplay starts even under a hidden parent; spinners in closed popups must not tick.
	_on_visibility_changed()


func _on_visibility_changed():
	if animation_player == null:
		return

	if is_visible_in_tree():
		animation_player.play("spin")
	else:
		animation_player.pause()
