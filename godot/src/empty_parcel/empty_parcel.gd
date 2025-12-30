class_name EmptyParcel
extends Node3D

const PARCEL_SIZE: float = 16.0
const PARCEL_HALF_SIZE: float = 8.0
const PARCEL_HEIGHT_BOUND: float = 100.0
const PARCEL_FULL_HEIGHT: float = 200.0
const OBSTACLE_COLLISION_LAYER: int = 2
const FALLOFF_DISTANCE: float = 8.0
const GRID_SIZE: int = 32
const CELL_SIZE: float = 0.5


class SpawnLocation:
	extends RefCounted
	var position: Vector3
	var normal: Vector3
	var falloff: float

	func _init(pos: Vector3 = Vector3.ZERO, norm: Vector3 = Vector3.UP, fall: float = 1.0):
		position = pos
		normal = norm
		falloff = fall


var corner_config: CornerConfiguration = CornerConfiguration.new()
var spawn_locations: Array[SpawnLocation] = []

@onready var terrain_generator: TerrainGenerator = $TerrainGenerator
@onready var cliff_generator: CliffGenerator = $CliffGenerator
@onready var grass_spawner: GrassSpawner = $GrassSpawner
@onready var rock_spawner: RockSpawner = $RockSpawner
@onready var props_spawner: PropsSpawner = $PropsSpawner
@onready var tree_spawner: TreeSpawner = $TreeSpawner


func _ready():
	terrain_generator.terrain_generated.connect(_on_terrain_generated)


func set_corner_configuration(config: CornerConfiguration) -> void:
	corner_config = config
	# Only generate if we're in the tree (global_position requires this)
	# This can happen if the node was removed before the deferred call executed
	if is_inside_tree():
		generate()


func get_corner_configuration() -> CornerConfiguration:
	return corner_config


func generate():
	terrain_generator.generate_terrain()
	cliff_generator.generate_cliffs()


func _on_terrain_generated():
	_populate_spawners()
	spawn_locations.clear()


func _populate_spawners():
	var grass_multimesh = get_node("Grass")
	grass_spawner.set_grass_multimesh(grass_multimesh)
	grass_spawner.populate_grass()

	rock_spawner.populate_rocks()
	props_spawner.populate_props()
	tree_spawner.populate_trees()


func calculate_displacement_falloff(grid_x: int, grid_z: int, grid_size: int) -> float:
	var local_pos = _grid_to_local_position(grid_x, grid_z, grid_size)
	var min_distance = _calculate_minimum_distance_to_neighbors(local_pos)
	var falloff = clamp(min_distance / FALLOFF_DISTANCE, 0.0, 1.0)
	return smoothstep(0.0, 1.0, falloff)


func _grid_to_local_position(grid_x: int, grid_z: int, grid_size: int) -> Vector2:
	var local_x: float
	var local_z: float

	if grid_x == 0:
		local_x = -PARCEL_HALF_SIZE
	elif grid_x == grid_size:
		local_x = PARCEL_HALF_SIZE
	else:
		var normalized_x = float(grid_x) / float(grid_size) - 0.5
		local_x = normalized_x * PARCEL_SIZE

	if grid_z == 0:
		local_z = -PARCEL_HALF_SIZE
	elif grid_z == grid_size:
		local_z = PARCEL_HALF_SIZE
	else:
		var normalized_z = float(grid_z) / float(grid_size) - 0.5
		local_z = normalized_z * PARCEL_SIZE

	return Vector2(local_x, local_z)


func _calculate_minimum_distance_to_neighbors(local_pos: Vector2) -> float:
	var min_distance = PARCEL_SIZE

	min_distance = _update_distance_for_corners(local_pos, min_distance)
	min_distance = _update_distance_for_edges(local_pos, min_distance)
	return min_distance


func _update_distance_for_corners(local_pos: Vector2, current_min: float) -> float:
	var corners = [
		{
			"config": corner_config.northwest,
			"pos": Vector2(-PARCEL_HALF_SIZE, -PARCEL_HALF_SIZE),
			"adjacent_edges": [corner_config.north, corner_config.west]
		},
		{
			"config": corner_config.northeast,
			"pos": Vector2(PARCEL_HALF_SIZE, -PARCEL_HALF_SIZE),
			"adjacent_edges": [corner_config.north, corner_config.east]
		},
		{
			"config": corner_config.southwest,
			"pos": Vector2(-PARCEL_HALF_SIZE, PARCEL_HALF_SIZE),
			"adjacent_edges": [corner_config.south, corner_config.west]
		},
		{
			"config": corner_config.southeast,
			"pos": Vector2(PARCEL_HALF_SIZE, PARCEL_HALF_SIZE),
			"adjacent_edges": [corner_config.south, corner_config.east]
		}
	]

	var result_distance = current_min
	for corner in corners:
		if corner.config == CornerConfiguration.ParcelState.LOADED:
			var both_edges_empty = (
				corner.adjacent_edges[0] == CornerConfiguration.ParcelState.EMPTY
				and corner.adjacent_edges[1] == CornerConfiguration.ParcelState.EMPTY
			)
			if both_edges_empty:
				var dist = (local_pos - corner.pos).length()
				result_distance = min(result_distance, dist)

	return result_distance


func _update_distance_for_edges(local_pos: Vector2, current_min: float) -> float:
	var edges = [
		{"config": corner_config.north, "distance": local_pos.y + PARCEL_HALF_SIZE},
		{"config": corner_config.south, "distance": PARCEL_HALF_SIZE - local_pos.y},
		{"config": corner_config.east, "distance": PARCEL_HALF_SIZE - local_pos.x},
		{"config": corner_config.west, "distance": local_pos.x + PARCEL_HALF_SIZE}
	]

	var result_distance = current_min
	for edge in edges:
		if edge.config != CornerConfiguration.ParcelState.EMPTY:
			result_distance = min(result_distance, edge.distance)

	return result_distance
