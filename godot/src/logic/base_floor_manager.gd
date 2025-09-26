class_name BaseFloorManager
extends Node3D

const FLOOR_SIZE = 16.0
const EMPTY_PARCEL_MATERIAL = preload("res://assets/empty-scenes/empty_parcel_material.tres")

var base_floor_multimesh: MultiMeshInstance3D = null
var loaded_parcels_floors: Dictionary = {}


func _ready():
	_initialize_base_floor_multimesh()

	if Global.scene_fetcher != null:
		Global.scene_fetcher.player_parcel_changed.connect(_on_player_parcel_changed)


func _initialize_base_floor_multimesh():
	base_floor_multimesh = MultiMeshInstance3D.new()
	base_floor_multimesh.name = "BaseFloorMultimesh"
	add_child(base_floor_multimesh)

	var quad_mesh = QuadMesh.new()
	quad_mesh.size = Vector2(FLOOR_SIZE, FLOOR_SIZE)
	quad_mesh.material = EMPTY_PARCEL_MATERIAL
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
	array_mesh.surface_set_material(0, EMPTY_PARCEL_MATERIAL)
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

	var multimesh = base_floor_multimesh.multimesh
	var current_count = multimesh.instance_count
	var new_floors = []
	multimesh.instance_count = current_count + parcels.size()
	for i in range(parcels.size()):
		var parcel = parcels[i]
		var instance_idx = current_count + i

		var parcel_x = parcel.x * FLOOR_SIZE + FLOOR_SIZE / 2
		var parcel_z = -parcel.y * FLOOR_SIZE - FLOOR_SIZE / 2

		var transform = Transform3D()
		transform.origin = Vector3(parcel_x, 0, parcel_z)
		multimesh.set_instance_transform(instance_idx, transform)

		new_floors.append(instance_idx)

	loaded_parcels_floors[scene_id] = new_floors
	_update_base_floor_visibility()


func remove_scene_floors(scene_id: String):
	if not base_floor_multimesh or not base_floor_multimesh.multimesh:
		return

	if not scene_id in loaded_parcels_floors:
		return

	var multimesh = base_floor_multimesh.multimesh

	var remaining_transforms = []
	var remaining_scene_ids = {}

	for sid in loaded_parcels_floors:
		if sid != scene_id:
			for idx in loaded_parcels_floors[sid]:
				remaining_transforms.append(multimesh.get_instance_transform(idx))
	multimesh.instance_count = remaining_transforms.size()
	var new_idx = 0
	for sid in loaded_parcels_floors:
		if sid != scene_id:
			var new_indices = []
			for old_idx in loaded_parcels_floors[sid]:
				multimesh.set_instance_transform(new_idx, remaining_transforms[new_idx])
				new_indices.append(new_idx)
				new_idx += 1
			remaining_scene_ids[sid] = new_indices

	loaded_parcels_floors = remaining_scene_ids
	_update_base_floor_visibility()


func _update_base_floor_visibility():
	if base_floor_multimesh:
		base_floor_multimesh.visible = base_floor_multimesh.multimesh.instance_count > 0


func _on_player_parcel_changed(_new_parcel: Vector2i):
	pass


func clear_all_floors():
	if base_floor_multimesh and base_floor_multimesh.multimesh:
		base_floor_multimesh.multimesh.instance_count = 0
		loaded_parcels_floors.clear()
