@tool
class_name DclBuildEditorPlugin
extends EditorPlugin

const GODOT_XR_TOOLS_PLUGIN_NAME = "Godot XR Tools"

static func set_xr_mode(enabled: bool):
	if EditorInterface.is_plugin_enabled(GODOT_XR_TOOLS_PLUGIN_NAME) != enabled:
		EditorInterface.set_plugin_enabled(GODOT_XR_TOOLS_PLUGIN_NAME, enabled)
	ProjectSettings.set_setting("xr/openxr/enabled", enabled)
	ProjectSettings.set_setting("xr/shaders/enabled", enabled)

class DclBuildPlugin extends EditorExportPlugin:
	var is_xr_export: bool = false
	
	const xr_paths: PackedStringArray = ["res://addons/godot-xr-tools", "res://addons/godotopenxrvendors/"]
	
	func _export_begin(features, is_debug, path, flags):
		is_xr_export = features.has("xr")

		if is_xr_export:
			DclBuildEditorPlugin.set_xr_mode(true)
			
		prints("Start export XR=", is_xr_export)
		
	func _export_end():
		if is_xr_export:
			DclBuildEditorPlugin.set_xr_mode(false)

	func _export_file(path, type, features):
		if !is_xr_export:
			for xr_path in xr_paths:
				if path.begins_with(xr_path):
					prints("Skip:", path)
					skip()
		
	func _get_name():
		return "DclBuildPlugin"

var BUILD_PLUGIN = DclBuildPlugin.new()

func _enter_tree():
	DclBuildEditorPlugin.set_xr_mode(false)
	add_export_plugin(BUILD_PLUGIN)

func _exit_tree():
	remove_export_plugin(BUILD_PLUGIN)
