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
const SETTINGS_OVERLAY_KEY := "dcl_mobile_preview/overlay_visible"

const ORIENT_NONE := 0
const ORIENT_BOTH := 1
const ORIENT_PORTRAIT := 2
const ORIENT_LANDSCAPE := 3

# Toolbar controls
var _device_button: OptionButton
var _orient_toggle: Button
var _overlay_toggle: Button

# Scene navbar menu (like DCL Tools)
var _scene_menu_2d: MenuButton
var _scene_menu_3d: MenuButton

# Overlay (generated ImageTexture — no SubViewport/shader needed)
var _overlay_texture: ImageTexture
var _bezel: int = 0  # bezel thickness in game pixels

# Dialogs
var _confirm_dialog: ConfirmationDialog
var _error_dialog: AcceptDialog
var _pending_apply: bool = false

# State
var _is_portrait: bool = true
var _scene_orient: int = ORIENT_NONE
var _overlay_visible: bool = true
var _icon_portrait: ImageTexture
var _icon_landscape: ImageTexture
var _icon_overlay_on: ImageTexture
var _icon_overlay_off: ImageTexture
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
	var popup := _device_button.get_popup()
	var empty_img := Image.create(1, 1, false, Image.FORMAT_RGBA8)
	empty_img.fill(Color.TRANSPARENT)
	var empty_icon := ImageTexture.create_from_image(empty_img)
	popup.add_theme_icon_override("radio_checked", empty_icon)
	popup.add_theme_icon_override("radio_unchecked", empty_icon)
	add_control_to_container(CONTAINER_TOOLBAR, _device_button)

	_orient_toggle = Button.new()
	_orient_toggle.flat = true
	_orient_toggle.tooltip_text = "Toggle preview orientation"
	_orient_toggle.pressed.connect(_on_orient_pressed)
	add_control_to_container(CONTAINER_TOOLBAR, _orient_toggle)

	_overlay_toggle = Button.new()
	_overlay_toggle.flat = true
	_overlay_toggle.tooltip_text = "Toggle phone frame overlay"
	_overlay_toggle.pressed.connect(_on_overlay_toggled)
	add_control_to_container(CONTAINER_TOOLBAR, _overlay_toggle)

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
	var last_device: int = (
		es.get_setting(SETTINGS_DEVICE_KEY) if es.has_setting(SETTINGS_DEVICE_KEY) else 0
	)
	var last_orient: int = (
		es.get_setting(SETTINGS_ORIENT_KEY) if es.has_setting(SETTINGS_ORIENT_KEY) else 0
	)
	_overlay_visible = (
		es.get_setting(SETTINGS_OVERLAY_KEY) if es.has_setting(SETTINGS_OVERLAY_KEY) else true
	)
	_is_portrait = (last_orient == 0)
	_device_button.select(last_device)
	_update_overlay_icon()

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
	if is_instance_valid(_overlay_toggle):
		remove_control_from_container(CONTAINER_TOOLBAR, _overlay_toggle)
		_overlay_toggle.queue_free()
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
	popup.add_radio_check_item("None (default)", ORIENT_NONE)
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
		for i in 4:
			popup.set_item_checked(i, i == current)


func _on_scene_orient_selected(id: int) -> void:
	var root = EditorInterface.get_edited_scene_root()
	if not root:
		return

	match id:
		ORIENT_BOTH:
			root.set_meta(SCENE_META_KEY, "both")
		ORIENT_PORTRAIT:
			root.set_meta(SCENE_META_KEY, "portrait")
		ORIENT_LANDSCAPE:
			root.set_meta(SCENE_META_KEY, "landscape")
		_:
			root.remove_meta(SCENE_META_KEY)

	_sync_scene_menus()
	_sync_orient_from_scene()

	# Save first so the meta persists through the reload, then apply + reload
	var scene_path: String = root.scene_file_path
	if not scene_path.is_empty():
		EditorInterface.save_scene()
		_apply_current()
		_record_clean_version()
		EditorInterface.reload_scene_from_path(scene_path)
	else:
		_apply_current()


# --- Icon helpers ---

const ICON_DIR := "res://addons/dcl_mobile_preview/icons/"


static func _load_tinted_icon(svg_name: String, color: Color) -> ImageTexture:
	var path := ICON_DIR + svg_name
	var file := FileAccess.open(path, FileAccess.READ)
	if not file:
		push_error("Cannot open icon: " + path)
		return null
	var svg_text := file.get_as_text()
	file.close()
	var img := Image.new()
	img.load_svg_from_string(svg_text)
	for y in img.get_height():
		for x in img.get_width():
			var px := img.get_pixel(x, y)
			if px.a > 0.0:
				img.set_pixel(x, y, Color(color.r, color.g, color.b, px.a))
	return ImageTexture.create_from_image(img)


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
	var gray := Color(0.82, 0.82, 0.82)
	_icon_portrait = _load_tinted_icon("portrait.svg", gray)
	_icon_landscape = _load_tinted_icon("landscape.svg", gray)
	_icon_overlay_on = _load_tinted_icon("overlay.svg", Color(0.35, 0.65, 1.0))
	_icon_overlay_off = _load_tinted_icon("overlay.svg", Color(0.6, 0.6, 0.6))


# --- Overlay image generation (SDF helpers) ---


static func _sdf_rounded_box(p: Vector2, half_size: Vector2, radius: float) -> float:
	var q := Vector2(absf(p.x) - half_size.x + radius, absf(p.y) - half_size.y + radius)
	return Vector2(maxf(q.x, 0.0), maxf(q.y, 0.0)).length() + minf(maxf(q.x, q.y), 0.0) - radius


static func _sdf_pill(p: Vector2, half_size: Vector2) -> float:
	return _sdf_rounded_box(p, half_size, minf(half_size.x, half_size.y))


func _generate_overlay(width: int, height: int, is_ios: bool, is_portrait: bool) -> void:
	var min_dim := mini(width, height)
	var inner_r := int(ceil(float(min_dim) * 0.04))
	var bz := maxi(6, int(ceil(float(min_dim) * 0.015)))
	_bezel = bz
	var outer_r := inner_r + bz

	var img_w := width + bz * 2
	var img_h := height + bz * 2
	var img := Image.create(img_w, img_h, false, Image.FORMAT_RGBA8)
	var black := Color.BLACK
	var transparent := Color.TRANSPARENT

	# Start with black (the phone body), then carve out transparent areas
	img.fill(black)

	# --- Clear outer corners (outside phone body rounded rect) ---
	var body_center := Vector2(img_w * 0.5, img_h * 0.5)
	var body_half := Vector2(img_w * 0.5, img_h * 0.5)
	for cy in outer_r:
		for cx in outer_r:
			var px := float(cx) + 0.5
			var py := float(cy) + 0.5
			if _sdf_rounded_box(Vector2(px, py) - body_center, body_half, float(outer_r)) > 0.0:
				img.set_pixel(cx, cy, transparent)
			var rx := img_w - 1 - cx
			if (
				_sdf_rounded_box(
					Vector2(float(rx) + 0.5, py) - body_center, body_half, float(outer_r)
				)
				> 0.0
			):
				img.set_pixel(rx, cy, transparent)
			var by := img_h - 1 - cy
			if (
				_sdf_rounded_box(
					Vector2(px, float(by) + 0.5) - body_center, body_half, float(outer_r)
				)
				> 0.0
			):
				img.set_pixel(cx, by, transparent)
			if (
				_sdf_rounded_box(
					Vector2(float(rx) + 0.5, float(by) + 0.5) - body_center,
					body_half,
					float(outer_r)
				)
				> 0.0
			):
				img.set_pixel(rx, by, transparent)

	# --- Clear screen area (transparent where scene shows through) ---
	# Screen rect in image coords: (bz, bz) to (bz+width, bz+height)
	# Clear main strips (excluding inner corner squares)
	img.fill_rect(Rect2i(bz + inner_r, bz, width - 2 * inner_r, height), transparent)
	img.fill_rect(Rect2i(bz, bz + inner_r, inner_r, height - 2 * inner_r), transparent)
	img.fill_rect(
		Rect2i(bz + width - inner_r, bz + inner_r, inner_r, height - 2 * inner_r), transparent
	)

	# Clear inner corner squares (pixel-by-pixel SDF for screen rounded corners)
	var scr_center := Vector2(bz + width * 0.5, bz + height * 0.5)
	var scr_half := Vector2(width * 0.5, height * 0.5)
	for cy in inner_r:
		for cx in inner_r:
			var sx := bz + cx
			var sy := bz + cy
			if (
				_sdf_rounded_box(Vector2(sx + 0.5, sy + 0.5) - scr_center, scr_half, float(inner_r))
				< 0.0
			):
				img.set_pixel(sx, sy, transparent)
			var srx := bz + width - 1 - cx
			if (
				_sdf_rounded_box(
					Vector2(srx + 0.5, sy + 0.5) - scr_center, scr_half, float(inner_r)
				)
				< 0.0
			):
				img.set_pixel(srx, sy, transparent)
			var sby := bz + height - 1 - cy
			if (
				_sdf_rounded_box(
					Vector2(sx + 0.5, sby + 0.5) - scr_center, scr_half, float(inner_r)
				)
				< 0.0
			):
				img.set_pixel(sx, sby, transparent)
			if (
				_sdf_rounded_box(
					Vector2(srx + 0.5, sby + 0.5) - scr_center, scr_half, float(inner_r)
				)
				< 0.0
			):
				img.set_pixel(srx, sby, transparent)

	# --- Screen features (coordinates offset by bezel) ---
	if is_ios:
		# Dynamic island
		var iw: float
		var ih: float
		var icx: float
		var icy: float
		if is_portrait:
			iw = width * 0.27
			ih = height * 0.035
			icx = bz + width * 0.5
			icy = bz + height * 0.012 + ih * 0.5
		else:
			iw = width * 0.035
			ih = height * 0.27
			icx = bz + width * 0.012 + iw * 0.5
			icy = bz + height * 0.5
		var ihs := Vector2(iw, ih) * 0.5
		var ix0 := maxi(0, int(icx - iw * 0.5) - 1)
		var ix1 := mini(img_w, int(icx + iw * 0.5) + 2)
		var iy0 := maxi(0, int(icy - ih * 0.5) - 1)
		var iy1 := mini(img_h, int(icy + ih * 0.5) + 2)
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
			hcx = bz + width * 0.5
			hcy = bz + height * (1.0 - 0.008) - hh * 0.5
		else:
			hw = width * 0.35
			hh = height * 0.005 * 2.0
			hcx = bz + width * 0.5
			hcy = bz + height * (1.0 - 0.016) - hh * 0.5
		var hhs := Vector2(hw, hh) * 0.5
		var hx0 := maxi(0, int(hcx - hw * 0.5) - 1)
		var hx1 := mini(img_w, int(hcx + hw * 0.5) + 2)
		var hy0 := maxi(0, int(hcy - hh * 0.5) - 1)
		var hy1 := mini(img_h, int(hcy + hh * 0.5) + 2)
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
			cam_cx = bz + width * 0.5
			cam_cy = bz + height * 0.022
		else:
			cam_cx = bz + width * 0.022
			cam_cy = bz + height * 0.5
		var cx0 := maxi(0, int(cam_cx - cam_r) - 1)
		var cx1 := mini(img_w, int(cam_cx + cam_r) + 2)
		var cy0 := maxi(0, int(cam_cy - cam_r) - 1)
		var cy1 := mini(img_h, int(cam_cy + cam_r) + 2)
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
		if val == "both":
			return ORIENT_BOTH
	return ORIENT_NONE


func _sync_orient_from_scene() -> void:
	_scene_orient = _get_scene_orientation_id()
	if _scene_orient == ORIENT_PORTRAIT:
		_is_portrait = true
	elif _scene_orient == ORIENT_LANDSCAPE:
		_is_portrait = false
	# ORIENT_NONE / ORIENT_BOTH → keep current _is_portrait
	_update_orient_icon()
	_sync_scene_menus()


func _update_orient_icon() -> void:
	_orient_toggle.icon = _icon_portrait if _is_portrait else _icon_landscape
	if _is_portrait:
		_orient_toggle.tooltip_text = "Preview: Portrait (click to switch to landscape)"
	else:
		_orient_toggle.tooltip_text = "Preview: Landscape (click to switch to portrait)"


func _update_overlay_icon() -> void:
	_overlay_toggle.icon = _icon_overlay_on if _overlay_visible else _icon_overlay_off
	_overlay_toggle.tooltip_text = (
		"Phone frame overlay: ON" if _overlay_visible else "Phone frame overlay: OFF"
	)


# --- Toolbar event handlers ---


func _on_overlay_toggled() -> void:
	_overlay_visible = not _overlay_visible
	_update_overlay_icon()
	EditorInterface.get_editor_settings().set_setting(SETTINGS_OVERLAY_KEY, _overlay_visible)
	update_overlays()


func _on_orient_pressed() -> void:
	if _scene_orient == ORIENT_NONE:
		_error_dialog.dialog_text = "This scene has no orientation set.\nUse the Scene Orientation menu to configure it."
		_error_dialog.popup_centered()
		return

	var want_portrait := not _is_portrait
	var orient := _scene_orient

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
	if _scene_orient == ORIENT_NONE:
		# Save preference but don't apply — no effect until scene has orientation
		EditorInterface.get_editor_settings().set_setting(SETTINGS_DEVICE_KEY, _index)
		return
	_request_apply_with_reload()


func _on_scene_changed(_scene_root: Node) -> void:
	var old_portrait := _is_portrait
	var old_orient := _scene_orient
	_sync_orient_from_scene()
	_apply_current()
	_record_clean_version()
	# Reload when resolution changed: portrait flipped, or transitioning to/from NONE
	var needs_reload := (
		_is_portrait != old_portrait
		or (_scene_orient == ORIENT_NONE) != (old_orient == ORIENT_NONE)
	)
	if needs_reload:
		var root = EditorInterface.get_edited_scene_root()
		if root and not root.scene_file_path.is_empty():
			call_deferred("_reload_current_scene")


func _reload_current_scene() -> void:
	var root = EditorInterface.get_edited_scene_root()
	if root and not root.scene_file_path.is_empty():
		EditorInterface.reload_scene_from_path(root.scene_file_path)


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
		var prev: int = (
			es.get_setting(SETTINGS_DEVICE_KEY) if es.has_setting(SETTINGS_DEVICE_KEY) else 0
		)
		_device_button.select(prev)
		var prev_orient: int = (
			es.get_setting(SETTINGS_ORIENT_KEY) if es.has_setting(SETTINGS_ORIENT_KEY) else 0
		)
		_is_portrait = (prev_orient == 0)
		_update_orient_icon()


# --- Apply ---


func _apply_current() -> void:
	if _scene_orient == ORIENT_NONE:
		# No orientation set — fall back to default resolution, no overlay
		_apply_settings(0, _is_portrait, false)
	else:
		_apply_settings(_device_button.selected, _is_portrait, true)


func _apply_settings(device_index: int, is_portrait: bool, save_device: bool = true) -> void:
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
		_bezel = 0

	if is_instance_valid(_renderer_control):
		_renderer_control.visible = false

	if save_device:
		EditorInterface.get_editor_settings().set_setting(SETTINGS_DEVICE_KEY, device_index)

	update_overlays()


# --- Overlay drawing ---


func _forward_canvas_force_draw_over_viewport(overlay: Control) -> void:
	if not _overlay_visible or _overlay_texture == null:
		return
	var device_index: int = _device_button.selected if is_instance_valid(_device_button) else 0
	if device_index == 0:
		return

	var device = DEVICES[device_index]
	var vp_width: float = device[1] if _is_portrait else device[3]
	var vp_height: float = device[2] if _is_portrait else device[4]

	var bz := Vector2(_bezel, _bezel)
	var xform := EditorInterface.get_editor_viewport_2d().get_final_transform()
	var top_left: Vector2 = xform * (-bz)
	var bottom_right: Vector2 = xform * (Vector2(vp_width, vp_height) + bz)

	overlay.draw_texture_rect(_overlay_texture, Rect2(top_left, bottom_right - top_left), false)
