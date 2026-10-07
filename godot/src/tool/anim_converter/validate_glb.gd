# gdlint: disable=async-function-name
extends SceneTree

# Validate the locomotion library: play clips on the avatar skeleton and
# compare bone global positions against the known-good locomotion.res clips.
# Run:
#   .bin/godot/Godot.app/Contents/MacOS/Godot --headless --path godot \
#       --script src/tool/anim_converter/validate_glb.gd

const LIB := "res://assets/animations/locomotion_full.tres"
const BONES := ["Avatar_Hips", "Avatar_Head", "Avatar_LeftHand", "Avatar_LeftFoot"]
# converted clip -> existing clip in locomotion.res
const PAIRS := [["Jump_Fall", "Jump_Fall"], ["Idle", "Idle"], ["Walk", "Walk"]]


func _initialize() -> void:
	var new_lib: AnimationLibrary = load(LIB)  # sanity: must load
	if new_lib == null:
		printerr("library did not load: ", LIB)
		quit(1)
		return
	var scene: PackedScene = load("res://src/decentraland_components/avatar/avatar.tscn")
	var avatar := scene.instantiate()
	root.add_child(avatar)
	var tree: AnimationTree = avatar.get_node("AnimationTree")
	tree.active = false  # pure AnimationPlayer application
	var player: AnimationPlayer = avatar.get_node("AnimationPlayer")
	var skel: Skeleton3D = avatar.get_node("Armature/Skeleton3D")
	await process_frame

	var worst := 0.0
	for pair in PAIRS:
		var new_clip: String = "loco/" + pair[0]
		skel.reset_bone_poses()
		player.play(new_clip)
		player.advance(0.3)
		await process_frame
		var got := {}
		for bone in BONES:
			got[bone] = skel.get_bone_global_pose(skel.find_bone(bone)).origin
		skel.reset_bone_poses()
		player.play(pair[1])
		player.advance(0.3)
		await process_frame
		for bone in BONES:
			var diff: float = (
				(got[bone] - skel.get_bone_global_pose(skel.find_bone(bone)).origin).length()
			)
			worst = maxf(worst, diff)
	prints("worst bone position diff (cm):", worst)
	quit(0 if worst < 1.0 else 1)
