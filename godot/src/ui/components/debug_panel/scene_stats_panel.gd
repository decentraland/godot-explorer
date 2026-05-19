class_name SceneStatsPanel
extends CanvasLayer

## Lightweight overlay shown in preview mode with realtime scene stats.
## Instantiated lazily by debug_panel.gd when its toggle is ON; freed when OFF.

const UPDATE_INTERVAL := 0.5

@onready var label_scene_name: Label = %Label_SceneName
@onready var label_scene_meta: Label = %Label_SceneMeta
@onready var label_fps: Label = %Value_FPS
@onready var label_visible_meshes: Label = %Value_VisibleMeshes
@onready var label_visible_triangles: Label = %Value_VisibleTriangles
@onready var label_draw_calls: Label = %Value_DrawCalls
@onready var label_texture_mem: Label = %Value_TextureMem
@onready var label_video_mem: Label = %Value_VideoMem
@onready var label_total_entities: Label = %Value_TotalEntities

var _timer: float = 0.0

# Cache triangle count per Mesh resource (keyed by instance_id) so repeated
# walks don't repeatedly hit surface_get_array — a single mesh stays cheap
# even if it's instanced across thousands of entities.
var _mesh_tri_cache: Dictionary = {}


func _ready() -> void:
	layer = 50
	_refresh()


func _process(delta: float) -> void:
	_timer += delta
	if _timer < UPDATE_INTERVAL:
		return
	_timer = 0.0
	_refresh()


func _refresh() -> void:
	var current_scene = Global.scene_fetcher.get_current_scene_data()
	if current_scene != null:
		label_scene_name.text = current_scene.scene_entity_definition.get_title()
		var base: Vector2i = current_scene.scene_entity_definition.get_base_parcel()
		label_scene_meta.text = "(%d, %d)" % [base.x, base.y]
	else:
		label_scene_name.text = "—"
		label_scene_meta.text = ""

	label_fps.text = "%d" % Engine.get_frames_per_second()
	label_visible_meshes.text = (
		"%d" % Performance.get_monitor(Performance.RENDER_TOTAL_OBJECTS_IN_FRAME)
	)
	label_visible_triangles.text = _format_int(_count_scene_triangles())
	label_draw_calls.text = (
		"%d" % Performance.get_monitor(Performance.RENDER_TOTAL_DRAW_CALLS_IN_FRAME)
	)
	label_texture_mem.text = (
		"%.1f MB" % (Performance.get_monitor(Performance.RENDER_TEXTURE_MEM_USED) / 1048576.0)
	)
	label_video_mem.text = (
		"%.1f MB" % (Performance.get_monitor(Performance.RENDER_VIDEO_MEM_USED) / 1048576.0)
	)
	label_total_entities.text = _format_int(Global.scene_runner.get_all_scenes_total_entities())


func _count_scene_triangles() -> int:
	var total := 0
	for root in Global.scene_runner.get_scene_root_nodes():
		total += _count_node_triangles(root)
	return total


func _count_node_triangles(node: Node) -> int:
	var total := 0
	if node is MeshInstance3D:
		var mi := node as MeshInstance3D
		if mi.visible and mi.mesh != null:
			total += _mesh_triangles(mi.mesh)
	elif node is MultiMeshInstance3D:
		var mmi := node as MultiMeshInstance3D
		if mmi.visible and mmi.multimesh != null and mmi.multimesh.mesh != null:
			var per_inst := _mesh_triangles(mmi.multimesh.mesh)
			var count := mmi.multimesh.visible_instance_count
			if count < 0:
				count = mmi.multimesh.instance_count
			total += per_inst * count
	for child in node.get_children():
		total += _count_node_triangles(child)
	return total


func _mesh_triangles(mesh: Mesh) -> int:
	var key: int = mesh.get_instance_id()
	if _mesh_tri_cache.has(key):
		return _mesh_tri_cache[key]
	var tri := 0
	for i in range(mesh.get_surface_count()):
		var arrays: Array = mesh.surface_get_arrays(i)
		var indices = arrays[Mesh.ARRAY_INDEX]
		if indices != null and indices.size() > 0:
			tri += indices.size() / 3
		else:
			var verts = arrays[Mesh.ARRAY_VERTEX]
			if verts != null:
				tri += verts.size() / 3
	_mesh_tri_cache[key] = tri
	return tri


static func _format_int(value: int) -> String:
	var s := str(value)
	var result := ""
	var count := 0
	for i in range(s.length() - 1, -1, -1):
		result = s[i] + result
		count += 1
		if count % 3 == 0 and i > 0:
			result = "," + result
	return result
