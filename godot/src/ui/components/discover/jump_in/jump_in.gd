extends Control

signal jump_in(position: Vector2i, realm: String)

@onready var panel_jump_in_portrait: JumpIn = %PanelJumpInPortrait
@onready var panel_jump_in_landscape: JumpIn = %PanelJumpInLandscape

func _ready():
	panel_jump_in_portrait.jump_in.connect(self._emit_jump_in)
	panel_jump_in_landscape.jump_in.connect(self._emit_jump_in)

func _emit_jump_in(position: Vector2i, realm: String):
	jump_in.emit(position, realm)

func set_data(item_data):
	panel_jump_in_landscape.set_data(item_data)
	panel_jump_in_portrait.set_data(item_data)


func _on_visibility_changed() -> void:
	if visible and panel_jump_in_portrait != null and panel_jump_in_landscape != null:
		if Global.is_orientation_portrait():
			panel_jump_in_portrait.show()
			panel_jump_in_landscape.hide()
			var _animation_target_y = panel_jump_in_portrait.position.y
			# Place the menu off-screen above (its height above the target position)
			panel_jump_in_portrait.position.y = panel_jump_in_portrait.position.y + panel_jump_in_portrait.size.y
			
			create_tween().tween_property(panel_jump_in_portrait, "position:y", _animation_target_y, 0.5).set_trans(Tween.TRANS_SINE).set_ease(Tween.EASE_OUT)
		else:
			panel_jump_in_portrait.hide()
			panel_jump_in_landscape.show()
			var _animation_target_x = panel_jump_in_landscape.position.x
			# Place the menu off-screen above (its height above the target position)
			panel_jump_in_landscape.position.x = panel_jump_in_landscape.position.x + panel_jump_in_landscape.size.x
			
			create_tween().tween_property(panel_jump_in_landscape, "position:x", _animation_target_x, 0.5).set_trans(Tween.TRANS_SINE).set_ease(Tween.EASE_OUT)


func _on_gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		if !event.pressed:
			self.hide()
			UiSounds.play_sound("mainmenu_widget_close")
