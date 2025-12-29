@tool
extends EditorPlugin

# A class member to hold the editor export plugin during its lifecycle.
var export_plugin: AndroidExportPlugin


func _enter_tree():
	# Initialization of the plugin goes here.
	export_plugin = AndroidExportPlugin.new()
	add_export_plugin(export_plugin)


func _exit_tree():
	# Clean-up of the plugin goes here.
	remove_export_plugin(export_plugin)
	export_plugin = null


class AndroidExportPlugin:
	extends EditorExportPlugin
	var _plugin_name = "dcl-godot-android"

	func _supports_platform(platform):
		if platform is EditorExportPlatformAndroid:
			return true
		return false

	func _get_android_libraries(platform, debug):
		if debug:
			return PackedStringArray([_plugin_name + "/bin/debug/" + _plugin_name + "-debug.aar"])
		else:
			return PackedStringArray(
				[_plugin_name + "/bin/release/" + _plugin_name + "-release.aar"]
			)

	func _get_android_dependencies(platform, debug):
		return PackedStringArray(
			[
				"androidx.browser:browser:1.5.0",
				# ExoPlayer dependencies for video playback
				"androidx.media3:media3-exoplayer:1.4.1",
				"androidx.media3:media3-exoplayer-dash:1.4.1",
				"androidx.media3:media3-exoplayer-hls:1.4.1",
				"androidx.media3:media3-ui:1.4.1"
			]
		)

	func _get_name():
		return _plugin_name
