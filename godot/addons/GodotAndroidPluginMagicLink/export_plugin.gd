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
	const PLUGIN_NAME = "GodotAndroidPluginMagicLink"

	func _supports_platform(platform):
		if platform is EditorExportPlatformAndroid:
			return true
		return false

	func _get_android_libraries(_platform, debug):
		if debug:
			return PackedStringArray([PLUGIN_NAME + "/bin/debug/" + PLUGIN_NAME + "-debug.aar"])

		return PackedStringArray([PLUGIN_NAME + "/bin/release/" + PLUGIN_NAME + "-release.aar"])

	func _get_android_dependencies(_platform, _debug):
		var default_dependencies = PackedStringArray(
			[
				"link.magic:magic-android:10.6.0",
				"link.magic:magic-ext-oauth:5.0.1",
				"link.magic:magic-ext-oidc:2.0.4",
				"org.web3j:core:4.8.8-android",
				"org.web3j:geth:4.8.8-android"
			]
		)

		return default_dependencies

	func _get_name():
		return PLUGIN_NAME
