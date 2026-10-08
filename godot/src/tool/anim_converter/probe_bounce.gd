# gdlint: disable=async-function-name
extends SceneTree


func _initialize() -> void:
	var scene: PackedScene = load("res://src/decentraland_components/avatar/avatar.tscn")
	var avatar = scene.instantiate()
	root.add_child(avatar)
	await process_frame
	await process_frame
	var tree: AnimationTree = avatar.get_node("AnimationTree")
	var pb = tree.get("parameters/Locomotion/playback")
	var last := ""
	# long fall -> touch pad (1 frame grounded) -> bounce up -> fall again
	var script := [
		[30, {"fall": true, "long_fall": true, "is_grounded": false}],
		[3, {"land": true, "is_grounded": true, "fall": false, "long_fall": false}],
		[10, {"land": false, "is_grounded": false, "rise": true}],
		[30, {"rise": false, "fall": true}],
	]
	var f := 0
	for step in script:
		for k in step[0]:
			for prop in step[1]:
				avatar.set(prop, step[1][prop])
			await process_frame
			f += 1
			var cur: String = pb.get_current_node()
			if cur != last:
				prints("f", f, "->", cur)
				last = cur
	quit(0)
