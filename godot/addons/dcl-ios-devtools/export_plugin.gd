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

	## EditorExportPlatform.DebugFlags.DEBUG_FLAG_REMOTE_DEBUG bit, defined locally
	## so we don't depend on the enum being exposed to GDScript.
	const REMOTE_DEBUG_FLAG := 2

	func _get_name():
		return "dcl-ios-devtools"

	func _supports_platform(platform):
		return platform is EditorExportPlatformIOS

	func _export_begin(_features, is_debug, _path, flags):
		# `is_debug` alone is not a dev signal: CI exports the store build with
		# --export-debug too, which shipped the local-network keys and a
		# --scene-inspector pointed at the builder's LAN IP to every App Store user.
		# Require an explicit dev signal on top — the xtask env or an editor deploy
		# with remote debug. CI sets neither.
		if not is_debug:
			return
		if _cmdline_env().is_empty() and (flags & REMOTE_DEBUG_FLAG) == 0:
			return
		var lines: Array = DEV_PLIST_LINES.duplicate()
		lines.append_array(_godot_cmdline_lines(flags))
		add_apple_embedded_platform_plist_content("\n".join(lines))

	## Inject extra launch args into the iOS build via the `godot_cmdline` Info.plist
	## array — Godot's iOS template appends each `<string>` to argv at startup
	## (`add_cmdline` in drivers/apple_embedded/main_utilities.mm), reaching
	## `OS.get_cmdline_args()`. The canonical way to pass `--remote-debug` /
	## `--scene-inspector=ws://…` / `--log-stream=…` to a device build (an iOS app
	## has no real CLI).
	##
	## `DCL_IOS_GODOT_CMDLINE`: "none"/"-" injects nothing, "auto" (or an editor
	## deploy, which sets no env) points at the dev hub on this machine's LAN,
	## anything else is used verbatim.
	func _godot_cmdline_lines(flags: int) -> Array:
		var args: Array = []
		var raw := _cmdline_env()
		if raw.to_lower() == "none" or raw == "-":
			return []
		if raw.is_empty() or raw.to_lower() == "auto":
			var host := _lan_ip()
			if not host.is_empty():
				args.append("--scene-inspector=ws://%s:%d" % [host, HUB_DEVICE_PORT])
		else:
			args.append_array(raw.split(" ", false))
		# Remote debugger: an iOS app has no argv, so the ONLY way the editor's
		# remote debugger can attach is to bake `--remote-debug <uri>` in here. The
		# editor sets REMOTE_DEBUG on a "deploy with remote debug"; the device must
		# reach the Mac at the configured host (loopback won't work over the air).
		if (flags & REMOTE_DEBUG_FLAG) != 0:
			var uri := _remote_debug_uri()
			if not uri.is_empty():
				args.append("--remote-debug")
				args.append(uri)
		if args.is_empty():
			return []
		var lines := ["\t<key>godot_cmdline</key>", "\t<array>"]
		for arg in args:
			lines.append("\t\t<string>%s</string>" % _xml_escape(arg))
		lines.append("\t</array>")
		return lines

	## Build the `tcp://host:port` the iOS app should dial for the editor's remote
	## debugger, from the editor's debug settings. Falls back to this machine's LAN
	## IP whenever the configured host isn't a concrete LAN address a device can
	## reach — loopback (`127.0.0.1`/`localhost`) or a bind-all wildcard
	## (`0.0.0.0`/`*`). Empty when no usable host is found.
	##
	## NOTE: the editor listens on `network/debug/remote_host`. Set it to `0.0.0.0`
	## so the listener binds every interface — that's the only value that serves
	## Android (adb-reverse delivers on 127.0.0.1) and iOS (device dials the LAN IP)
	## at the same time. This helper then rewrites the wildcard to the LAN IP so the
	## phone gets a reachable address instead of `0.0.0.0`.
	func _remote_debug_uri() -> String:
		var es := EditorInterface.get_editor_settings()
		if es == null:
			return ""
		var port := 6007
		if es.has_setting("network/debug/remote_port"):
			port = int(es.get_setting("network/debug/remote_port"))
		var host := ""
		if es.has_setting("network/debug/remote_host"):
			host = str(es.get_setting("network/debug/remote_host")).strip_edges()
		if (
			host.is_empty()
			or host == "127.0.0.1"
			or host == "localhost"
			or host == "0.0.0.0"
			or host == "*"
		):
			host = _lan_ip()
		if host.is_empty():
			return ""
		return "tcp://%s:%d" % [host, port]

	static func _cmdline_env() -> String:
		return OS.get_environment("DCL_IOS_GODOT_CMDLINE").strip_edges()

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
