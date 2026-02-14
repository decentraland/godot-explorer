@tool
extends EditorPlugin

# Preset definitions: [label, width, height, is_ios, is_portrait]
const PRESETS := [
	["Default (720x720)", 720, 720, false, false],
	["iPhone 14 Pro — Portrait", 590, 1280, true, true],
	["iPhone 14 Pro — Landscape", 1561, 720, true, false],
	["Moto Edge 60 Pro — Portrait", 576, 1280, false, true],
	["Moto Edge 60 Pro — Landscape", 1600, 720, false, false],
]

const SETTINGS_KEY := "dcl_mobile_preview/last_selection"

var _option_button: OptionButton
var _overlay_viewport: SubViewport
var _overlay_rect: ColorRect
var _shader_material: ShaderMaterial


func _enter_tree() -> void:
	_option_button = OptionButton.new()
	_option_button.flat = true
	_option_button.tooltip_text = "Mobile Preview Preset"

	for i in PRESETS.size():
		_option_button.add_item(PRESETS[i][0], i)

	_option_button.item_selected.connect(_on_preset_selected)
	add_control_to_container(CONTAINER_TOOLBAR, _option_button)

	# Create the overlay SubViewport + ColorRect for the phone frame
	_setup_overlay()

	# Restore last selection
	var last := EditorInterface.get_editor_settings().get_setting(SETTINGS_KEY) if EditorInterface.get_editor_settings().has_setting(SETTINGS_KEY) else 0
	_option_button.select(last)
	_apply_preset(last)


func _exit_tree() -> void:
	# Restore defaults
	_apply_preset(0)

	if is_instance_valid(_option_button):
		remove_control_from_container(CONTAINER_TOOLBAR, _option_button)
		_option_button.queue_free()

	if is_instance_valid(_overlay_viewport):
		_overlay_viewport.queue_free()


func _setup_overlay() -> void:
	_overlay_viewport = SubViewport.new()
	_overlay_viewport.transparent_bg = true
	_overlay_viewport.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	_overlay_viewport.size = Vector2i(720, 720)

	_overlay_rect = ColorRect.new()
	_overlay_rect.set_anchors_preset(Control.PRESET_FULL_RECT)
	_overlay_rect.mouse_filter = Control.MOUSE_FILTER_IGNORE

	var shader = load("res://assets/no-export/phone_frame_overlay.gdshader")
	if shader:
		_shader_material = ShaderMaterial.new()
		_shader_material.shader = shader
		_overlay_rect.material = _shader_material

	_overlay_viewport.add_child(_overlay_rect)
	add_child(_overlay_viewport)


func _on_preset_selected(index: int) -> void:
	_apply_preset(index)


func _apply_preset(index: int) -> void:
	var preset = PRESETS[index]
	var vp_width: int = preset[1]
	var vp_height: int = preset[2]
	var is_ios: bool = preset[3]
	var is_portrait: bool = preset[4]
	var is_active: bool = index != 0

	# 1. Set viewport size in memory only (updates the blue guide in 2D editor)
	ProjectSettings.set_setting("display/window/size/viewport_width", vp_width)
	ProjectSettings.set_setting("display/window/size/viewport_height", vp_height)

	# 2. Set custom keys for @tool scripts to read
	ProjectSettings.set_setting("_mobile_preview/active", is_active)
	ProjectSettings.set_setting("_mobile_preview/is_ios", is_ios)
	ProjectSettings.set_setting("_mobile_preview/is_portrait", is_portrait)
	ProjectSettings.set_setting("_mobile_preview/viewport_width", vp_width)
	ProjectSettings.set_setting("_mobile_preview/viewport_height", vp_height)

	# 3. Update main_run_args so Play matches the preview
	if is_active:
		var run_args := ""
		if is_ios:
			run_args = "--emulate-ios"
		else:
			run_args = "--emulate-android"
		if not is_portrait:
			run_args += " --landscape"
		ProjectSettings.set_setting("editor/run/main_run_args", run_args)
	else:
		ProjectSettings.set_setting("editor/run/main_run_args", "")

	# 4. Update phone frame overlay
	if _shader_material:
		_shader_material.set_shader_parameter("is_ios", is_ios)
		_shader_material.set_shader_parameter("is_portrait", is_portrait)
	if _overlay_viewport:
		_overlay_viewport.size = Vector2i(vp_width, vp_height)

	# 5. Persist selection to EditorSettings
	EditorInterface.get_editor_settings().set_setting(SETTINGS_KEY, index)

	# 6. Trigger redraw
	update_overlays()


func _forward_canvas_draw_over_viewport(overlay: Control) -> void:
	var index: int = _option_button.selected if is_instance_valid(_option_button) else 0
	if index == 0:
		return
	if _overlay_viewport == null or _shader_material == null:
		return

	var texture := _overlay_viewport.get_texture()
	if texture == null:
		return

	var preset = PRESETS[index]
	var vp_width: float = preset[1]
	var vp_height: float = preset[2]

	# Map scene viewport (0,0)-(vp_width, vp_height) to overlay coordinates
	var xform: Transform2D = overlay.get_viewport_transform() * overlay.get_canvas_transform()
	var top_left: Vector2 = xform * Vector2.ZERO
	var bottom_right: Vector2 = xform * Vector2(vp_width, vp_height)

	overlay.draw_texture_rect(texture, Rect2(top_left, bottom_right - top_left), false)
