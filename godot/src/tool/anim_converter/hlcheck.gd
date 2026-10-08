extends SceneTree


func _initialize() -> void:
	var lib: AnimationLibrary = load("res://assets/animations/locomotion_full.tres")
	var a: Animation = lib.get_animation("Hard_Landing")
	prints("Hard_Landing length:", a.length)
	for i in range(a.get_track_count()):
		var p := str(a.track_get_path(i))
		if a.track_get_type(i) == Animation.TYPE_POSITION_3D and p.ends_with(":Avatar_Hips"):
			for k in range(11):
				var t := a.length * k / 10.0
				prints("t=%.2f" % t, " hips:", a.position_track_interpolate(i, t))
	quit(0)
