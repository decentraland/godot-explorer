class_name BaseFloorManager
extends Node3D

const FLOOR_SIZE = 16.0
const BASE_FLOOR_MATERIAL = preload("res://assets/empty-scenes/base_floor_material.tres")

var base_floor_multimesh: MultiMeshInstance3D = null
var loaded_parcels_floors: Dictionary = {}


func _ready():
	_initialize_base_floor_multimesh()

	if Services.scene_fetcher != null:
		Services.scene_fetcher.player_parcel_changed.connect(_on_player_parcel_changed)


func _initialize_base_floor_multimesh():
	base_floor_multimesh = MultiMeshInstance3D.new()
	base_floor_multimesh.name = "BaseFloorMultimesh"
	add_child(base_floor_multimesh)

	var array_mesh = ArrayMesh.new()
	var arrays = []
	arrays.resize(Mesh.ARRAY_MAX)
	var vertices = PackedVector3Array(
		[
			Vector3(-FLOOR_SIZE / 2, 0, -FLOOR_SIZE / 2),
			Vector3(FLOOR_SIZE / 2, 0, -FLOOR_SIZE / 2),
			Vector3(FLOOR_SIZE / 2, 0, FLOOR_SIZE / 2),
			Vector3(-FLOOR_SIZE / 2, 0, FLOOR_SIZE / 2)
		]
	)

	var uvs = PackedVector2Array([Vector2(0, 0), Vector2(1, 0), Vector2(1, 1), Vector2(0, 1)])
	var normals = PackedVector3Array([Vector3.UP, Vector3.UP, Vector3.UP, Vector3.UP])
	var indices = PackedInt32Array([0, 1, 2, 0, 2, 3])

	arrays[Mesh.ARRAY_VERTEX] = vertices
	arrays[Mesh.ARRAY_TEX_UV] = uvs
	arrays[Mesh.ARRAY_NORMAL] = normals
	arrays[Mesh.ARRAY_INDEX] = indices

	array_mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	array_mesh.surface_set_material(0, BASE_FLOOR_MATERIAL)
	var multimesh = MultiMesh.new()
	multimesh.mesh = array_mesh
	multimesh.transform_format = MultiMesh.TRANSFORM_3D
	multimesh.instance_count = 0
	base_floor_multimesh.multimesh = multimesh


func add_scene_floors(scene_id: String, parcels: Array):
	if not base_floor_multimesh or not base_floor_multimesh.multimesh:
		return

	if scene_id in loaded_parcels_floors:
		return

	loaded_parcels_floors[scene_id] = parcels.duplicate()
	_rebuild_multimesh()


func remove_scene_floors(scene_id: String):
	if not base_floor_multimesh or not base_floor_multimesh.multimesh:
		return

	if not scene_id in loaded_parcels_floors:
		return

	loaded_parcels_floors.erase(scene_id)
	_rebuild_multimesh()


func _rebuild_multimesh():
	var multimesh = base_floor_multimesh.multimesh
	var total := 0
	for parcels in loaded_parcels_floors.values():
		total += parcels.size()

	multimesh.instance_count = total

	var idx := 0
	for parcels in loaded_parcels_floors.values():
		for parcel in parcels:
			var parcel_x: float = parcel.x * FLOOR_SIZE + FLOOR_SIZE / 2.0
			var parcel_z: float = -parcel.y * FLOOR_SIZE - FLOOR_SIZE / 2.0
			var transform := Transform3D(Basis.IDENTITY, Vector3(parcel_x, -0.05, parcel_z))
			multimesh.set_instance_transform(idx, transform)
			idx += 1

	_update_base_floor_visibility()


func _update_base_floor_visibility():
	if base_floor_multimesh:
		base_floor_multimesh.visible = base_floor_multimesh.multimesh.instance_count > 0


func _on_player_parcel_changed(_new_parcel: Vector2i):
	pass


func clear_all_floors():
	loaded_parcels_floors.clear()
	if base_floor_multimesh and base_floor_multimesh.multimesh:
		base_floor_multimesh.multimesh.instance_count = 0
