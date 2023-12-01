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


func _ready():
	%Tree_Console.set_column_expand(0, false)
	%Tree_Console.set_column_custom_minimum_width(0, ICON_COLUMN_WIDTH)

	%Tree_Console.set_column_expand(1, true)
	%Tree_Console.set_column_clip_content(1, true)

	%Tree_Console.create_item()  # root


func on_console_add(scene_title: String, level: int, _timestamp: float, text: String) -> void:
	var root: TreeItem = %Tree_Console.get_root()
	var item: TreeItem = %Tree_Console.create_item(root)
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
	var hide = message.find(filter_text) == -1 and not filter_text.is_empty()
	item.visible = not hide

	# Check size of text...
	var width = get_string_width(message)
	var text_column_width = %Tree_Console.size.x - ICON_COLUMN_WIDTH - 24
	if text_column_width <= width:
		var lines: Array[String] = word_wrap(message)
		var index = 0
		for line in lines:
			index += 1
			var subitem: TreeItem = %Tree_Console.create_item(item)
			subitem.set_text(0, "Line %d" % index)
			subitem.set_text(1, line)


func get_string_width(message: String) -> int:
	var font: Font = %Tree_Console.get_theme_font("font")
	var font_size = %Tree_Console.get_theme_font_size("font_size")
	return font.get_string_size(message, 0, -1, font_size).x


func word_wrap(message: String) -> Array[String]:
	var text_column_width = %Tree_Console.size.x - ICON_COLUMN_WIDTH - 24
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
	%Tree_Console.clear()


func _on_line_edit_filter_text_changed(new_text):
	var children = %Tree_Console.get_root().get_children()
	for item: TreeItem in children:
		var hide = item.get_text(1).find(new_text) == -1 and not new_text.is_empty()
		item.visible = not hide


func _on_button_show_hide_pressed():
	%TabContainer_DebugPanel.visible = not %TabContainer_DebugPanel.visible
	if %TabContainer_DebugPanel.visible:
		%Button_ShowHide.icon = icon_visible
	else:
		%Button_ShowHide.icon = icon_hidden


func _on_tree_console_item_mouse_selected(position, mouse_button_index):
	if mouse_button_index != MOUSE_BUTTON_RIGHT:
		return

	%PopupMenu.clear(true)
	%PopupMenu.reset_size()

	if %Tree_Console.get_selected():
		%PopupMenu.add_icon_item(icon_action_copy, "Copy ")
		%PopupMenu.set_position(%Tree_Console.get_screen_position() + position)
		%PopupMenu.popup()


func _on_popup_menu_index_pressed(index):
	# Only has copy
	if index == 0:
		var text = %Tree_Console.get_selected().get_text(1)
		DisplayServer.clipboard_set(text)
