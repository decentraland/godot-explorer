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
	var http := HTTPRequest.new()
	var timer := Timer.new()
	timer.wait_time = 0.1  # 100ms
	timer.one_shot = false

	# Prepare HTTPRequest
	parent_node.add_child(http)
	http.set_download_file(save_path)

	# Track progress using a timer
	parent_node.add_child(timer)
	var total_size := -1
	timer.timeout.connect(func():
		var downloaded = http.get_downloaded_bytes()

		if total_size <= 0:
			total_size = http.get_body_size()  # This becomes >0 when headers are received

		if total_size > 0:
			var percent = int((float(downloaded) / float(total_size)) * 100.0)
			print("Download progress: %d%% (%d / %d bytes)" % [percent, downloaded, total_size])
		else:
			print("Downloading... %d bytes (total size unknown yet)" % downloaded)
	)

	timer.start()

	# Start request
	var err = http.request(url)
	if err != OK:
		push_error("HTTP request error: %s" % err)
		timer.queue_free()
		http.queue_free()
		return false

	# Wait for completion
	var result = await http.request_completed
	var status_code = result[1]

	timer.stop()
	timer.queue_free()
	http.queue_free()

	if status_code != 200:
		push_error("Download failed. HTTP status: %d" % status_code)
		return false

	print("Download completed successfully.")
	return true



# Check if lib dependency exists; if not, download and extract it.
func _check_lib_dependency():
	var rust_folder_hash: String = get_rust_folder_hash()
	var zip_save = ProjectSettings.globalize_path("res://../libdclgodot.zip")
	var dep_url: String = LIB_DEP_URL % rust_folder_hash
	prints("Downloading lib...", dep_url)
	if await DclBuildEditorPlugin.download_file(self, dep_url, zip_save):
		prints("Done")
		# Defaulting to debug build in editor; adjust as needed.
		var target_dir = ProjectSettings.globalize_path("res://../lib/target/")
		unzip_to_dir(zip_save, target_dir)
		DirAccess.remove_absolute(zip_save)
		
		write_downloaded_rust_version(rust_folder_hash)
		prints("Lib dependency downloaded and extracted.")

func write_downloaded_rust_version(hash: String):
	var file_path = ProjectSettings.globalize_path("res://../lib/target/downloaded_rust_version.txt")
	var file = FileAccess.open(file_path, FileAccess.WRITE)
	if file:
		file.store_string(hash)
		file.close()

func read_downloaded_rust_version() -> String:
	var file_path = ProjectSettings.globalize_path("res://../lib/target/downloaded_rust_version.txt")
	
	if FileAccess.file_exists(file_path):
		var file = FileAccess.open(file_path, FileAccess.READ)
		if file:
			var content = file.get_as_text()
			file.close()
			return content
		else:
			return ""
	else:
		return ""

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
					DclBuildEditorPlugin.unzip_to_dir(zip_save, target_dir)
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
	var exit_code = OS.execute("python", ["../folder_hash.py", "."], output)
	var hash: String = output[0]
	return hash.strip_edges()

func _enter_tree():
	set_xr_mode(false)
	add_export_plugin(BUILD_PLUGIN)
	add_tool_menu_item("Update Godot Rust", self._check_lib_dependency)

func _exit_tree():
	remove_export_plugin(BUILD_PLUGIN)
	remove_tool_menu_item("Update Godot Rust")


static func unzip_to_dir(zip_path: String, extract_to: String) -> bool:
	var reader = ZIPReader.new()
	var err = reader.open(zip_path)
	if err != OK:
		push_error("Failed to open ZIP archive: %s" % zip_path)
		return false

	var dir = DirAccess.open(extract_to)
	if dir == null:
		DirAccess.make_dir_recursive_absolute(extract_to)
		dir = DirAccess.open(extract_to)
		if dir == null:
			push_error("Failed to create target extraction directory: %s" % extract_to)
			return false

	var files = reader.get_files()
	for file_path in files:
		var full_path = extract_to.path_join(file_path)

		if file_path.ends_with("/"):
			DirAccess.make_dir_recursive_absolute(full_path)
			continue

		# Ensure parent directories exist
		DirAccess.make_dir_recursive_absolute(full_path.get_base_dir())

		var buffer = reader.read_file(file_path)
		var file = FileAccess.open(full_path, FileAccess.WRITE)
		if file:
			file.store_buffer(buffer)
			file.close()
		else:
			push_error("Failed to write extracted file: %s" % full_path)

	reader.close()
	return true
