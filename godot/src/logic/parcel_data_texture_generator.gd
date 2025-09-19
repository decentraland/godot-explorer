class_name ParcelDataTextureGenerator
extends Node

signal parcel_data_texture_updated(texture: ImageTexture)

const TEXTURE_SIZE = 64
const TEXTURE_RADIUS = 32

var current_center_parcel: Vector2i = Vector2i.ZERO
var parcel_data_texture: ImageTexture
var map_data_texture: Texture2D

# Parcel colors (black/white/green)
const PARCEL_COLORS = {
	"empty": Color.BLACK,  # No parcel data
	"district": Color.WHITE,  # District parcels
	"plaza": Color.GREEN,  # Plaza parcels
	"road": Color.WHITE,  # Road parcels
	"owned": Color.WHITE,  # Owned parcels
	"unowned": Color.BLACK  # Unowned parcels
}


func _ready():
	parcel_data_texture = ImageTexture.new()


func set_map_data(texture: Texture2D):
	map_data_texture = texture


func generate_parcel_data_texture_for_parcel(center_parcel: Vector2i):
	current_center_parcel = center_parcel

	# Create 64x64 image
	var image = Image.create(TEXTURE_SIZE, TEXTURE_SIZE, false, Image.FORMAT_RGB8)

	# Get parcel data from SceneFetcher
	var scene_fetcher = Global.scene_fetcher
	if not scene_fetcher:
		return


	# Fill the texture based on SceneFetcher parcel data
	for y in range(TEXTURE_SIZE):
		for x in range(TEXTURE_SIZE):
			# Convert texture pixel to world parcel coordinate
			var offset_x = x - TEXTURE_RADIUS
			var offset_y = y - TEXTURE_RADIUS
			var world_parcel = center_parcel + Vector2i(offset_x, offset_y)

			var pixel_color = _get_parcel_color_from_scene_fetcher(world_parcel, scene_fetcher)
			image.set_pixel(x, y, pixel_color)

	# Update texture
	parcel_data_texture.set_image(image)

	# Set as global uniform
	RenderingServer.global_shader_parameter_set("parcel_data_texture", parcel_data_texture)

	parcel_data_texture_updated.emit(parcel_data_texture)

func _get_parcel_color_from_map_pixel(pixel: Color) -> Color:
	# Extract flags from pixel (matching the map shader logic)
	var flags_g = int(pixel.g * 255.0)

	if flags_g == 32:
		return PARCEL_COLORS["district"]
	elif flags_g == 64:
		return PARCEL_COLORS["road"]
	elif flags_g == 128:
		return PARCEL_COLORS["owned"]
	elif flags_g > 0:
		return PARCEL_COLORS["plaza"]
	else:
		return PARCEL_COLORS["empty"]


func _get_parcel_color_from_scene_fetcher(world_parcel: Vector2i, scene_fetcher) -> Color:
	# Check if this parcel has a loaded scene (scenes use parcel arrays, not string keys)
	for scene_entity_id in scene_fetcher.loaded_scenes.keys():
		var scene = scene_fetcher.loaded_scenes[scene_entity_id]
		# Check if this parcel is in the scene's parcel list
		for parcel_coord in scene.parcels:
			if parcel_coord.x == world_parcel.x and parcel_coord.y == world_parcel.y:
				return PARCEL_COLORS["owned"]  # White for loaded scenes

	# Convert parcel coordinate to string format used by empty scenes
	var parcel_string = "%d,%d" % [world_parcel.x, world_parcel.y]

	# Check if this parcel has an empty scene
	if scene_fetcher.loaded_empty_scenes.has(parcel_string):
		return PARCEL_COLORS["plaza"]  # Green for empty parcels
	else:
		return PARCEL_COLORS["empty"]  # Black for unloaded parcels


func get_current_parcel_data_texture() -> ImageTexture:
	return parcel_data_texture
