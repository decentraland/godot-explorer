@tool
class_name DclBuildEditorPlugin
extends EditorPlugin

const GODOT_XR_TOOLS_PLUGIN_NAME = "godot-xr-tools"

# New URL constants for dependency downloads
const ANDROID_DEP_URL = "http://example.com/android_dependency.zip"
const LIB_DEP_URL = "https://godot-artifacts.kuruk.net/%s/libdclgodot.zip"

static func set_xr_mode(enabled: bool):
	if EditorInterface.is_plugin_enabled(GODOT_XR_TOOLS_PLUGIN_NAME) != enabled:
		EditorInterface.set_plugin_enabled(GODOT_XR_TOOLS_PLUGIN_NAME, enabled)
		ProjectSettings.set_setting("xr/openxr/enabled", enabled)
		ProjectSettings.set_setting("xr/shaders/enabled", enabled)

# Helper function to download a file from a URL and save it to a specified path.
static func download_file(parent_node: Node, url: String, save_path: String) -> bool:
	var http = HTTPRequest.new()
	parent_node.add_child(http)
	http.set_download_file(save_path)
	var err = http.request(url)
	if err != OK:
		prints("HTTP request error: ", err)
		return false
	await http.request_completed
	http.queue_free()
	return true

# Check if lib dependency exists; if not, download and extract it.
func _check_lib_dependency():
	var rust_folder_hash: String = get_rust_folder_hash()
	prints("Downloading lib...", rust_folder_hash)
	var zip_save = ProjectSettings.globalize_path("res://../libdclgodot.zip")
	var dep_url: String = LIB_DEP_URL % rust_folder_hash
	if await DclBuildEditorPlugin.download_file(self, LIB_DEP_URL, zip_save):
		# Defaulting to debug build in editor; adjust as needed.
		var target_dir = ProjectSettings.globalize_path("res://../lib/target/")
		var output := []
		OS.execute("unzip", [zip_save, "-d", target_dir], output)
		prints("Unzip:", output)
		prints("Lib dependency downloaded and extracted.")

class DclBuildPlugin extends EditorExportPlugin:
	var is_xr_export: bool = false
	var is_android_export: bool = false
	
	const include_only_xr: PackedStringArray = ["res://addons/godot-xr-tools/", "res://addons/godotopenxrvendors/"]
	const include_only_android: PackedStringArray = []
	
	func _export_begin(features, is_debug, path, flags):
		prints("Start export, using XR is ", is_xr_export)

		is_xr_export = features.has("xr")
		is_android_export = features.has("android") and !is_xr_export

		# Check for Android dependency if in Android or XR export.
		if (is_android_export or is_xr_export):
			var android_dir = ProjectSettings.globalize_path("res://android")
			if DirAccess.dir_exists_absolute(android_dir):
				prints("Android dependency missing? Downloading dependency ZIP...")
				var zip_save = ProjectSettings.globalize_path("res://android_dependency.zip")
				var parent_node = EditorInterface.get_base_control()
				if await DclBuildEditorPlugin.download_file(parent_node, ANDROID_DEP_URL, zip_save):
					var build_type = "debug" if is_debug else "release"
					var target_dir = ProjectSettings.globalize_path("res://android/build/libs/" + build_type + "/")
					OS.execute("unzip", [zip_save, "-d", target_dir])
					prints("Android dependency downloaded and extracted.")
		
		if is_xr_export:
			DclBuildEditorPlugin.set_xr_mode(true)
			
		
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

func get_rust_folder_hash() -> String:
	var output = []
	var exit_code = OS.execute("python3", ["../folder_hash.py", "."], output)
	var hash: String = output[0]
	return hash.strip_edges()

func _enter_tree():
	set_xr_mode(false)
	add_export_plugin(BUILD_PLUGIN)
	add_tool_menu_item("Update Godot Rust", self._check_lib_dependency)

func _exit_tree():
	remove_export_plugin(BUILD_PLUGIN)
	remove_tool_menu_item("Update Godot Rust")
