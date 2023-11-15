class_name NftFrameStyleLoader

enum NFTFrameStyles {
	NFT_CLASSIC,
	NFT_BAROQUE_ORNAMENT,
	NFT_DIAMOND_ORNAMENT,
	NFT_MINIMAL_WIDE,
	NFT_MINIMAL_GREY,
	NFT_BLOCKY,
	NFT_GOLD_EDGES,
	NFT_GOLD_CARVED,
	NFT_GOLD_WIDE,
	NFT_GOLD_ROUNDED,
	NFT_METAL_MEDIUM,
	NFT_METAL_WIDE,
	NFT_METAL_SLIM,
	NFT_METAL_ROUNDED,
	NFT_PINS,
	NFT_MINIMAL_BLACK,
	NFT_MINIMAL_WHITE,
	NFT_TAPE,
	NFT_WOOD_SLIM,
	NFT_WOOD_WIDE,
	NFT_WOOD_TWIGS,
	NFT_CANVAS,
	NFT_NONE
}

var type_to_asset: Dictionary = {}


func _init():
	type_to_asset[NFTFrameStyles.NFT_CLASSIC] = preload("res://assets/nftshape/Classic.glb")
	type_to_asset[NFTFrameStyles.NFT_BAROQUE_ORNAMENT] = preload(
		"res://assets/nftshape/Baroque_Ornament.glb"
	)
	type_to_asset[NFTFrameStyles.NFT_DIAMOND_ORNAMENT] = preload(
		"res://assets/nftshape/Diamond_Ornament.glb"
	)
	type_to_asset[NFTFrameStyles.NFT_MINIMAL_WIDE] = preload(
		"res://assets/nftshape/Minimal_Wide.glb"
	)
	type_to_asset[NFTFrameStyles.NFT_MINIMAL_GREY] = preload(
		"res://assets/nftshape/Minimal_Grey.glb"
	)
	type_to_asset[NFTFrameStyles.NFT_BLOCKY] = preload("res://assets/nftshape/Blocky.glb")
	type_to_asset[NFTFrameStyles.NFT_GOLD_EDGES] = preload("res://assets/nftshape/Gold_Edges.glb")
	type_to_asset[NFTFrameStyles.NFT_GOLD_CARVED] = preload("res://assets/nftshape/Gold_Carved.glb")
	type_to_asset[NFTFrameStyles.NFT_GOLD_WIDE] = preload("res://assets/nftshape/Gold_Wide.glb")
	type_to_asset[NFTFrameStyles.NFT_GOLD_ROUNDED] = preload(
		"res://assets/nftshape/Gold_Rounded.glb"
	)
	type_to_asset[NFTFrameStyles.NFT_METAL_MEDIUM] = preload(
		"res://assets/nftshape/Metal_Medium.glb"
	)
	type_to_asset[NFTFrameStyles.NFT_METAL_WIDE] = preload("res://assets/nftshape/Metal_Wide.glb")
	type_to_asset[NFTFrameStyles.NFT_METAL_SLIM] = preload("res://assets/nftshape/Metal_Slim.glb")
	type_to_asset[NFTFrameStyles.NFT_METAL_ROUNDED] = preload(
		"res://assets/nftshape/Metal_Rounded.glb"
	)
	type_to_asset[NFTFrameStyles.NFT_PINS] = preload("res://assets/nftshape/Pins.glb")
	type_to_asset[NFTFrameStyles.NFT_MINIMAL_BLACK] = preload(
		"res://assets/nftshape/Minimal_Black.glb"
	)
	type_to_asset[NFTFrameStyles.NFT_MINIMAL_WHITE] = preload(
		"res://assets/nftshape/Minimal_White.glb"
	)
	type_to_asset[NFTFrameStyles.NFT_TAPE] = preload("res://assets/nftshape/Tape.glb")
	type_to_asset[NFTFrameStyles.NFT_WOOD_SLIM] = preload("res://assets/nftshape/Wood_Slim.glb")
	type_to_asset[NFTFrameStyles.NFT_WOOD_WIDE] = preload("res://assets/nftshape/Wood_Wide.glb")
	type_to_asset[NFTFrameStyles.NFT_WOOD_TWIGS] = preload("res://assets/nftshape/Wood_Twigs.glb")
	type_to_asset[NFTFrameStyles.NFT_CANVAS] = preload("res://assets/nftshape/Canvas.glb")
	type_to_asset[NFTFrameStyles.NFT_NONE] = preload("res://assets/nftshape/empty_frame.tscn")


func instantiate(type: NFTFrameStyles):
	var resource: Resource = type_to_asset[type]
	if resource == null:
		return null

	var node: Node3D = resource.instantiate()
	if type == NFTFrameStyles.NFT_CLASSIC:
		node.rotate_x(-deg_to_rad(90.0))
		node.rotate_y(deg_to_rad(180.0))
	return node
