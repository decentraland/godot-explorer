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
func _make_grounded() -> void:
	var bs := AnimationNodeBlendSpace1D.new()
	# sync: blend by normalized phase — without it the different-length gait
	# clips (Idle 2s / Walk 1s / Jog 0.67s / Run 0.53s) drift against each
	# other and the legs churn ("patinando").
	bs.sync = true
	for entry in [["Idle", 0.0], ["Walk", 1.0], ["Jog", 2.0], ["Run", 3.0]]:
		var anim := AnimationNodeAnimation.new()
		anim.animation = StringName("loco/" + entry[0])
		bs.add_blend_point(anim, entry[1])
	bs.min_space = 0.0
	bs.max_space = 3.0
	bs.value_label = "movement_blend"
	_save(bs, "grounded")


func _save(res: Resource, res_name: String) -> void:
	var err := ResourceSaver.save(res, OUT_DIR + res_name + ".tres")
	prints(res_name, "err:", err)
