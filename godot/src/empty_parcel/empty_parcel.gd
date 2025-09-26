class_name EmptyParcel
extends Node3D


class SpawnLocation:
	extends RefCounted
	var position: Vector3
	var normal: Vector3
	var falloff: float

	func _init(pos: Vector3 = Vector3.ZERO, norm: Vector3 = Vector3.UP, fall: float = 1.0):
		position = pos
		normal = norm
		falloff = fall


enum EmptyParcelType {
	NONE,
	NORTH,
	EAST,
	SOUTH,
	WEST,
	NORTHEAST,
	NORTHWEST,
	SOUTHEAST,
	SOUTHWEST,
	INNER_NORTH,
	INNER_EAST,
	INNER_SOUTH,
	INNER_WEST,
	INNER_NORTHEAST,
	INNER_NORTHWEST,
	INNER_SOUTHEAST,
	INNER_SOUTHWEST
}

@export var parcel_type: EmptyParcelType = EmptyParcelType.NONE

var spawn_locations: Array[SpawnLocation] = []

@onready var terrain_generator: TerrainGenerator = $TerrainGenerator
@onready var cliff_generator: CliffGenerator = $CliffGenerator
@onready var grass_spawner: GrassSpawner = $GrassSpawner
@onready var rock_spawner: RockSpawner = $RockSpawner
@onready var props_spawner: PropsSpawner = $PropsSpawner
@onready var tree_spawner: TreeSpawner = $TreeSpawner


func _ready():
	if terrain_generator:
		terrain_generator.terrain_generated.connect(_on_terrain_generated)


func set_parcel_type(type: EmptyParcelType) -> void:
	parcel_type = type
	regenerate()


func get_parcel_type() -> EmptyParcelType:
	return parcel_type


func regenerate():
	if terrain_generator:
		terrain_generator.generate_terrain()
	if cliff_generator:
		cliff_generator.generate_cliffs()


func _on_terrain_generated():
	if grass_spawner:
		var grass_multimesh = get_node("Grass")
		grass_spawner.set_grass_multimesh(grass_multimesh)
		grass_spawner.populate_grass()
	if rock_spawner:
		rock_spawner.populate_rocks()
	if props_spawner:
		props_spawner.populate_props()
	if tree_spawner:
		tree_spawner.populate_trees()

	spawn_locations.clear()


func calculate_displacement_falloff(grid_x: int, grid_z: int, grid_size: int) -> float:
	var norm_x = float(grid_x) / float(grid_size - 1)
	var norm_z = float(grid_z) / float(grid_size - 1)

	var falloff = 1.0

	match parcel_type:
		EmptyParcelType.NORTH:
			falloff = clamp(norm_z * 2.0, 0.0, 1.0)
		EmptyParcelType.SOUTH:
			falloff = clamp((1.0 - norm_z) * 2.0, 0.0, 1.0)
		EmptyParcelType.EAST:
			falloff = clamp((1.0 - norm_x) * 2.0, 0.0, 1.0)
		EmptyParcelType.WEST:
			falloff = clamp(norm_x * 2.0, 0.0, 1.0)

		EmptyParcelType.NORTHEAST:
			falloff = clamp(norm_z * 2.0, 0.0, 1.0) * clamp((1.0 - norm_x) * 2.0, 0.0, 1.0)
		EmptyParcelType.NORTHWEST:
			falloff = clamp(norm_z * 2.0, 0.0, 1.0) * clamp(norm_x * 2.0, 0.0, 1.0)
		EmptyParcelType.SOUTHEAST:
			falloff = clamp((1.0 - norm_z) * 2.0, 0.0, 1.0) * clamp((1.0 - norm_x) * 2.0, 0.0, 1.0)
		EmptyParcelType.SOUTHWEST:
			falloff = clamp((1.0 - norm_z) * 2.0, 0.0, 1.0) * clamp(norm_x * 2.0, 0.0, 1.0)

		EmptyParcelType.INNER_NORTH:
			falloff = clamp((1.0 - norm_z) * 2.0, 0.0, 1.0)
		EmptyParcelType.INNER_SOUTH:
			falloff = clamp(norm_z * 2.0, 0.0, 1.0)
		EmptyParcelType.INNER_EAST:
			falloff = clamp(norm_x * 2.0, 0.0, 1.0)
		EmptyParcelType.INNER_WEST:
			falloff = clamp((1.0 - norm_x) * 2.0, 0.0, 1.0)

		EmptyParcelType.INNER_NORTHEAST:
			var north_falloff = clamp((1.0 - norm_z) * 2.0, 0.0, 1.0)
			var east_falloff = clamp(norm_x * 2.0, 0.0, 1.0)
			falloff = max(north_falloff, east_falloff)
		EmptyParcelType.INNER_NORTHWEST:
			var north_falloff = clamp((1.0 - norm_z) * 2.0, 0.0, 1.0)
			var west_falloff = clamp((1.0 - norm_x) * 2.0, 0.0, 1.0)
			falloff = max(north_falloff, west_falloff)
		EmptyParcelType.INNER_SOUTHEAST:
			var south_falloff = clamp(norm_z * 2.0, 0.0, 1.0)
			var east_falloff = clamp(norm_x * 2.0, 0.0, 1.0)
			falloff = max(south_falloff, east_falloff)
		EmptyParcelType.INNER_SOUTHWEST:
			var south_falloff = clamp(norm_z * 2.0, 0.0, 1.0)
			var west_falloff = clamp((1.0 - norm_x) * 2.0, 0.0, 1.0)
			falloff = max(south_falloff, west_falloff)

		_:
			falloff = 1.0

	return falloff
