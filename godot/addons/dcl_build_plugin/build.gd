@tool
class_name DclBuildEditorPlugin
extends EditorPlugin

const GODOT_XR_TOOLS_PLUGIN_NAME = "godot-xr-tools"

static func set_xr_mode(enabled: bool):
	if EditorInterface.is_plugin_enabled(GODOT_XR_TOOLS_PLUGIN_NAME) != enabled:
		EditorInterface.set_plugin_enabled(GODOT_XR_TOOLS_PLUGIN_NAME, enabled)
	ProjectSettings.set_setting("xr/openxr/enabled", enabled)
	ProjectSettings.set_setting("xr/shaders/enabled", enabled)

class DclBuildPlugin extends EditorExportPlugin:
	var is_xr_export: bool = false
	var is_android_export: bool = false
	
	const include_only_xr: PackedStringArray = ["res://addons/godot-xr-tools/", "res://addons/godotopenxrvendors/"]
	const include_only_android: PackedStringArray = ["res://addons/GodotAndroidPluginMagicLink/"]
	
	func _export_begin(features, is_debug, path, flags):
		is_xr_export = features.has("xr")
		is_android_export = features.has("android") and !is_xr_export

		if is_xr_export:
			DclBuildEditorPlugin.set_xr_mode(true)
			
		prints("Start export, using XR is ", is_xr_export)
		
	func _export_end():
		if is_xr_export:
			DclBuildEditorPlugin.set_xr_mode(false)

	func check_excluded(path, paths_excluded):
		for path_excluded in paths_excluded:
				if path.begins_with(path_excluded):
					prints("Skip:", path)
					skip()

	func _export_file(path, type, features):
		if !is_xr_export:
			check_excluded(path, include_only_xr)
			
		if !is_android_export:
			check_excluded(path, include_only_android)
		
	func _get_name():
		return "DclBuildPlugin"

var BUILD_PLUGIN = DclBuildPlugin.new()

func _enter_tree():
	set_xr_mode(false)
	add_export_plugin(BUILD_PLUGIN)

func _exit_tree():
	remove_export_plugin(BUILD_PLUGIN)
