extends SceneTree

# Builds the locomotion AnimationNodes with the engine API (canonical
# serialization guaranteed) and saves them as .tres resources referenced by
# avatar.tscn. Run:
#   .bin/godot/Godot.app/Contents/MacOS/Godot --headless --path godot \
#       --script src/tool/anim_converter/build_locomotion_nodes.gd

const OUT_DIR := "res://src/decentraland_components/avatar/anim_nodes/"


func _initialize() -> void:
	DirAccess.make_dir_recursive_absolute(
		ProjectSettings.globalize_path(OUT_DIR.trim_prefix("res://"))
	)
	_make_grounded()
	prints("done")
	quit(0)


# Grounded gait blend: Idle=0, Walk=1, Jog=2, Run=3 over the MovementBlend
# parameter (Unity CharacterAnimator Movement blend tree, thresholds 0..3).
# Gait clips are phase-aligned by their left-foot contact (lowest point of
# Avatar_LeftFoot) so mid-blend footfalls don't stumble. Reference = Walk.
func _make_grounded() -> void:
	var offsets := _foot_contact_offsets()
	prints("foot contact offsets:", offsets)
	var bs := AnimationNodeBlendSpace1D.new()
	# sync: blend by normalized phase — without it the different-length gait
	# clips (Idle 2s / Walk 1s / Jog 0.67s / Run 0.53s) drift against each
	# other and the legs churn ("patinando").
	bs.sync = true
	for entry in [["Idle", 0.0], ["Walk", 1.0], ["Jog", 2.0], ["Run", 3.0]]:
		var anim := AnimationNodeAnimation.new()
		anim.animation = StringName("loco/" + entry[0])
		anim.start_offset = offsets.get(entry[0], 0.0)
		bs.add_blend_point(anim, entry[1])
	bs.min_space = 0.0
	bs.max_space = 3.0
	bs.value_label = "movement_blend"
	_save(bs, "grounded")


# clip name -> start_offset aligning its left-foot contact to Walk's.
func _foot_contact_offsets() -> Dictionary:
	var lib: AnimationLibrary = load("res://assets/animations/locomotion_full.tres")
	var contact_phase := {}
	for clip in ["Walk", "Jog", "Run"]:
		contact_phase[clip] = _contact_phase(lib.get_animation(clip))
	var ref: float = contact_phase["Walk"]
	var out := {}
	for clip in contact_phase:
		var a: Animation = lib.get_animation(clip)
		# Shift so the contact lands at the reference normalized phase.
		var delta: float = contact_phase[clip] - ref
		out[clip] = -delta * a.length
	return out


# Normalized phase (0..1) of the left foot's lowest point in the clip.
func _contact_phase(anim: Animation) -> float:
	for i in range(anim.get_track_count()):
		var p := str(anim.track_get_path(i))
		if anim.track_get_type(i) == Animation.TYPE_POSITION_3D and p.ends_with(":Avatar_LeftFoot"):
			var best_t := 0.0
			var best_y := INF
			var steps := 120
			for k in range(steps):
				var t := anim.length * k / float(steps)
				var y: float = anim.position_track_interpolate(i, t).y
				if y < best_y:
					best_y = y
					best_t = t
			return best_t / anim.length
	return 0.0


func _save(res: Resource, res_name: String) -> void:
	var err := ResourceSaver.save(res, OUT_DIR + res_name + ".tres")
	prints(res_name, "err:", err)
