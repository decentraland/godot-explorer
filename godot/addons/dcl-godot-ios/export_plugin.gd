@tool
extends EditorPlugin

var export_plugin: IosExportPlugin


func _enter_tree():
	export_plugin = IosExportPlugin.new()
	add_export_plugin(export_plugin)


func _exit_tree():
	remove_export_plugin(export_plugin)
	export_plugin = null


class IosExportPlugin:
	extends EditorExportPlugin
	# The iOS counterpart of godot/google-services.json. Optional: without it the build still
	# exports and the plugin reports Firebase as unavailable.
	const FIREBASE_PLIST := "res://ios/GoogleService-Info.plist"

	func _get_name():
		return "dcl-godot-ios"

	func _supports_platform(platform):
		return platform is EditorExportPlatformIOS

	func _export_begin(_features, _is_debug, _path, _flags):
		if FileAccess.file_exists(FIREBASE_PLIST):
			add_ios_bundle_file(FIREBASE_PLIST)
