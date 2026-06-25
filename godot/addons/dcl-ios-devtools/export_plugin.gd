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

	## Default device-facing port of the desktop `cargo run -- debug-hub`.
	const HUB_DEVICE_PORT := 9231

	func _get_name():
		return "dcl-ios-devtools"

	func _supports_platform(platform):
		return platform is EditorExportPlatformIOS

	func _export_begin(_features, is_debug, _path, _flags):
		# Only debug builds declare local networking / inject launch args. The relay
		# and log-stream are themselves gated to OS.is_debug_build(), so release
		# builds neither use nor declare it — which is what keeps App Review happy.
		if not is_debug:
			return
		var lines: Array = DEV_PLIST_LINES.duplicate()
		lines.append_array(_godot_cmdline_lines())
		add_apple_embedded_platform_plist_content("\n".join(lines))

	## Inject extra launch args into the iOS build via the `godot_cmdline` Info.plist
	## array — Godot's iOS template appends each `<string>` to argv at startup
	## (`add_cmdline` in drivers/apple_embedded/main_utilities.mm), reaching
	## `OS.get_cmdline_args()`. The canonical way to pass `--remote-debug` /
	## `--scene-inspector=ws://…` / `--log-stream=…` to a device build (an iOS app
	## has no real CLI).
	##
	## Precedence: an explicit `DCL_IOS_GODOT_CMDLINE` env (set by the xtask for full
	## control) wins. Otherwise the build is auto-pointed at the dev hub on this
	## machine's LAN, so the app "phones home" however it was launched — including a
	## plain Godot-editor deploy, which never sets the env. Harmless when no hub is
	## up: the scene-inspector client just retries with backoff and (being
	## connection-gated) captures nothing until a consumer subscribes.
	func _godot_cmdline_lines() -> Array:
		var raw := OS.get_environment("DCL_IOS_GODOT_CMDLINE").strip_edges()
		# Sentinel: an explicit "none"/"-" injects nothing (the xtask sets this for
		# `run --target ios --no-hub` → plain `--console` log streaming, no hub).
		if raw.to_lower() == "none" or raw == "-":
			return []
		if raw.is_empty():
			var host := _lan_ip()
			if host.is_empty():
				return []
			raw = "--scene-inspector=ws://%s:%d" % [host, HUB_DEVICE_PORT]
		var lines := ["\t<key>godot_cmdline</key>", "\t<array>"]
		for arg in raw.split(" ", false):
			lines.append("\t\t<string>%s</string>" % _xml_escape(arg))
		lines.append("\t</array>")
		return lines

	## This machine's private-LAN IPv4 (the address a device on the same network
	## can reach). Skips loopback, link-local and IPv6. Empty if none found.
	static func _lan_ip() -> String:
		for addr in IP.get_local_addresses():
			if addr.contains(":") or not addr.is_valid_ip_address():
				continue
			if addr.begins_with("192.168.") or addr.begins_with("10."):
				return addr
			if addr.begins_with("172."):
				var second: int = addr.split(".")[1].to_int()
				if second >= 16 and second <= 31:
					return addr
		return ""

	static func _xml_escape(s: String) -> String:
		return s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")
