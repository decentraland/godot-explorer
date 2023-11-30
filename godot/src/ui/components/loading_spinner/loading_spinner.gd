extends TextureProgressBar

var tween: Tween = null


func _ready() -> void:
	tween = get_tree().create_tween().set_loops()
	tween.tween_property(self, "radial_initial_angle", 360.0, 1.5).as_relative()


func _on_visibility_changed():
	if tween == null:
		return

	if self.visible:
		tween.play()
	else:
		tween.pause()
