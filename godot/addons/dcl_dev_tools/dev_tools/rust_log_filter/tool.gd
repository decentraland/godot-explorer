extends "res://addons/dcl_dev_tools/dev_tools/dcl_dev_tool.gd"

const LOG_LEVELS: PackedStringArray = ["error", "warn", "info", "debug", "trace"]
const LOG_LABELS: PackedStringArray = ["E", "W", "I", "D", "T"]
const LOG_COLORS_ACTIVE: Array[Color] = [
	Color(0.9, 0.2, 0.2),  # ERROR  - red
	Color(0.9, 0.75, 0.1),  # WARN   - yellow
	Color(0.3, 0.55, 0.9),  # INFO   - blue
	Color(0.3, 0.8, 0.3),  # DEBUG  - green
	Color(0.6, 0.6, 0.6),  # TRACE  - gray
]
const LOG_COLOR_INACTIVE := Color(0.3, 0.3, 0.3)
const ICON_SIZE := 16

const SETTINGS_DEFAULT_LEVEL := "rust_log_filter/default_level"
const SETTINGS_MODULE_OVERRIDES := "rust_log_filter/module_overrides"
const CRATE_NAME := "dclgodot"

var dialog: AcceptDialog
var tree: Tree
var option_default: OptionButton
var label_preview: Label

# { "dclgodot::comms": 3 } where 3 = index into LOG_LEVELS (debug)
var module_levels: Dictionary = {}

# Cached icon textures: [level_index][is_active] -> ImageTexture
var _icon_cache: Array[Array] = []


func populate_menu(menu: PopupMenu, id: int):
	menu.add_item("Rust Log Filter...", id)


func execute():
	_create_dialog()
	dialog.popup_centered()


func cleanup():
	if dialog and is_instance_valid(dialog):
		dialog.queue_free()
		dialog = null


# ---------------------------------------------------------------------------
# Dialog creation
# ---------------------------------------------------------------------------


func _create_dialog():
	if dialog:
		return

	_build_icon_cache()
	_load_settings()

	dialog = AcceptDialog.new()
	dialog.title = "Rust Log Filter"
	dialog.size = Vector2(700, 500)
	dialog.unresizable = false
	dialog.get_ok_button().hide()

	var vbox := VBoxContainer.new()

	# --- Toolbar ---
	var toolbar := HBoxContainer.new()
	toolbar.add_theme_constant_override("separation", 6)

	var lbl_default := Label.new()
	lbl_default.text = "Default Level:"
	toolbar.add_child(lbl_default)

	option_default = OptionButton.new()
	for level_name in ["ERROR", "WARN", "INFO", "DEBUG", "TRACE"]:
		option_default.add_item(level_name)
	option_default.selected = 1
	option_default.item_selected.connect(_on_default_level_changed)
	toolbar.add_child(option_default)

	var sep1 := VSeparator.new()
	toolbar.add_child(sep1)

	var btn_apply := Button.new()
	btn_apply.text = "Apply to Run Args"
	btn_apply.pressed.connect(_on_apply_pressed)
	toolbar.add_child(btn_apply)

	var btn_reset := Button.new()
	btn_reset.text = "Reset"
	btn_reset.pressed.connect(_on_reset_pressed)
	toolbar.add_child(btn_reset)

	# Spacer pushes copy buttons to the right
	var spacer := Control.new()
	spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	toolbar.add_child(spacer)

	var btn_copy_arg := Button.new()
	btn_copy_arg.text = "Copy --rust-log"
	btn_copy_arg.pressed.connect(_on_copy_rust_log_pressed)
	toolbar.add_child(btn_copy_arg)

	var btn_copy_env := Button.new()
	btn_copy_env.text = "Copy RUST_LOG"
	btn_copy_env.pressed.connect(_on_copy_env_var_pressed)
	toolbar.add_child(btn_copy_env)

	vbox.add_child(toolbar)

	# --- Separator ---
	vbox.add_child(HSeparator.new())

	# --- Filter preview ---
	label_preview = Label.new()
	label_preview.text = "--rust-log=warn"
	label_preview.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
	vbox.add_child(label_preview)

	# --- Separator ---
	vbox.add_child(HSeparator.new())

	# --- Tree ---
	tree = Tree.new()
	tree.columns = 6
	tree.column_titles_visible = true
	tree.hide_root = true
	tree.size_flags_vertical = Control.SIZE_EXPAND_FILL
	tree.custom_minimum_size = Vector2(0, 300)
	_setup_tree_columns()
	tree.button_clicked.connect(_on_tree_button_clicked)
	vbox.add_child(tree)

	dialog.add_child(vbox)
	plugin.get_editor_interface().get_base_control().add_child(dialog)

	# Now that settings are loaded, apply them
	_apply_loaded_settings()
	_scan_and_populate()
	_update_filter_preview()


func _apply_loaded_settings():
	var es := plugin.get_editor_interface().get_editor_settings()
	var default_idx: int = (
		es.get_setting(SETTINGS_DEFAULT_LEVEL) if es.has_setting(SETTINGS_DEFAULT_LEVEL) else 1
	)
	option_default.selected = clampi(default_idx, 0, 4)


# ---------------------------------------------------------------------------
# Icon cache
# ---------------------------------------------------------------------------


func _build_icon_cache() -> void:
	_icon_cache.clear()
	for level_idx in LOG_LEVELS.size():
		var pair: Array = []
		pair.resize(2)
		pair[0] = _make_icon(LOG_COLOR_INACTIVE)
		pair[1] = _make_icon(LOG_COLORS_ACTIVE[level_idx])
		_icon_cache.append(pair)


func _make_icon(color: Color) -> ImageTexture:
	var img := Image.create(ICON_SIZE, ICON_SIZE, false, Image.FORMAT_RGBA8)
	img.fill(color)
	return ImageTexture.create_from_image(img)


# ---------------------------------------------------------------------------
# Tree setup & scanning
# ---------------------------------------------------------------------------


func _setup_tree_columns() -> void:
	tree.set_column_title(0, "Module")
	tree.set_column_expand(0, true)
	tree.set_column_clip_content(0, true)
	for i in range(5):
		var col := i + 1
		tree.set_column_title(col, LOG_LABELS[i])
		tree.set_column_expand(col, false)
		tree.set_column_custom_minimum_width(col, 36)


func _scan_and_populate() -> void:
	tree.clear()
	var root := tree.create_item()

	var lib_src := ProjectSettings.globalize_path("res://../lib/src/")
	var dir := DirAccess.open(lib_src)
	if dir == null:
		var fallback := tree.create_item(root)
		fallback.set_text(0, "(Could not open lib/src/ — path: %s)" % lib_src)
		return

	var entries: Array[String] = []
	dir.list_dir_begin()
	var entry := dir.get_next()
	while entry != "":
		if not entry.begins_with("."):
			entries.append(entry)
		entry = dir.get_next()
	dir.list_dir_end()
	entries.sort()

	for e in entries:
		var full_path := lib_src.path_join(e)
		if DirAccess.dir_exists_absolute(full_path):
			_add_module_branch(root, CRATE_NAME + "::" + e, e, full_path)
		elif e.ends_with(".rs") and e != "lib.rs":
			var mod_name := e.get_basename()
			var mod_path := CRATE_NAME + "::" + mod_name
			_add_module_leaf(root, mod_path, mod_name)


func _add_module_branch(
	parent: TreeItem, mod_path: String, display_name: String, dir_path: String
) -> void:
	var item := tree.create_item(parent)
	item.set_text(0, display_name)
	item.set_metadata(0, mod_path)
	item.collapsed = true
	_add_level_buttons(item, mod_path)

	var dir := DirAccess.open(dir_path)
	if dir == null:
		return

	var sub_entries: Array[String] = []
	dir.list_dir_begin()
	var entry := dir.get_next()
	while entry != "":
		if not entry.begins_with("."):
			sub_entries.append(entry)
		entry = dir.get_next()
	dir.list_dir_end()
	sub_entries.sort()

	for e in sub_entries:
		var full_path := dir_path.path_join(e)
		if DirAccess.dir_exists_absolute(full_path):
			_add_module_branch(item, mod_path + "::" + e, e, full_path)
		elif e.ends_with(".rs") and e != "mod.rs" and e != "lib.rs":
			var mod_name := e.get_basename()
			_add_module_leaf(item, mod_path + "::" + mod_name, mod_name)


func _add_module_leaf(parent: TreeItem, mod_path: String, display_name: String) -> void:
	var item := tree.create_item(parent)
	item.set_text(0, display_name)
	item.set_metadata(0, mod_path)
	_add_level_buttons(item, mod_path)


func _add_level_buttons(item: TreeItem, mod_path: String) -> void:
	var active_level: int = module_levels.get(mod_path, -1)
	for level_idx in 5:
		var col := level_idx + 1
		var is_active := active_level == level_idx
		var icon: Texture2D = _icon_cache[level_idx][1] if is_active else _icon_cache[level_idx][0]
		item.add_button(col, icon, level_idx, false, LOG_LEVELS[level_idx])


func _refresh_buttons(item: TreeItem) -> void:
	var mod_path: String = item.get_metadata(0)
	if mod_path == null or mod_path.is_empty():
		return
	var active_level: int = module_levels.get(mod_path, -1)
	for level_idx in 5:
		var col := level_idx + 1
		var is_active := active_level == level_idx
		var icon: Texture2D = _icon_cache[level_idx][1] if is_active else _icon_cache[level_idx][0]
		item.set_button(col, 0, icon)


func _refresh_all_tree_buttons(item: TreeItem) -> void:
	if item == null:
		return
	if item.get_metadata(0) != null and not str(item.get_metadata(0)).is_empty():
		_refresh_buttons(item)
	var child := item.get_first_child()
	while child != null:
		_refresh_all_tree_buttons(child)
		child = child.get_next()


# ---------------------------------------------------------------------------
# Filter string generation
# ---------------------------------------------------------------------------


func _get_default_level() -> String:
	return LOG_LEVELS[option_default.selected]


func _build_filter_string() -> String:
	var default_level := _get_default_level()

	var parts: PackedStringArray = []
	var sorted_keys: Array = module_levels.keys()
	sorted_keys.sort()

	for mod_path in sorted_keys:
		var level_idx: int = module_levels[mod_path]
		var level_str: String = LOG_LEVELS[level_idx]

		var inherited := _get_inherited_level(mod_path)
		if level_str != inherited:
			parts.append("%s=%s" % [mod_path, level_str])

	parts.append(default_level)
	return ",".join(parts)


func _get_inherited_level(mod_path: String) -> String:
	var parts := mod_path.split("::")
	for i in range(parts.size() - 1, 0, -1):
		var ancestor := "::".join(parts.slice(0, i))
		if ancestor in module_levels:
			return LOG_LEVELS[module_levels[ancestor]]
	return _get_default_level()


func _update_filter_preview() -> void:
	var filter := _build_filter_string()
	label_preview.text = "--rust-log=" + filter


# ---------------------------------------------------------------------------
# Run args manipulation
# ---------------------------------------------------------------------------


func _inject_run_args(filter: String) -> void:
	var current_args: String = ProjectSettings.get_setting("editor/run/main_run_args", "")
	var args_array := current_args.split(" ")
	var new_args: PackedStringArray = []
	var skip_next := false

	for i in args_array.size():
		if skip_next:
			skip_next = false
			continue
		var arg: String = args_array[i]
		if arg.begins_with("--rust-log="):
			continue
		if arg == "--rust-log":
			skip_next = true
			continue
		if arg != "":
			new_args.append(arg)

	new_args.append("--rust-log=" + filter)
	ProjectSettings.set_setting("editor/run/main_run_args", " ".join(new_args))


func _remove_run_args() -> void:
	var current_args: String = ProjectSettings.get_setting("editor/run/main_run_args", "")
	var args_array := current_args.split(" ")
	var new_args: PackedStringArray = []
	var skip_next := false

	for i in args_array.size():
		if skip_next:
			skip_next = false
			continue
		var arg: String = args_array[i]
		if arg.begins_with("--rust-log="):
			continue
		if arg == "--rust-log":
			skip_next = true
			continue
		if arg != "":
			new_args.append(arg)

	ProjectSettings.set_setting("editor/run/main_run_args", " ".join(new_args))


# ---------------------------------------------------------------------------
# Persistence (EditorSettings)
# ---------------------------------------------------------------------------


func _load_settings() -> void:
	var es := plugin.get_editor_interface().get_editor_settings()

	if es.has_setting(SETTINGS_MODULE_OVERRIDES):
		var json_str: String = es.get_setting(SETTINGS_MODULE_OVERRIDES)
		var parsed = JSON.parse_string(json_str)
		if parsed is Dictionary:
			module_levels = {}
			for key in parsed:
				module_levels[key] = int(parsed[key])


func _save_settings() -> void:
	var es := plugin.get_editor_interface().get_editor_settings()
	es.set_setting(SETTINGS_DEFAULT_LEVEL, option_default.selected)
	es.set_setting(SETTINGS_MODULE_OVERRIDES, JSON.stringify(module_levels))


# ---------------------------------------------------------------------------
# Signal handlers
# ---------------------------------------------------------------------------


func _on_default_level_changed(_index: int) -> void:
	_update_filter_preview()
	_save_settings()


func _on_tree_button_clicked(item: TreeItem, column: int, _id: int, _mouse_button: int) -> void:
	var mod_path: String = item.get_metadata(0)
	if mod_path == null or mod_path.is_empty():
		return

	var level_idx := column - 1
	if level_idx < 0 or level_idx > 4:
		return

	var current: int = module_levels.get(mod_path, -1)
	if current == level_idx:
		module_levels.erase(mod_path)
	else:
		module_levels[mod_path] = level_idx

	_refresh_buttons(item)
	_update_filter_preview()
	_save_settings()


func _on_apply_pressed() -> void:
	var filter := _build_filter_string()
	_inject_run_args(filter)


func _on_reset_pressed() -> void:
	module_levels.clear()
	option_default.selected = 1  # WARN
	_remove_run_args()
	_save_settings()
	_refresh_all_tree_buttons(tree.get_root())
	_update_filter_preview()


func _on_copy_rust_log_pressed() -> void:
	var filter := _build_filter_string()
	DisplayServer.clipboard_set("--rust-log=" + filter)


func _on_copy_env_var_pressed() -> void:
	var filter := _build_filter_string()
	DisplayServer.clipboard_set("RUST_LOG=" + filter)
