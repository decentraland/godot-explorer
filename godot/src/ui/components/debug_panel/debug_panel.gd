extends Control

enum SceneLogLevel {
	LOG = 1,
	SCENE_ERROR = 2,
	SYSTEM_ERROR = 3,
}

const ICON_COLUMN_WIDTH = 20

var icon_log: Texture2D = preload("res://src/ui/components/debug_panel/icons/Log.svg")
var icon_error: Texture2D = preload("res://src/ui/components/debug_panel/icons/Error.svg")
var icon_warning: Texture2D = preload("res://src/ui/components/debug_panel/icons/Warning.svg")
var icon_action_copy: Texture2D = preload(
	"res://src/ui/components/debug_panel/icons/ActionCopy.svg"
)

var icon_hidden: Texture2D = preload(
	"res://src/ui/components/debug_panel/icons/GuiVisibilityHidden.svg"
)
var icon_visible: Texture2D = preload(
	"res://src/ui/components/debug_panel/icons/GuiVisibilityVisible.svg"
)

@onready var expression_text_edit = $TabContainer_DebugPanel/Expression/TextEdit
@onready var expression_label = $TabContainer_DebugPanel/Expression/Label

@onready var tree_console: Tree = %Tree_Console
@onready var tab_container_debug_panel: TabContainer = %TabContainer_DebugPanel
@onready var button_show_hide: Button = %Button_ShowHide
@onready var popup_menu: PopupMenu = %PopupMenu
@onready var button_debug_js = %Button_DebugJS
@onready var button_open_source = %Button_OpenSource
@onready var label_debug_info = $"TabContainer_DebugPanel/Misc&Debugger/Label_DebugInfo"


func _ready():
	button_debug_js.disabled = not Global.has_javascript_debugger
	if Global.has_javascript_debugger:
		button_debug_js.text = "Debug JS"

	clear_console()
	if tab_container_debug_panel.visible:
		_on_button_show_hide_pressed()


func clear_console():
	tree_console.set_column_expand(0, false)
	tree_console.set_column_custom_minimum_width(0, ICON_COLUMN_WIDTH)

	tree_console.set_column_expand(1, true)
	tree_console.set_column_clip_content(1, true)

	tree_console.create_item()  # root


func on_console_add(scene_title: String, level: int, _timestamp: float, text: String) -> void:
	var root: TreeItem = tree_console.get_root()
	var item: TreeItem = tree_console.create_item(root)
	item.set_collapsed_recursive(true)

	var color := Color.BLACK
	match level:
		SceneLogLevel.LOG:
			color = Color.LIGHT_GRAY
			item.set_icon(0, icon_log)
		SceneLogLevel.SCENE_ERROR:
			color = Color.YELLOW
			item.set_icon(0, icon_warning)
		SceneLogLevel.SYSTEM_ERROR:
			color = Color.RED
			item.set_icon(0, icon_error)

	# TODO: Implement timestamp correctly
	#item.set_text(0, str(snappedf(timestamp, 0.01)))
	item.set_text_alignment(0, HORIZONTAL_ALIGNMENT_LEFT)

	item.set_custom_color(0, color)
	item.set_custom_color(1, color)

	var message: String = scene_title + "> " + text
	item.set_text(1, message)

	var filter_text = %LineEdit_Filter.text
	var should_hide = message.find(filter_text) == -1 and not filter_text.is_empty()
	item.visible = not should_hide

	# Check size of text...
	var width = get_string_width(message)
	var text_column_width = tree_console.size.x - ICON_COLUMN_WIDTH - 24
	if text_column_width <= width:
		var lines: Array[String] = word_wrap(message)
		var index = 0
		for line in lines:
			index += 1
			var subitem: TreeItem = tree_console.create_item(item)
			subitem.set_text(0, "Line %d" % index)
			subitem.set_text(1, line)

	if not tree_console.has_focus():
		# go to last
		tree_console.scroll_to_item(item)


func get_string_width(message: String) -> int:
	var font: Font = tree_console.get_theme_font("font")
	var font_size = tree_console.get_theme_font_size("font_size")
	return int(font.get_string_size(message, HORIZONTAL_ALIGNMENT_LEFT, -1.0, font_size).x)


func word_wrap(message: String) -> Array[String]:
	var text_column_width = tree_console.size.x - ICON_COLUMN_WIDTH - 24
	var words: PackedStringArray = message.split(" ")
	var line_text_constructor: String = ""
	var lines: Array[String] = []

	for word in words:
		var potential_line = line_text_constructor + word + " "
		var line_width = get_string_width(potential_line)

		if line_width > text_column_width:
			if line_text_constructor != "":
				lines.append(line_text_constructor)
			line_text_constructor = word + " "
		else:
			line_text_constructor = potential_line

	# Add the last line if there's any text left.
	if line_text_constructor.strip_edges() != "":
		lines.append(line_text_constructor)

	return lines


func _on_button_clear_pressed():
	tree_console.clear()
	clear_console()


func _on_line_edit_filter_text_changed(new_text):
	var children = tree_console.get_root().get_children()
	for item: TreeItem in children:
		var should_hide = item.get_text(1).find(new_text) == -1 and not new_text.is_empty()
		item.visible = not should_hide


func _on_button_show_hide_pressed():
	tab_container_debug_panel.visible = not tab_container_debug_panel.visible


func _on_tree_console_item_mouse_selected(tree_position, mouse_button_index):
	if mouse_button_index != MOUSE_BUTTON_RIGHT:
		return

	popup_menu.clear(true)
	popup_menu.reset_size()

	if tree_console.get_selected():
		popup_menu.add_icon_item(icon_action_copy, "Copy ")
		popup_menu.set_position(tree_console.get_screen_position() + tree_position)
		popup_menu.popup()


func _on_popup_menu_index_pressed(index):
	# Only has copy
	if index == 0:
		var text = tree_console.get_selected().get_text(1)
		DisplayServer.clipboard_set(text)


func _on_button_copy_pressed():
	var text: String = ""
	var children = tree_console.get_root().get_children()
	for item: TreeItem in children:
		if item.visible:
			text += item.get_text(1) + "\n"
	DisplayServer.clipboard_set(text)


func _on_text_edit_text_changed():
	var expression = Expression.new()
	var err = expression.parse(expression_text_edit.text, ["Global"])

	if err != OK:
		expression_label.text = "Parse failed: " + expression.get_error_text()
		return

	var result = expression.execute([Global], self)
	if expression.has_execute_failed():
		expression_label.text = "Execution failed: " + expression.get_error_text()
		return

	expression_label.text = "Ok: " + str(result)


func _on_tab_container_debug_panel_visibility_changed():
	if is_instance_valid(tab_container_debug_panel):
		if tab_container_debug_panel.visible:
			button_show_hide.icon = icon_visible
		else:
			button_show_hide.icon = icon_hidden


func _on_button_show_network_pressed():
	Global.open_network_inspector_ui()


func _on_button_debug_js_pressed():
	var current_scene = Global.scene_fetcher.get_current_scene_data()
	if current_scene == null:
		printerr("there is no current scene")
		return

	Global.scene_fetcher.set_debugging_js_scene_id(current_scene.id)
	Global.scene_fetcher.reload_scene(current_scene.id)
	label_debug_info.show()

	print("debugging js file ", current_scene.scene_entity_definition.get_main_js_hash())


func _on_button_open_source_pressed():
	var current_scene = Global.scene_fetcher.get_current_scene_data()
	if current_scene == null:
		printerr("there is no current scene")
		return

	var main_js_hash = current_scene.scene_entity_definition.get_main_js_hash()
	if main_js_hash.is_empty():
		printerr("no main js file found for current scene")
		return

	var js_file_path = "user://content/" + main_js_hash
	var absolute_path = ProjectSettings.globalize_path(js_file_path)

	if not FileAccess.file_exists(js_file_path):
		printerr("js file does not exist: ", absolute_path)
		return

	DisplayServer.clipboard_set(absolute_path)
	print("Opening scene source code: ", absolute_path)
	print("Absolute path copied to clipboard")
	OS.shell_open(absolute_path)
