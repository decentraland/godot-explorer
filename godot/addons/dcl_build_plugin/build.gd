@tool
class_name DclBuildEditorPlugin
extends EditorPlugin

class DclBuildPlugin:
	extends EditorExportPlugin
	var is_android_export: bool = false

	const include_only_android: PackedStringArray = []
	const include_only_editor: PackedStringArray = ["res://addons/dcl_mobile_preview/"]

	func _export_begin(features, is_debug, path, flags):
		is_android_export = features.has("android")

		if is_android_export:
			var android_dir = ProjectSettings.globalize_path("res://android")
			if DirAccess.dir_exists_absolute(android_dir):
				var build_type = "debug" if is_debug else "release"
				var target_dir = ProjectSettings.globalize_path(
					"res://android/build/libs/" + build_type + "/arm64-v8a/deps/"
				)

				# Check if directory exists and contains at least one file
				var needs_extraction = true
				if DirAccess.dir_exists_absolute(target_dir):
					var dir = DirAccess.open(target_dir)
					if dir:
						dir.list_dir_begin()
						var file_name = dir.get_next()
						if file_name != "":
							needs_extraction = false
							prints("Android dependencies found in:", target_dir)
						dir.list_dir_end()

				if needs_extraction:
					DirAccess.make_dir_recursive_absolute(target_dir)
					prints("Android dependencies missing. Unzipping dependency ZIP...")
					var zip_save = ProjectSettings.globalize_path(
						"res://../.bin/android_dependencies.zip"
					)
					if FileAccess.file_exists(zip_save):
						UnZip.unzip_to_dir(zip_save, target_dir)
						prints("Android dependencies extracted to:", target_dir)
					else:
						push_error("Android dependencies ZIP not found at: " + zip_save)
						push_error("Please run: cargo run -- install --targets android")

	func check_excluded(path, paths_excluded):
		for path_excluded in paths_excluded:
			if path.begins_with(path_excluded):
				prints("Skip:", path)
				skip()

	func _export_file(path, type, features):
		if !is_android_export:
			check_excluded(path, include_only_android)

		check_excluded(path, include_only_editor)

	func _get_name():
		return "DclBuildPlugin"


var BUILD_PLUGIN = DclBuildPlugin.new()


func _enter_tree():
	prints("Dcl Build Plugin enabled")
	add_export_plugin(BUILD_PLUGIN)


func _exit_tree():
	remove_export_plugin(BUILD_PLUGIN)
