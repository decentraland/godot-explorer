extends Node
## DEV-ONLY: the iOS equivalent of Android's `adb reverse` for localhost.
##
## Binds 127.0.0.1:<port> inside the app (native `DclDevRelay` Swift class) and
## forwards every connection to a dev server on your Mac. This lets a webview /
## OAuth callback that hard-codes `http://localhost:<port>` reach your machine.
## iOS never routes `localhost` through a proxy or VPN, so an in-app relay is the
## only way to do this on a physical device without a 3rd-party SSH app.
##
## Active ONLY on a native-iOS debug build. It's a no-op everywhere else: off
## iOS the Swift GDExtension isn't loaded, so `ClassDB.class_exists` is false and
## we bail. It also bails on release builds.
##
## Mac side: run the dev server on the LAN (`vite --host`, not just loopback) and
## accept the iOS "local network" permission prompt once.
##
## Target defaults to the constants below; override at runtime with the user arg:
##     cargo run -- run --target ios -- --dev-relay=my-mac.local:5173

const DEFAULT_HOST := "Leandros-MacBook-Pro.local"
const DEFAULT_PORT := 5173

# Held so the RefCounted relay (and its listener) isn't freed.
var _relay = null


func _ready() -> void:
	if not OS.is_debug_build():
		return
	if not ClassDB.class_exists(&"DclDevRelay"):
		# Swift GDExtension not loaded (i.e. not iOS) — nothing to relay.
		return

	var host := DEFAULT_HOST
	var port := DEFAULT_PORT
	var override := _cli_override()
	if not override.is_empty():
		host = override[0]
		port = override[1]
	if host.is_empty():
		return

	_relay = ClassDB.instantiate(&"DclDevRelay")
	var ok: bool = _relay.start(host, port, port)
	print("[dev-relay] start %s:%d -> 127.0.0.1:%d ok=%s" % [host, port, port, ok])


## Parse `--dev-relay=host[:port]` from the user args (after `--`).
## Returns [host, port] or [] when absent.
func _cli_override() -> Array:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--dev-relay="):
			var value := arg.substr("--dev-relay=".length())
			var parts := value.rsplit(":", false, 1)
			if parts.size() == 2 and parts[1].is_valid_int():
				return [parts[0], parts[1].to_int()]
			return [value, DEFAULT_PORT]
	return []
