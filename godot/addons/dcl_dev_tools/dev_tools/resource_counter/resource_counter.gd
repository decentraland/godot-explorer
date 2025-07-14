extends Node

var meshes = []
var materials = []
var unique_meshes = {}
var unique_materials = {}


func count(node: Node):
	var mesh_nodes = get_meshes_below(node)
	for instance in mesh_nodes:
		var mesh = instance.mesh
		meshes.push_back(mesh)
		for idx in range(mesh.get_surface_count()):
			var material = mesh.surface_get_material(idx)
			materials.push_back(material)


func get_meshes_below(root: Node):
	var mesh_nodes = []
	var children = root.get_children()
	while children.size():
		var child = children.pop_back()
		if child is MeshInstance3D:
			mesh_nodes.push_back(child)
		var grandchildren = child.get_children()
		for grandchild in grandchildren:
			children.push_back(grandchild)

	return mesh_nodes


func log_active_counts():
	var counts = PerformanceCounts.new()

	count(get_tree().get_root().get_node("scene_runner"))

	counts.total_meshes = meshes.size()
	counts.total_materials = materials.size()

	var mesh_rid_map = meshes.reduce(
		func(acc, mesh):
			if not mesh:
				return acc
			var mesh_rid = mesh.get_rid()
			if not acc.has(mesh_rid):
				acc.objects[mesh_rid] = mesh
			acc[mesh_rid] = acc.get_or_add(mesh_rid, 0) + 1
			return acc,
		{"objects": {}}
	)
	counts.mesh_rid_count = mesh_rid_map.size() - 1  # Subtract the "objects" key

	var material_rid_map = materials.reduce(
		func(acc, material):
			if not material:
				return acc
			var material_rid = material.get_rid()
			if not acc.has(material_rid):
				acc.objects[material_rid] = material
			acc[material_rid] = acc.get_or_add(material_rid, 0) + 1
			return acc,
		{"objects": {}}
	)
	counts.material_rid_count = material_rid_map.size() - 1

	var mesh_hash_map = meshes.reduce(
		func(acc, mesh):
			var arrays = []
			for surface_id in mesh.get_surface_count():
				arrays.append_array(mesh.surface_get_arrays(surface_id))
			var mesh_hash = arrays.hash()
			if not acc.has(mesh_hash):
				acc.objects[mesh_hash] = mesh
			acc[mesh_hash] = acc.get_or_add(mesh_hash, 0) + 1
			return acc,
		{"objects": {}}
	)
	counts.mesh_hash_count = mesh_hash_map.size() - 1

	counts.potential_dedup_count = counts.mesh_rid_count - counts.mesh_hash_count
	counts.mesh_savings_percent = (counts.potential_dedup_count * 100.0) / counts.mesh_rid_count

	counts.timestamp = Time.get_datetime_string_from_system()
	counts.parcel_position = Global.get_explorer().parcel_position

	counts.fps = Performance.get_monitor(Performance.TIME_FPS)
	counts.draw_calls = Performance.get_monitor(Performance.RENDER_TOTAL_DRAW_CALLS_IN_FRAME)
	counts.primitives_drawn = Performance.get_monitor(Performance.RENDER_TOTAL_PRIMITIVES_IN_FRAME)
	counts.video_mem_used = Performance.get_monitor(Performance.RENDER_VIDEO_MEM_USED)
	counts.static_memory = Performance.get_monitor(Performance.MEMORY_STATIC)

	counts.print()
	counts.log(get_viewport())
	get_tree().quit()


class PerformanceCounts:
	var total_meshes: int
	var total_materials: int
	var mesh_rid_count: int
	var material_rid_count: int
	var mesh_hash_count: int
	var potential_dedup_count: int
	var mesh_savings_percent: float
	var fps: int
	var draw_calls: int
	var primitives_drawn: int
	var video_mem_used: int
	var static_memory: int
	var timestamp: String
	var parcel_position: Vector2i

	func print():
		print("############## - TOTAL COUNTS - ##############")
		print("Total mesh references: %d" % total_meshes)
		print("Total material references: %d" % total_materials)
		print("RID mesh count: %d" % mesh_rid_count)
		print("RID material count: %d" % material_rid_count)
		print("Hashed mesh count: %d" % mesh_hash_count)
		print("Potential mesh deduplication count: %d" % potential_dedup_count)
		print("Potential mesh count savings: %.1f%%" % mesh_savings_percent)
		print("FPS: %d" % fps)
		print("Draw calls: %d" % draw_calls)
		print("Primitives drawn: %d" % primitives_drawn)
		print("Video memory in use (bytes): %d" % video_mem_used)
		print("Static memory in use (bytes): %d" % static_memory)
		print("Timestamp: %s" % timestamp)
		print("Parcel position: %s" % parcel_position)
		print("##############################################")

	func log(view: Viewport):
		var screenshot = view.get_texture().get_image().save_png(
			"res://output/[%d,%d]-%s.png" % [parcel_position.x, parcel_position.y, timestamp]
		)
		var file_path = "res://output/performance_log.csv"
		var file
		if !FileAccess.file_exists(file_path):
			file = FileAccess.open(file_path, FileAccess.WRITE)
			if not file:
				print("Failed to create log file.")
				return
			(
				file
				. store_line(
					"timestamp;parcel_position;total_meshes;total_materials;mesh_rid_count;material_rid_count;mesh_hash_count;potential_dedup_count;mesh_savings_percent;fps;draw_calls;primitives_drawn;video_mem_used;static_memory"
				)
			)
		else:
			file = FileAccess.open(file_path, FileAccess.READ_WRITE)
			if not file:
				print("Failed to open log file.")
				return
		file.seek_end()
		# Write data row
		file.store_line(
			(
				"%s;%s;%d;%d;%d;%d;%d;%d;%.1f;%d;%d;%d;%d;%d"
				% [
					timestamp,
					parcel_position,
					total_meshes,
					total_materials,
					mesh_rid_count,
					material_rid_count,
					mesh_hash_count,
					potential_dedup_count,
					mesh_savings_percent,
					fps,
					draw_calls,
					primitives_drawn,
					video_mem_used,
					static_memory
				]
			)
		)
		file.close()
