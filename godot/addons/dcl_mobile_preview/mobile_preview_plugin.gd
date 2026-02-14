@tool
extends EditorPlugin

# Device definitions: [label, portrait_w, portrait_h, landscape_w, landscape_h, is_ios, color]
const DEVICES := [
	["Default", 720, 720, 720, 720, false, Color(0.5, 0.5, 0.5)],
	["iPhone 14 Pro", 590, 1280, 1561, 720, true, Color(0.35, 0.6, 1.0)],
	["Moto Edge 60 Pro", 576, 1280, 1600, 720, false, Color(0.35, 0.85, 0.45)],
]

const SCENE_META_KEY := "mobile_preview_orientation"
const SETTINGS_DEVICE_KEY := "dcl_mobile_preview/last_device"
const SETTINGS_ORIENT_KEY := "dcl_mobile_preview/last_orientation"

const ORIENT_BOTH := 0
const ORIENT_PORTRAIT := 1
const ORIENT_LANDSCAPE := 2

# Toolbar controls
var _device_button: OptionButton
var _orient_toggle: Button

# Scene navbar menu (like DCL Tools)
var _scene_menu_2d: MenuButton
var _scene_menu_3d: MenuButton

# Overlay (generated ImageTexture — no SubViewport/shader needed)
var _overlay_texture: ImageTexture

# Dialogs
var _confirm_dialog: ConfirmationDialog
var _error_dialog: AcceptDialog
var _pending_apply: bool = false

# State
var _is_portrait: bool = true
var _icon_portrait: ImageTexture
var _icon_landscape: ImageTexture
var _clean_versions: Dictionary = {}  # scene_path -> UndoRedo version at load/save
var _renderer_control: Control  # Editor's rendering method chooser (hidden when preview active)


func _enter_tree() -> void:
	set_force_draw_over_forwarding_enabled()
	_create_icons()

	# --- Toolbar: device dropdown + orientation toggle ---
	_device_button = OptionButton.new()
	_device_button.flat = true
	_device_button.fit_to_longest_item = false
	_device_button.tooltip_text = "Mobile Preview Device"
	var bold_font := EditorInterface.get_editor_theme().get_font("bold", "EditorFonts")
	_device_button.add_theme_font_override("font", bold_font)
	for i in DEVICES.size():
		_device_button.add_item(DEVICES[i][0], i)
		_device_button.set_item_icon(i, _make_color_circle(DEVICES[i][6]))
	_device_button.item_selected.connect(_on_device_selected)
	add_control_to_container(CONTAINER_TOOLBAR, _device_button)

	_orient_toggle = Button.new()
	_orient_toggle.flat = true
	_orient_toggle.tooltip_text = "Toggle preview orientation"
	_orient_toggle.pressed.connect(_on_orient_pressed)
	add_control_to_container(CONTAINER_TOOLBAR, _orient_toggle)

	# --- Scene navbar: orientation config menu (like DCL Tools) ---
	_scene_menu_2d = _create_scene_menu()
	add_control_to_container(CONTAINER_CANVAS_EDITOR_MENU, _scene_menu_2d)

	_scene_menu_3d = _create_scene_menu()
	add_control_to_container(CONTAINER_SPATIAL_EDITOR_MENU, _scene_menu_3d)

	# --- Dialogs ---
	_confirm_dialog = ConfirmationDialog.new()
	_confirm_dialog.title = "Reload Scene"
	_confirm_dialog.dialog_text = "Save and reload the current scene to apply the new resolution?"
	_confirm_dialog.ok_button_text = "Save & Reload"
	_confirm_dialog.confirmed.connect(_on_confirm_save_reload)
	_confirm_dialog.canceled.connect(_on_confirm_canceled)
	EditorInterface.get_base_control().add_child(_confirm_dialog)

	_error_dialog = AcceptDialog.new()
	_error_dialog.title = "Orientation Locked"
	EditorInterface.get_base_control().add_child(_error_dialog)

	scene_changed.connect(_on_scene_changed)

	# Hide the rendering method chooser to save toolbar space
	_renderer_control = _find_renderer_option_button()
	if is_instance_valid(_renderer_control):
		_renderer_control.visible = false

	# Restore last selections
	var es := EditorInterface.get_editor_settings()
	var last_device: int = es.get_setting(SETTINGS_DEVICE_KEY) if es.has_setting(SETTINGS_DEVICE_KEY) else 0
	var last_orient: int = es.get_setting(SETTINGS_ORIENT_KEY) if es.has_setting(SETTINGS_ORIENT_KEY) else 0
	_is_portrait = (last_orient == 0)
	_device_button.select(last_device)

	_sync_orient_from_scene()
	_apply_current()
	_record_clean_version()


func _exit_tree() -> void:
	_apply_settings(0, false)
	if is_instance_valid(_renderer_control):
		_renderer_control.visible = true

	if is_instance_valid(_device_button):
		remove_control_from_container(CONTAINER_TOOLBAR, _device_button)
		_device_button.queue_free()
	if is_instance_valid(_orient_toggle):
		remove_control_from_container(CONTAINER_TOOLBAR, _orient_toggle)
		_orient_toggle.queue_free()
	if is_instance_valid(_scene_menu_2d):
		remove_control_from_container(CONTAINER_CANVAS_EDITOR_MENU, _scene_menu_2d)
		_scene_menu_2d.queue_free()
	if is_instance_valid(_scene_menu_3d):
		remove_control_from_container(CONTAINER_SPATIAL_EDITOR_MENU, _scene_menu_3d)
		_scene_menu_3d.queue_free()
	if is_instance_valid(_confirm_dialog):
		_confirm_dialog.queue_free()
	if is_instance_valid(_error_dialog):
		_error_dialog.queue_free()


# --- Find editor renderer control ---

func _find_renderer_option_button() -> Control:
	var base := EditorInterface.get_base_control()
	var editor_node := base.get_parent()
	if not editor_node:
		return null
	return _find_renderer_in(editor_node, 6)


func _find_renderer_in(node: Node, depth: int) -> Control:
	if depth < 0:
		return null
	if node is OptionButton and "rendering" in node.tooltip_text.to_lower():
		return node
	for child in node.get_children():
		var found := _find_renderer_in(child, depth - 1)
		if found:
			return found
	return null


# --- Scene navbar menu ---

func _create_scene_menu() -> MenuButton:
	var menu := MenuButton.new()
	menu.text = " ▾"
	menu.icon = _icon_portrait
	menu.tooltip_text = "Scene Orientation"
	var popup := menu.get_popup()
	popup.add_radio_check_item("Both Orientations", ORIENT_BOTH)
	popup.add_radio_check_item("Portrait Only", ORIENT_PORTRAIT)
	popup.add_radio_check_item("Landscape Only", ORIENT_LANDSCAPE)
	popup.id_pressed.connect(_on_scene_orient_selected)
	return menu


func _sync_scene_menus() -> void:
	var current := _get_scene_orientation_id()
	for menu in [_scene_menu_2d, _scene_menu_3d]:
		if not is_instance_valid(menu):
			continue
		var popup: PopupMenu = menu.get_popup()
		for i in 3:
			popup.set_item_checked(i, i == current)


func _on_scene_orient_selected(id: int) -> void:
	var root = EditorInterface.get_edited_scene_root()
	if not root:
		return

	match id:
		ORIENT_PORTRAIT:
			root.set_meta(SCENE_META_KEY, "portrait")
		ORIENT_LANDSCAPE:
			root.set_meta(SCENE_META_KEY, "landscape")
		_:
			root.remove_meta(SCENE_META_KEY)

	_sync_scene_menus()
	_sync_orient_from_scene()
	_request_apply_with_reload()


# --- Icon creation (32x32, 2px lines) ---

static func _make_color_circle(color: Color) -> ImageTexture:
	var s := 12
	var img := Image.create(s, s, false, Image.FORMAT_RGBA8)
	img.fill(Color.TRANSPARENT)
	var center := Vector2(s / 2.0, s / 2.0)
	var radius := s / 2.0 - 0.5
	for y in s:
		for x in s:
			if Vector2(x + 0.5, y + 0.5).distance_to(center) <= radius:
				img.set_pixel(x, y, color)
	return ImageTexture.create_from_image(img)


func _create_icons() -> void:
	_icon_portrait = _make_phone_icon(true)
	_icon_landscape = _make_phone_icon(false)


static func _make_phone_icon(vertical: bool) -> ImageTexture:
	var s := 32
	var img := Image.create(s, s, false, Image.FORMAT_RGBA8)
	img.fill(Color.TRANSPARENT)
	var c := Color(0.82, 0.82, 0.82, 1.0)
	if vertical:
		_draw_rounded_rect_thick(img, Rect2i(9, 2, 14, 28), c, 2, 3)
	else:
		_draw_rounded_rect_thick(img, Rect2i(2, 9, 28, 14), c, 2, 3)
	return ImageTexture.create_from_image(img)


static func _draw_rounded_rect_thick(
	img: Image, rect: Rect2i, color: Color, thickness: int, radius: int
) -> void:
	for t in thickness:
		var rx := rect.position.x + t
		var ry := rect.position.y + t
		var rw := (rect.end.x - t) - rx
		var rh := (rect.end.y - t) - ry
		var r := max(radius - t, 0)
		_draw_rect_outline(img, rx, ry, rw, rh, r, color)


static func _draw_rect_outline(
	img: Image, x0: int, y0: int, w: int, h: int, r: int, color: Color
) -> void:
	var x1 := x0 + w - 1
	var y1 := y0 + h - 1
	for x in range(x0 + r, x1 - r + 1):
		img.set_pixel(x, y0, color)
		img.set_pixel(x, y1, color)
	for y in range(y0 + r, y1 - r + 1):
		img.set_pixel(x0, y, color)
		img.set_pixel(x1, y, color)
	_draw_corner_arc(img, x0 + r, y0 + r, r, color)
	_draw_corner_arc(img, x1 - r, y0 + r, r, color)
	_draw_corner_arc(img, x0 + r, y1 - r, r, color)
	_draw_corner_arc(img, x1 - r, y1 - r, r, color)


static func _draw_corner_arc(img: Image, cx: int, cy: int, r: int, color: Color) -> void:
	if r <= 0:
		img.set_pixel(cx, cy, color)
		return
	var x := r
	var y := 0
	var err := 1 - r
	while x >= y:
		for px in [cx + x, cx - x, cx + y, cx - y]:
			for py in [cy + x, cy - x, cy + y, cy - y]:
				if px >= 0 and px < img.get_width() and py >= 0 and py < img.get_height():
					img.set_pixel(px, py, color)
		y += 1
		if err < 0:
			err += 2 * y + 1
		else:
			x -= 1
			err += 2 * (y - x) + 1


# --- Overlay image generation (SDF helpers) ---

static func _sdf_rounded_box(p: Vector2, half_size: Vector2, radius: float) -> float:
	var q := Vector2(absf(p.x) - half_size.x + radius, absf(p.y) - half_size.y + radius)
	return Vector2(maxf(q.x, 0.0), maxf(q.y, 0.0)).length() + minf(maxf(q.x, q.y), 0.0) - radius


static func _sdf_pill(p: Vector2, half_size: Vector2) -> float:
	return _sdf_rounded_box(p, half_size, minf(half_size.x, half_size.y))


func _generate_overlay(width: int, height: int, is_ios: bool, is_portrait: bool) -> void:
	var img := Image.create(width, height, false, Image.FORMAT_RGBA8)
	img.fill(Color.TRANSPARENT)

	var min_dim := mini(width, height)
	var r := int(ceil(float(min_dim) * 0.04))
	var center := Vector2(width * 0.5, height * 0.5)
	var half_size := Vector2(width * 0.5, height * 0.5)
	var black := Color.BLACK

	# Rounded corners — only iterate the 4 corner squares (r×r each)
	for cy in r:
		for cx in r:
			var px := float(cx) + 0.5
			var py := float(cy) + 0.5
			if _sdf_rounded_box(Vector2(px, py) - center, half_size, float(r)) > 0.0:
				img.set_pixel(cx, cy, black)
			var rx := width - 1 - cx
			if _sdf_rounded_box(Vector2(float(rx) + 0.5, py) - center, half_size, float(r)) > 0.0:
				img.set_pixel(rx, cy, black)
			var by := height - 1 - cy
			if _sdf_rounded_box(Vector2(px, float(by) + 0.5) - center, half_size, float(r)) > 0.0:
				img.set_pixel(cx, by, black)
			if _sdf_rounded_box(Vector2(float(rx) + 0.5, float(by) + 0.5) - center, half_size, float(r)) > 0.0:
				img.set_pixel(rx, by, black)

	if is_ios:
		# Dynamic island
		var iw: float
		var ih: float
		var icx: float
		var icy: float
		if is_portrait:
			iw = width * 0.27
			ih = height * 0.035
			icx = width * 0.5
			icy = height * 0.012 + ih * 0.5
		else:
			iw = width * 0.035
			ih = height * 0.27
			icx = width * 0.012 + iw * 0.5
			icy = height * 0.5
		var ihs := Vector2(iw, ih) * 0.5
		var ix0 := maxi(0, int(icx - iw * 0.5) - 1)
		var ix1 := mini(width, int(icx + iw * 0.5) + 2)
		var iy0 := maxi(0, int(icy - ih * 0.5) - 1)
		var iy1 := mini(height, int(icy + ih * 0.5) + 2)
		for y in range(iy0, iy1):
			for x in range(ix0, ix1):
				if _sdf_pill(Vector2(x + 0.5, y + 0.5) - Vector2(icx, icy), ihs) < 0.0:
					img.set_pixel(x, y, black)

		# Home indicator
		var hw: float
		var hh: float
		var hcx: float
		var hcy: float
		if is_portrait:
			hw = width * 0.35
			hh = height * 0.005
			hcx = width * 0.5
			hcy = height * (1.0 - 0.008) - hh * 0.5
		else:
			hw = width * 0.35
			hh = height * 0.005 * 2.0
			hcx = width * 0.5
			hcy = height * (1.0 - 0.016) - hh * 0.5
		var hhs := Vector2(hw, hh) * 0.5
		var hx0 := maxi(0, int(hcx - hw * 0.5) - 1)
		var hx1 := mini(width, int(hcx + hw * 0.5) + 2)
		var hy0 := maxi(0, int(hcy - hh * 0.5) - 1)
		var hy1 := mini(height, int(hcy + hh * 0.5) + 2)
		var home_color := Color(0, 0, 0, 0.3)
		for y in range(hy0, hy1):
			for x in range(hx0, hx1):
				if _sdf_pill(Vector2(x + 0.5, y + 0.5) - Vector2(hcx, hcy), hhs) < 0.0:
					img.set_pixel(x, y, home_color)
	else:
		# Android camera hole
		var cam_r := float(min_dim) * 0.018
		var cam_cx: float
		var cam_cy: float
		if is_portrait:
			cam_cx = width * 0.5
			cam_cy = height * 0.022
		else:
			cam_cx = width * 0.022
			cam_cy = height * 0.5
		var cx0 := maxi(0, int(cam_cx - cam_r) - 1)
		var cx1 := mini(width, int(cam_cx + cam_r) + 2)
		var cy0 := maxi(0, int(cam_cy - cam_r) - 1)
		var cy1 := mini(height, int(cam_cy + cam_r) + 2)
		for y in range(cy0, cy1):
			for x in range(cx0, cx1):
				if Vector2(x + 0.5, y + 0.5).distance_to(Vector2(cam_cx, cam_cy)) < cam_r:
					img.set_pixel(x, y, black)

	_overlay_texture = ImageTexture.create_from_image(img)


# --- Scene orientation metadata ---

func _get_scene_orientation_id() -> int:
	var root = EditorInterface.get_edited_scene_root()
	if root and root.has_meta(SCENE_META_KEY):
		var val: String = root.get_meta(SCENE_META_KEY)
		if val == "portrait":
			return ORIENT_PORTRAIT
		if val == "landscape":
			return ORIENT_LANDSCAPE
	return ORIENT_BOTH


func _sync_orient_from_scene() -> void:
	var orient := _get_scene_orientation_id()
	if orient == ORIENT_PORTRAIT:
		_is_portrait = true
	elif orient == ORIENT_LANDSCAPE:
		_is_portrait = false
	# ORIENT_BOTH → keep current _is_portrait
	_update_orient_icon()
	_sync_scene_menus()


func _update_orient_icon() -> void:
	_orient_toggle.icon = _icon_portrait if _is_portrait else _icon_landscape
	if _is_portrait:
		_orient_toggle.tooltip_text = "Preview: Portrait (click to switch to landscape)"
	else:
		_orient_toggle.tooltip_text = "Preview: Landscape (click to switch to portrait)"


# --- Toolbar event handlers ---

func _on_orient_pressed() -> void:
	var want_portrait := not _is_portrait
	var orient := _get_scene_orientation_id()

	if orient == ORIENT_PORTRAIT and not want_portrait:
		_error_dialog.dialog_text = "This scene is set to Portrait Only.\nUse Scene Orientation menu to change it."
		_error_dialog.popup_centered()
		return
	if orient == ORIENT_LANDSCAPE and want_portrait:
		_error_dialog.dialog_text = "This scene is set to Landscape Only.\nUse Scene Orientation menu to change it."
		_error_dialog.popup_centered()
		return

	_is_portrait = want_portrait
	EditorInterface.get_editor_settings().set_setting(SETTINGS_ORIENT_KEY, 0 if _is_portrait else 1)
	_update_orient_icon()
	_request_apply_with_reload()


func _on_device_selected(_index: int) -> void:
	_request_apply_with_reload()


func _on_scene_changed(_scene_root: Node) -> void:
	_sync_orient_from_scene()
	_apply_current()
	_record_clean_version()


# --- Scene change detection ---

func _record_clean_version() -> void:
	var root = EditorInterface.get_edited_scene_root()
	if not root or root.scene_file_path.is_empty():
		return
	var ur_mgr := get_undo_redo()
	var hist_id := ur_mgr.get_object_history_id(root)
	var ur := ur_mgr.get_history_undo_redo(hist_id)
	_clean_versions[root.scene_file_path] = ur.get_version()


func _is_scene_modified() -> bool:
	var root = EditorInterface.get_edited_scene_root()
	if not root or root.scene_file_path.is_empty():
		return false
	var ur_mgr := get_undo_redo()
	var hist_id := ur_mgr.get_object_history_id(root)
	var ur := ur_mgr.get_history_undo_redo(hist_id)
	var current_version := ur.get_version()
	var clean_version: int = _clean_versions.get(root.scene_file_path, -1)
	return current_version != clean_version


# --- Save & reload flow ---

func _request_apply_with_reload() -> void:
	var root = EditorInterface.get_edited_scene_root()
	if root and not root.scene_file_path.is_empty():
		if _is_scene_modified():
			_pending_apply = true
			_confirm_dialog.popup_centered()
		else:
			var scene_path: String = root.scene_file_path
			_apply_current()
			EditorInterface.reload_scene_from_path(scene_path)
	else:
		_apply_current()


func _on_confirm_save_reload() -> void:
	if not _pending_apply:
		return
	_pending_apply = false

	var root = EditorInterface.get_edited_scene_root()
	var scene_path: String = root.scene_file_path if root else ""

	_apply_current()

	if not scene_path.is_empty():
		EditorInterface.save_scene()
		EditorInterface.reload_scene_from_path(scene_path)


func _on_confirm_canceled() -> void:
	if _pending_apply:
		_pending_apply = false
		var es := EditorInterface.get_editor_settings()
		var prev: int = es.get_setting(SETTINGS_DEVICE_KEY) if es.has_setting(SETTINGS_DEVICE_KEY) else 0
		_device_button.select(prev)
		var prev_orient: int = es.get_setting(SETTINGS_ORIENT_KEY) if es.has_setting(SETTINGS_ORIENT_KEY) else 0
		_is_portrait = (prev_orient == 0)
		_update_orient_icon()


# --- Apply ---

func _apply_current() -> void:
	_apply_settings(_device_button.selected, _is_portrait)


func _apply_settings(device_index: int, is_portrait: bool) -> void:
	var device = DEVICES[device_index]
	var is_active: bool = device_index != 0
	var is_ios: bool = device[5]

	var vp_width: int = device[1] if is_portrait else device[3]
	var vp_height: int = device[2] if is_portrait else device[4]

	ProjectSettings.set_setting("display/window/size/viewport_width", vp_width)
	ProjectSettings.set_setting("display/window/size/viewport_height", vp_height)

	ProjectSettings.set_setting("_mobile_preview/active", is_active)
	ProjectSettings.set_setting("_mobile_preview/is_ios", is_ios)
	ProjectSettings.set_setting("_mobile_preview/is_portrait", is_portrait)
	ProjectSettings.set_setting("_mobile_preview/viewport_width", vp_width)
	ProjectSettings.set_setting("_mobile_preview/viewport_height", vp_height)

	if is_active:
		var run_args := "--emulate-ios" if is_ios else "--emulate-android"
		if not is_portrait:
			run_args += " --landscape"
		ProjectSettings.set_setting("editor/run/main_run_args", run_args)
	else:
		ProjectSettings.set_setting("editor/run/main_run_args", "")

	if is_active:
		_generate_overlay(vp_width, vp_height, is_ios, is_portrait)
	else:
		_overlay_texture = null

	if is_instance_valid(_renderer_control):
		_renderer_control.visible = false

	EditorInterface.get_editor_settings().set_setting(SETTINGS_DEVICE_KEY, device_index)

	update_overlays()


# --- Overlay drawing ---

func _forward_canvas_force_draw_over_viewport(overlay: Control) -> void:
	if _overlay_texture == null:
		return
	var device_index: int = _device_button.selected if is_instance_valid(_device_button) else 0
	if device_index == 0:
		return

	var device = DEVICES[device_index]
	var vp_width: float = device[1] if _is_portrait else device[3]
	var vp_height: float = device[2] if _is_portrait else device[4]

	var xform := EditorInterface.get_editor_viewport_2d().get_final_transform()
	var top_left: Vector2 = xform * Vector2.ZERO
	var bottom_right: Vector2 = xform * Vector2(vp_width, vp_height)

	overlay.draw_texture_rect(_overlay_texture, Rect2(top_left, bottom_right - top_left), false)
