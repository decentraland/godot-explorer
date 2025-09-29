class_name GrassSpawner
extends Node3D

@export var base_scale: float = 1.5

var parent_parcel: EmptyParcel
var grass_multimesh: MultiMeshInstance3D
var culling_range: int = 1


func _ready():
	parent_parcel = get_parent()
	if Global.scene_fetcher:
		Global.scene_fetcher.player_parcel_changed.connect(_on_player_parcel_changed)


func _on_player_parcel_changed(player_position: Vector2i):
	update_visibility(player_position)


func update_visibility(player_position: Vector2i):
	if not grass_multimesh:
		return

	var parcel_x = int(floor((parent_parcel.global_position.x - 8) / 16.0))
	var parcel_z = int(floor((-parent_parcel.global_position.z - 8) / 16.0))
	var parcel_pos = Vector2i(parcel_x, parcel_z)
	var distance_x = abs(parcel_pos.x - player_position.x)
	var distance_y = abs(parcel_pos.y - player_position.y)
	var distance = max(distance_x, distance_y)
	grass_multimesh.visible = distance <= culling_range


func populate_grass():
	if not grass_multimesh:
		return

	var multimesh = grass_multimesh.multimesh
	if not multimesh:
		return
	if Global.scene_fetcher:
		update_visibility(Global.scene_fetcher.current_position)

	var instance_count = parent_parcel.spawn_locations.size()
	multimesh.transform_format = MultiMesh.TRANSFORM_3D
	multimesh.instance_count = instance_count

	for i in range(instance_count):
		var spawn_location = parent_parcel.spawn_locations[i]
		var grass_pos = spawn_location.position
		var grass_normal = spawn_location.normal
		var falloff = spawn_location.falloff

		var random_variation = 0.8 + randf() * 0.4
		var grass_scale_falloff = pow(falloff, 0.3)
		var final_scale = base_scale * grass_scale_falloff * random_variation
		var transform = ParcelUtils.create_aligned_transform(
			grass_pos, grass_normal, true, final_scale
		)

		multimesh.set_instance_transform(i, transform)


func set_grass_multimesh(multimesh_instance: MultiMeshInstance3D):
	grass_multimesh = multimesh_instance
