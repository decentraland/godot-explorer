extends "res://addons/dcl_dev_tools/dev_tools/dcl_dev_tool.gd"

# Captures the skybox at every hour from 5 directions to compare A/B against Unity reference
# screenshots in color-lighting-test-scene/colortool/Screenshots/. Output naming matches:
# {N|E|S|W|U}{HH}.png so the existing colortool/index.html and skybox_analyzer.py work.

const SKY_LIGHTS = preload("res://assets/environment/sky_lights.tscn")
const ENVIRONMENT = preload("res://assets/environment/game_environment.tres")
const SUN_OPACITY = preload("res://assets/environment/sun_opacity_curve.tres")
const SUN_SIZE = preload("res://assets/environment/sun_size_curve.tres")
const MOON_MASK = preload("res://assets/environment/moon_mask_size_curve.tres")
const DIR_LIGHT_GRAD = preload("res://assets/environment/gradients/directional_light_color.tres")

# Look directions and up vectors. U uses (0,0,-1) up to avoid gimbal lock.
const DIRECTIONS: Array = [
	["N", Vector3(0, 0, 1), Vector3.UP],
	["E", Vector3(1, 0, 0), Vector3.UP],
	["S", Vector3(0, 0, -1), Vector3.UP],
	["W", Vector3(-1, 0, 0), Vector3.UP],
	["U", Vector3(0, 1, 0), Vector3(0, 0, -1)],
]

# Cubemap keyframes — uniform 2h spacing so the runtime shader can derive the adjacent
# layer indices and blend factor with pure arithmetic (no loop, no branches).
# Must stay in sync with BAKE_LAYERS in sky.gdshader and slices/amount in atm_array.png.import.
const BAKE_HOURS: Array = [0, 2, 4, 6, 8, 10, 12, 14, 16, 18, 20, 22]

# Face order matches Godot's Cubemap layer convention (+X, -X, +Y, -Y, +Z, -Z).
# Up vectors use Godot/Vulkan convention (world up for sides). The OpenGL cubemap spec
# wants face top = -Y world for sides, but Godot's framebuffer is already Y-flipped vs
# OpenGL, so we feed natural world-up and the resulting image lines up correctly when
# sampled with samplerCube + EYEDIR.
const FACE_DIRS: Array = [
	["px", Vector3(1, 0, 0), Vector3(0, 1, 0)],
	["nx", Vector3(-1, 0, 0), Vector3(0, 1, 0)],
	["py", Vector3(0, 1, 0), Vector3(0, 0, -1)],
	["ny", Vector3(0, -1, 0), Vector3(0, 0, 1)],
	["pz", Vector3(0, 0, 1), Vector3(0, 1, 0)],
	["nz", Vector3(0, 0, -1), Vector3(0, 1, 0)],
]

# All keyframes baked into a single super-atlas (atm_array.png) — 6 faces wide × N
# layers tall, one cubemap per row. Imported as CompressedCubemapArray so runtime is a
# single load() with no image processing. Tiny resolution: atmosphere is smooth color
# data and runtime lerps adjacent keyframes anyway. Total: 6×8 cells of 64² ≈ 40 KB.
const BAKE_FACE_RESOLUTION := Vector2i(64, 64)

# Bake output goes next to atm_array.png.import which configures CubemapArray import.
const DEFAULT_BAKE_REL := "assets/environment/"

const RESOLUTION_PRESETS: Array = [
	["1920 × 1080 (fast)", Vector2i(1920, 1080)],
	["2560 × 1440", Vector2i(2560, 1440)],
	["4480 × 2520 (Unity ref)", Vector2i(4480, 2520)],
]

const DEFAULT_UNITY_REF_DIR := "/Users/lordmanuel/Projects/decentraland/color-lighting-test-scene/colortool/Screenshots/"

# Output goes OUTSIDE godot/ so the editor doesn't try to import each PNG (causes freeze).
const DEFAULT_OUTPUT_REL := "../tmp/skybox_godot/"
const DEFAULT_APPROVED_REL := "../tmp/skybox_approved/"

var dialog: AcceptDialog
var output_input: LineEdit
var unity_ref_input: LineEdit
var target_hours_input: LineEdit
var resolution_dropdown: OptionButton
var status_label: Label
var capture_button: Button
var compare_button: Button
var approve_button: Button
var bake_button: Button
var capturing := false
var baking := false


func populate_menu(menu: PopupMenu, id: int):
	menu.add_item("Skybox Snapshot...", id)


func execute():
	if not dialog:
		_create_dialog()
	dialog.popup_centered()


func cleanup():
	if dialog and is_instance_valid(dialog):
		dialog.queue_free()
		dialog = null


func _create_dialog():
	dialog = AcceptDialog.new()
	dialog.title = "Skybox Snapshot"
	dialog.size = Vector2i(720, 360)
	dialog.unresizable = false
	dialog.get_ok_button().hide()

	var vbox := VBoxContainer.new()
	vbox.add_theme_constant_override("separation", 8)

	var info := Label.new()
	info.text = (
		"Captures 24 hours × 5 directions = 120 PNGs.\n"
		+ "Naming: {N|E|S|W|U}{HH}.png to match Unity reference screenshots."
	)
	info.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	vbox.add_child(info)

	vbox.add_child(HSeparator.new())

	# Output dir row
	var out_hbox := HBoxContainer.new()
	var out_label := Label.new()
	out_label.text = "Output dir:"
	out_label.custom_minimum_size = Vector2(110, 0)
	out_hbox.add_child(out_label)
	output_input = LineEdit.new()
	output_input.text = DEFAULT_OUTPUT_REL
	output_input.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	out_hbox.add_child(output_input)
	vbox.add_child(out_hbox)

	# Unity ref dir row
	var ref_hbox := HBoxContainer.new()
	var ref_label := Label.new()
	ref_label.text = "Unity ref dir:"
	ref_label.custom_minimum_size = Vector2(110, 0)
	ref_hbox.add_child(ref_label)
	unity_ref_input = LineEdit.new()
	unity_ref_input.text = DEFAULT_UNITY_REF_DIR
	unity_ref_input.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	ref_hbox.add_child(unity_ref_input)
	vbox.add_child(ref_hbox)

	# Target hours row — comma-separated hours we're actively tuning vs Unity this iteration.
	# All other hours are regression-checked against approved/ instead.
	var tgt_hbox := HBoxContainer.new()
	var tgt_label := Label.new()
	tgt_label.text = "Target hours:"
	tgt_label.custom_minimum_size = Vector2(110, 0)
	tgt_hbox.add_child(tgt_label)
	target_hours_input = LineEdit.new()
	target_hours_input.text = "00,12"
	target_hours_input.placeholder_text = "e.g. 00,12,06,18"
	target_hours_input.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	tgt_hbox.add_child(target_hours_input)
	vbox.add_child(tgt_hbox)

	# Resolution dropdown
	var res_hbox := HBoxContainer.new()
	var res_label := Label.new()
	res_label.text = "Resolution:"
	res_label.custom_minimum_size = Vector2(110, 0)
	res_hbox.add_child(res_label)
	resolution_dropdown = OptionButton.new()
	for preset in RESOLUTION_PRESETS:
		resolution_dropdown.add_item(preset[0])
	resolution_dropdown.selected = 0
	res_hbox.add_child(resolution_dropdown)
	vbox.add_child(res_hbox)

	# Capture row
	var btn_hbox := HBoxContainer.new()
	capture_button = Button.new()
	capture_button.text = "Capture Sequence"
	capture_button.pressed.connect(_on_capture_pressed)
	btn_hbox.add_child(capture_button)

	var open_dir_btn := Button.new()
	open_dir_btn.text = "Open output folder"
	open_dir_btn.pressed.connect(_on_open_folder_pressed)
	btn_hbox.add_child(open_dir_btn)
	vbox.add_child(btn_hbox)

	# Compare row
	var cmp_hbox := HBoxContainer.new()
	compare_button = Button.new()
	compare_button.text = "Compare (Unity for targets, approved for rest)"
	compare_button.pressed.connect(_on_compare_pressed)
	cmp_hbox.add_child(compare_button)

	var open_report_btn := Button.new()
	open_report_btn.text = "Open last report"
	open_report_btn.pressed.connect(_on_open_report_pressed)
	cmp_hbox.add_child(open_report_btn)
	vbox.add_child(cmp_hbox)

	# Approve row — locks current target hours into the approved/ regression baseline
	var apr_hbox := HBoxContainer.new()
	approve_button = Button.new()
	approve_button.text = "Approve target hours → approved/"
	approve_button.pressed.connect(_on_approve_pressed)
	apr_hbox.add_child(approve_button)

	var clear_approved_btn := Button.new()
	clear_approved_btn.text = "Clear approved/"
	clear_approved_btn.pressed.connect(_on_clear_approved_pressed)
	apr_hbox.add_child(clear_approved_btn)
	vbox.add_child(apr_hbox)

	vbox.add_child(HSeparator.new())

	# Bake row — Phase E2 atmosphere cubemap baker (8 hours × 6 faces).
	var bake_hbox := HBoxContainer.new()
	bake_button = Button.new()
	bake_button.text = "Bake Atmosphere Cubemaps (8 × 6 faces)"
	bake_button.pressed.connect(_on_bake_pressed)
	bake_hbox.add_child(bake_button)

	var open_bake_btn := Button.new()
	open_bake_btn.text = "Open bake folder"
	open_bake_btn.pressed.connect(_on_open_bake_folder_pressed)
	bake_hbox.add_child(open_bake_btn)
	vbox.add_child(bake_hbox)

	vbox.add_child(HSeparator.new())

	status_label = Label.new()
	status_label.text = "Ready."
	status_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	vbox.add_child(status_label)

	dialog.add_child(vbox)
	plugin.get_editor_interface().get_base_control().add_child(dialog)


func _on_open_folder_pressed():
	var abs_dir := ProjectSettings.globalize_path("res://" + output_input.text)
	DirAccess.make_dir_recursive_absolute(abs_dir)
	OS.shell_open(abs_dir)


func _report_dir() -> String:
	# Sibling of the godot capture dir
	var godot_capture := ProjectSettings.globalize_path("res://" + output_input.text)
	return godot_capture.get_base_dir().path_join("skybox_compare_report") + "/"


func _on_open_report_pressed():
	var report_html := _report_dir() + "index.html"
	if FileAccess.file_exists(report_html):
		OS.shell_open(report_html)
	else:
		status_label.text = "No report yet. Run 'Compare with Unity Reference' first."


func _approved_dir() -> String:
	return ProjectSettings.globalize_path("res://" + DEFAULT_APPROVED_REL)


func _parse_target_hours() -> Array:
	var raw := target_hours_input.text.strip_edges()
	if raw.is_empty():
		return []
	var hours: Array = []
	for tok in raw.split(","):
		var s := tok.strip_edges()
		if s.is_valid_int():
			hours.append("%02d" % s.to_int())
	return hours


func _on_compare_pressed():
	var unity_dir := unity_ref_input.text.strip_edges()
	var godot_dir := ProjectSettings.globalize_path("res://" + output_input.text)
	if not DirAccess.dir_exists_absolute(unity_dir):
		status_label.text = "Unity ref dir not found: %s" % unity_dir
		return
	if not DirAccess.dir_exists_absolute(godot_dir):
		status_label.text = "Godot capture dir not found. Run 'Capture Sequence' first."
		return

	var approved_dir := _approved_dir()
	DirAccess.make_dir_recursive_absolute(approved_dir)
	var target_hours := _parse_target_hours()

	var out_dir := _report_dir()
	DirAccess.make_dir_recursive_absolute(out_dir)

	var script_path := ProjectSettings.globalize_path("res://../tools/skybox_compare.py")
	var args := PackedStringArray(
		[
			script_path,
			unity_dir,
			godot_dir,
			out_dir,
			"--approved-dir",
			approved_dir,
			"--target-hours",
			",".join(target_hours),
		]
	)
	status_label.text = "Running comparison... (may take 30-60s)"
	compare_button.disabled = true
	await plugin.get_tree().process_frame

	var output: Array = []
	var exit_code := OS.execute("python3", args, output, true)
	compare_button.disabled = false

	if exit_code != 0:
		status_label.text = "Compare failed (exit %d). Output:\n%s" % [exit_code, "\n".join(output)]
		return

	var report_html := out_dir + "index.html"
	if FileAccess.file_exists(report_html):
		status_label.text = "Report ready: %s" % report_html
		OS.shell_open(report_html)
	else:
		status_label.text = "Compare done but index.html missing. Output:\n%s" % "\n".join(output)


func _on_approve_pressed():
	var hours := _parse_target_hours()
	if hours.is_empty():
		status_label.text = "No target hours to approve. Set 'Target hours' first."
		return

	var godot_dir := ProjectSettings.globalize_path("res://" + output_input.text)
	var approved_dir := _approved_dir()
	DirAccess.make_dir_recursive_absolute(approved_dir)

	var copied := 0
	var missing: Array = []
	for hour in hours:
		for dir_letter in ["N", "E", "S", "W", "U"]:
			var fname := "%s%s.png" % [dir_letter, hour]
			var src := godot_dir.path_join(fname)
			var dst := approved_dir.path_join(fname)
			if FileAccess.file_exists(src):
				DirAccess.copy_absolute(src, dst)
				copied += 1
			else:
				missing.append(fname)
	var msg := "Approved %d images for hours %s into %s" % [copied, hours, approved_dir]
	if not missing.is_empty():
		msg += "\nMissing: %s" % ", ".join(missing)
	status_label.text = msg


func _on_clear_approved_pressed():
	var approved_dir := _approved_dir()
	if not DirAccess.dir_exists_absolute(approved_dir):
		status_label.text = "No approved/ dir to clear."
		return
	var dir := DirAccess.open(approved_dir)
	if dir == null:
		return
	dir.list_dir_begin()
	var n := 0
	var f := dir.get_next()
	while f != "":
		if f.ends_with(".png"):
			dir.remove(f)
			n += 1
		f = dir.get_next()
	status_label.text = "Cleared %d approved images." % n


func _on_capture_pressed():
	if capturing:
		return
	capturing = true
	capture_button.disabled = true

	var rel_dir := output_input.text
	if not rel_dir.ends_with("/"):
		rel_dir += "/"
	var abs_dir := ProjectSettings.globalize_path("res://" + rel_dir)
	DirAccess.make_dir_recursive_absolute(abs_dir)

	var resolution: Vector2i = RESOLUTION_PRESETS[resolution_dropdown.selected][1]

	await _capture_sequence(abs_dir, resolution)

	capturing = false
	capture_button.disabled = false


func _capture_sequence(out_dir: String, resolution: Vector2i):
	status_label.text = "Setting up capture viewport..."
	await plugin.get_tree().process_frame

	# Build offscreen capture rig: SubViewport + Camera + WorldEnvironment + SkyLights.
	# own_world_3d=true so the WorldEnvironment we add affects THIS viewport's world, not the
	# editor's parent world (otherwise captures pick up the editor's default sky).
	var viewport := SubViewport.new()
	viewport.size = resolution
	viewport.transparent_bg = false
	viewport.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	viewport.msaa_3d = Viewport.MSAA_4X
	viewport.own_world_3d = true

	var world_env := WorldEnvironment.new()
	world_env.environment = ENVIRONMENT
	viewport.add_child(world_env)

	var sky_lights = SKY_LIGHTS.instantiate()
	viewport.add_child(sky_lights)

	var camera := Camera3D.new()
	camera.fov = 70.0
	viewport.add_child(camera)
	camera.current = true

	# Add to editor's scene tree so frames actually render
	plugin.get_editor_interface().get_base_control().add_child(viewport)
	await plugin.get_tree().process_frame
	await plugin.get_tree().process_frame

	var anim_player: AnimationPlayer = sky_lights.get_node("AnimationPlayer")
	var main_light: DirectionalLight3D = sky_lights.get_node("MainLight")
	anim_player.play("light_cycle")
	anim_player.pause()

	var total := 24 * DIRECTIONS.size()
	var done := 0
	for hour in range(24):
		var skybox_time: float = float(hour) / 24.0
		_drive_skybox(skybox_time, anim_player, main_light)

		# Let the animation+uniforms settle before camera starts moving
		await plugin.get_tree().process_frame

		for entry in DIRECTIONS:
			var dir_name: String = entry[0]
			var look_at: Vector3 = entry[1]
			var up: Vector3 = entry[2]

			camera.look_at_from_position(Vector3.ZERO, look_at, up)

			# Two frames: one for camera/uniform update, one for shader to settle
			await plugin.get_tree().process_frame
			await plugin.get_tree().process_frame

			var img := viewport.get_texture().get_image()
			var path := "%s%s%02d.png" % [out_dir, dir_name, hour]
			var err := img.save_png(path)
			if err != OK:
				push_error("Failed to save %s (err=%d)" % [path, err])

			done += 1
			status_label.text = ("Captured %d/%d  (%s%02d.png)" % [done, total, dir_name, hour])

	viewport.queue_free()
	status_label.text = "Done. %d images written to %s" % [total, out_dir]


func _bake_dir() -> String:
	return ProjectSettings.globalize_path("res://" + DEFAULT_BAKE_REL)


func _on_open_bake_folder_pressed():
	var abs_dir := _bake_dir()
	DirAccess.make_dir_recursive_absolute(abs_dir)
	OS.shell_open(abs_dir)


func _on_bake_pressed():
	if baking:
		return
	baking = true
	bake_button.disabled = true

	var abs_dir := _bake_dir()
	DirAccess.make_dir_recursive_absolute(abs_dir)

	await _bake_cubemaps(abs_dir)

	baking = false
	bake_button.disabled = false


# Renders the atmosphere-only sky (atm_bake_mode=true) into 6 cubemap faces × N keyframes.
# Output PNGs go into godot/assets/environment/sky_baked/ so Godot imports them as
# CompressedTexture2D — Phase E3 then assembles them into Cubemap resources.
#
# Uses a custom clean Environment (linear tonemap, no glow, no auto-exposure) instead of
# game_environment.tres. With filmic tonemap + glow, each face gets a different non-linear
# response (avg luminance of +Y differs from sides), producing visible seams in the cubemap.
func _bake_cubemaps(out_dir: String):
	status_label.text = "Setting up bake viewport..."
	await plugin.get_tree().process_frame

	var viewport := SubViewport.new()
	viewport.size = BAKE_FACE_RESOLUTION
	viewport.transparent_bg = false
	viewport.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	viewport.msaa_3d = Viewport.MSAA_DISABLED
	viewport.own_world_3d = true

	# Clean env so each face renders identically. Glow OFF (was causing the seams: bright
	# pixels bleed within a face but not across faces, so edges differ from neighbors).
	# Keep filmic tonemap — it's per-pixel deterministic, so identical EYEDIR samples produce
	# identical output across face boundaries (no seam). Linear tonemap clipped Rayleigh's
	# tiny linear values (~0.1) to near-black before sRGB encoding.
	var bake_env := Environment.new()
	bake_env.background_mode = Environment.BG_SKY
	bake_env.sky = ENVIRONMENT.sky
	bake_env.tonemap_mode = Environment.TONE_MAPPER_FILMIC
	bake_env.tonemap_exposure = 1.0
	bake_env.tonemap_white = 1.0
	bake_env.glow_enabled = false
	bake_env.adjustment_enabled = false
	bake_env.ambient_light_source = Environment.AMBIENT_SOURCE_DISABLED

	var world_env := WorldEnvironment.new()
	world_env.environment = bake_env
	viewport.add_child(world_env)

	var sky_lights = SKY_LIGHTS.instantiate()
	viewport.add_child(sky_lights)

	var camera := Camera3D.new()
	camera.fov = 90.0  # cubemap face FOV is exactly 90°
	camera.near = 0.01
	camera.far = 1000.0
	viewport.add_child(camera)
	camera.current = true

	plugin.get_editor_interface().get_base_control().add_child(viewport)
	await plugin.get_tree().process_frame
	await plugin.get_tree().process_frame

	var anim_player: AnimationPlayer = sky_lights.get_node("AnimationPlayer")
	var main_light: DirectionalLight3D = sky_lights.get_node("MainLight")
	anim_player.play("light_cycle")
	anim_player.pause()

	# Flip the bake-mode global on. Restored in the cleanup branch below regardless of error.
	RenderingServer.global_shader_parameter_set("atm_bake_mode", true)

	var face_size: int = BAKE_FACE_RESOLUTION.x
	var num_layers: int = BAKE_HOURS.size()

	# Super-atlas for CubemapArray: 6 faces wide × N hours tall. Cells are face_size².
	# Imported as CompressedCubemapArray (.import file specifies arrangement=6x1, vertical
	# stacking, amount=N). Single asset — no runtime slicing.
	var atlas := Image.create(face_size * 6, face_size * num_layers, false, Image.FORMAT_RGBA8)

	for layer in range(num_layers):
		var hour: float = BAKE_HOURS[layer]
		var skybox_time: float = hour / 24.0
		_drive_skybox(skybox_time, anim_player, main_light)
		await plugin.get_tree().process_frame

		for i in range(FACE_DIRS.size()):
			var entry: Array = FACE_DIRS[i]
			var look_at: Vector3 = entry[1]
			var up: Vector3 = entry[2]

			camera.look_at_from_position(Vector3.ZERO, look_at, up)
			await plugin.get_tree().process_frame
			await plugin.get_tree().process_frame

			var img := viewport.get_texture().get_image()
			# Godot Camera3D produces an image whose horizontal axis is mirrored vs the
			# OpenGL/Vulkan cubemap convention (samplerCube expects image right = -Z for +X
			# face, but Godot renders +Z to image right). Mirror once to align all 6 faces.
			img.flip_x()
			img.convert(Image.FORMAT_RGBA8)
			var dst := Vector2i(i * face_size, layer * face_size)
			atlas.blit_rect(img, Rect2i(0, 0, face_size, face_size), dst)

		status_label.text = ("Baked layer %d/%d (hour %02d)" % [layer + 1, num_layers, int(hour)])

	var path := "%satm_array.png" % out_dir
	var err := atlas.save_png(path)
	if err != OK:
		push_error("Failed to save %s (err=%d)" % [path, err])

	RenderingServer.global_shader_parameter_set("atm_bake_mode", false)
	viewport.queue_free()
	status_label.text = (
		"Bake done. Super-atlas: %s (%d×%d)" % [path, atlas.get_width(), atlas.get_height()]
	)


# Replicates sky_base.gd's _process logic so the editor doesn't need Global.skybox_time.
func _drive_skybox(
	skybox_time: float, anim_player: AnimationPlayer, main_light: DirectionalLight3D
):
	RenderingServer.global_shader_parameter_set("day_night_cycle", skybox_time)
	anim_player.seek(skybox_time, true)

	var sun_dir = main_light.global_transform.basis.z
	RenderingServer.global_shader_parameter_set("sun_direction", sun_dir)
	RenderingServer.global_shader_parameter_set("moon_direction", -sun_dir)

	# Atm sun: keep visual sun's azimuth (X/Z), synthesize Y so it cycles below horizon
	# at night — see sky_base.gd for the formula.
	var atm_sun_dir = Vector3(sun_dir.x, -cos(TAU * skybox_time), sun_dir.z).normalized()
	RenderingServer.global_shader_parameter_set("atm_sun_direction", atm_sun_dir)

	RenderingServer.global_shader_parameter_set("sun_opacity", SUN_OPACITY.sample(skybox_time))
	RenderingServer.global_shader_parameter_set("sun_size", SUN_SIZE.sample(skybox_time))
	RenderingServer.global_shader_parameter_set("moon_mask_size", MOON_MASK.sample(skybox_time))

	# Energy from elevation, matching sky_base.gd
	var light_dir = -main_light.global_transform.basis.z
	var elevation = -light_dir.y
	var energy_factor = smoothstep(-0.05, 0.3, elevation)
	main_light.visible = energy_factor > 0.01
	main_light.light_energy = 0.7 * energy_factor

	if DIR_LIGHT_GRAD:
		main_light.light_color = DIR_LIGHT_GRAD.sample(skybox_time)
