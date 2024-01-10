class_name SceneHelper


static func search_scene_node(target: Node3D) -> DclSceneNode:
	if target is DclSceneNode:
		return target

	var parent_node_3d = target.get_parent_node_3d()
	if parent_node_3d == null:
		return null

	return search_scene_node(parent_node_3d)
