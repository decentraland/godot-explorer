extends PanelContainer

@onready var label: Label = $Label
@onready var animation_player: AnimationPlayer = $AnimationPlayer


# Called when the node enters the scene tree for the first time.
func set_text(msg: String):
	size = Vector2(0,0)
	label.size = Vector2(0,0)
	label.text = msg

func show_at(screen_size: Vector2):
	await get_tree().process_frame
	var mouse_screen_pos = get_viewport().get_mouse_position()
	set_position_clamped(mouse_screen_pos + Vector2(16,16), screen_size)
	animation_player.play('show')
	

func set_position_clamped(pos: Vector2, screen_size: Vector2):
	var popup_size = size
	var clamped_pos = pos
	if clamped_pos.x + popup_size.x > screen_size.x:
		clamped_pos.x = screen_size.x - popup_size.x
	if clamped_pos.y + popup_size.y > screen_size.y:
		clamped_pos.y = screen_size.y - popup_size.y

	clamped_pos.x = max(clamped_pos.x, 0)
	clamped_pos.y = max(clamped_pos.y, 0)

	position = clamped_pos
