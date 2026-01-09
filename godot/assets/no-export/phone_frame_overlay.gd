extends CanvasLayer

var color_rect: ColorRect
var shader_material: ShaderMaterial


func _ready() -> void:
	layer = 100  # Render on top of everything
	follow_viewport_enabled = false

	# Create the ColorRect that covers the full screen
	color_rect = ColorRect.new()
	color_rect.name = "PhoneFrameRect"
	color_rect.set_anchors_preset(Control.PRESET_FULL_RECT)
	color_rect.mouse_filter = Control.MOUSE_FILTER_IGNORE

	# Load and apply the shader
	var shader = load("res://assets/no-export/phone_frame_overlay.gdshader")
	shader_material = ShaderMaterial.new()
	shader_material.shader = shader
	color_rect.material = shader_material

	add_child(color_rect)

	# Update shader params based on current state
	_update_shader_params()

	# Connect to window size changes to update orientation
	get_window().size_changed.connect(_on_size_changed)


func _on_size_changed() -> void:
	_update_shader_params()


func _update_shader_params() -> void:
	if shader_material == null:
		return

	var is_ios := Global.cli.emulate_ios
	var is_portrait := Global.is_orientation_portrait()

	shader_material.set_shader_parameter("is_ios", is_ios)
	shader_material.set_shader_parameter("is_portrait", is_portrait)
