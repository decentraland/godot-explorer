@tool
extends EditorPlugin
## DEV-ONLY iOS export plugin.
##
## Injects the Info.plist keys needed by the dev localhost relay (`DclDevRelay`)
## and developer log streaming (`--log-stream`) — but ONLY into debug builds.
## Release / TestFlight / App Store exports stay clean, keeping the App-Review
## scrutinized local-network keys out of production.
##
## Mirrors `addons/dcl-godot-android/export_plugin.gd`: an EditorExportPlugin that
## contributes platform config on demand at export time, instead of hardcoding the
## keys permanently in `export_presets.cfg`.

var export_plugin: DclIosDevExportPlugin


func _enter_tree():
	export_plugin = DclIosDevExportPlugin.new()
	add_export_plugin(export_plugin)


func _exit_tree():
	remove_export_plugin(export_plugin)
	export_plugin = null


class DclIosDevExportPlugin:
	extends EditorExportPlugin
	# Inserted verbatim before the closing </dict> of the generated Info.plist.
	# Each line carries its own leading tab(s) to match the surrounding plist.
	const DEV_PLIST_LINES := [
		"\t<key>NSLocalNetworkUsageDescription</key>",
		(
			"\t<string>Dev only: developer log streaming (--log-stream) and the"
			+ " localhost dev relay connect to a debug server on your local"
			+ " network.</string>"
		),
		"\t<key>NSAppTransportSecurity</key>",
		"\t<dict>",
		"\t\t<key>NSAllowsLocalNetworking</key>",
		"\t\t<true/>",
		"\t</dict>",
	]

	func _get_name():
		return "dcl-ios-devtools"

	func _supports_platform(platform):
		return platform is EditorExportPlatformIOS

	func _export_begin(_features, is_debug, _path, _flags):
		# Only debug builds declare local networking. The relay and log-stream are
		# themselves gated to OS.is_debug_build(), so release builds neither use
		# nor declare it — which is what keeps App Review happy.
		if is_debug:
			add_apple_embedded_platform_plist_content("\n".join(DEV_PLIST_LINES))
