extends Node

# Instrumented repro for #2732 (bounce after landing at mobile FPS).
# Caps FPS/physics to mobile rates and logs per-tick floor/velocity/position
# during each jump episode. Read the trace:
#   - velocity.y goes POSITIVE or position.y pops up after touchdown
#       -> real physics bounce (separation ray / y-clamp), touch geometry
#   - clean physics but is_on_floor flickers after landing
#       -> animation replay; the GROUNDED_GRACE_WINDOW debounce covers it

var _player: CharacterBody3D = null
var _airborne := false
var _post_land_ticks := 0


func _ready():
	Engine.max_fps = 30
	Engine.physics_ticks_per_second = 30
	print("[repro_2732] FPS cap forced to 30 — jump and watch the landing trace")


func _physics_process(_delta: float) -> void:
	if _player == null:
		var found := get_tree().root.find_children("*", "Player", true, false)
		if not found.is_empty():
			_player = found[0]
			print("[repro_2732] attached to player: ", _player.get_path())
		return

	var on_floor: bool = _player.is_on_floor()
	if not on_floor:
		if not _airborne:
			print("[repro_2732] --- left ground ---")
		_airborne = true
		_post_land_ticks = 40  # keep logging ~1.3s after touchdown

	if _airborne:
		print(
			(
				"[repro_2732] floor=%s vy=%.3f py=%.4f"
				% [on_floor, _player.velocity.y, _player.global_position.y]
			)
		)
		if on_floor:
			_post_land_ticks -= 1
			if _post_land_ticks <= 0:
				_airborne = false
				print("[repro_2732] --- settled ---")
