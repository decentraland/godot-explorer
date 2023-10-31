extends Node3D
class_name AnimationImporter

func get_animation_from_gltf(animation_name: String) -> Animation:
	var path := "res://assets/animations/%s.glb" % animation_name
	var gltf := GLTFDocument.new()
	var state := GLTFState.new()
	state.set_additional_data("placeholder_image", true)
	var err = gltf.append_from_file(path, state)
	if err != OK:
		printerr("Load failure")
		return null

	var node = gltf.generate_scene(state)

	if node == null:
		printerr("Node failure!")
		return null
		
	var animation_player: AnimationPlayer = node.get_node("AnimationPlayer")
	var animations_names: PackedStringArray = animation_player.get_animation_list()
	if animations_names.is_empty():
		printerr("No animations!")
		return null
		
	var animation: Animation = animation_player.get_animation(animations_names[0])
	if animation == null:
		printerr("Invalid animation")
		return
		
	for track_idx in range(animation.get_track_count()):
		var track_path = animation.track_get_path(track_idx)
		if track_path.get_concatenated_names().contains("Skeleton3D") == false:
			# Requires new path!
			var last_track_path = track_path.get_name(track_path.get_name_count() - 1)
			var new_track_path = "Armature/Skeleton3D:" + last_track_path
			animation.track_set_path(track_idx, new_track_path)

	return animation
