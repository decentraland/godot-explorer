extends Node
## Frame spike logger for hiccup investigation (issue #2593).
##
## Enabled via --spike-log / ?spike-log=true. Prints one line per frame slower
## than `threshold_ms` with subsystem context, so spikes can be attributed
## after the fact from logcat / Godot file logs (file logging is enabled in
## project.godot, lines land in user://logs/godot.log too).

@export var threshold_ms: float = 50.0
@export var breakdown_interval_s: float = 10.0

var _start_ticks_ms: int = 0
var _breakdown_enabled := false
var _breakdown_elapsed := 0.0


func _ready() -> void:
	_start_ticks_ms = Time.get_ticks_msec()
	print("[SPIKE-LOG] enabled, threshold=%.0fms" % threshold_ms)
	if Global.cli.crdt_breakdown:
		_breakdown_enabled = true
		DclGlobal.crdt_breakdown_begin()
		print("[CRDT-BREAKDOWN] enabled, draining every %.0fs" % breakdown_interval_s)


func _process(delta: float) -> void:
	if _breakdown_enabled:
		_breakdown_elapsed += delta
		if _breakdown_elapsed >= breakdown_interval_s:
			_breakdown_elapsed = 0.0
			print("[CRDT-BREAKDOWN] ", DclGlobal.crdt_breakdown_drain())
			var ctx: Dictionary = DclGlobal.get_spike_context()
			print(
				(
					"[TRANSFORM-PUTS] tween=%d avatar=%d"
					% [ctx["transform_puts_tween"], ctx["transform_puts_avatar"]]
				)
			)
			# drain disables recording; re-arm for the next window
			DclGlobal.crdt_breakdown_begin()

	var frame_ms := delta * 1000.0
	if frame_ms < threshold_ms:
		return

	var ctx: Dictionary = DclGlobal.get_spike_context()
	var t := (Time.get_ticks_msec() - _start_ticks_ms) / 1000.0
	var avatar_count := 0
	if is_instance_valid(Global.avatars):
		avatar_count = Global.avatars.get_child_count()

	print(
		(
			"[SPIKE] t=%.1fs frame=%.0fms gltf_groups=%d gltf_downloading=%d gltf_dl_queue=%d gltf_load_queue=%d impostor_queue=%d impostor_active=%s avatars=%d fps_cap=%d crdt_send_ops=%d crdt_recv_ops=%d crdt_recv_bytes=%d v8_used_mb=%d"
			% [
				t,
				frame_ms,
				GltfLoadingCoordinator._groups.size(),
				GltfLoadingCoordinator._downloading_count,
				GltfLoadingCoordinator._download_queue.size(),
				GltfLoadingCoordinator._load_queue.size(),
				ImpostorCapturer._queue.size(),
				str(ImpostorCapturer._in_progress),
				avatar_count,
				Engine.max_fps,
				ctx["crdt_send_ops"],
				ctx["crdt_recv_ops"],
				ctx["crdt_recv_bytes"],
				ctx["v8_used_heap_bytes"] / 1048576,
			]
		)
	)
