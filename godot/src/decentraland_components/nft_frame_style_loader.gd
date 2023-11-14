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
	type_to_asset[NFTFrameStyles.NFT_BAROQUE_ORNAMENT] = preload("res://assets/nftshape/Barroque_01.glb")
	type_to_asset[NFTFrameStyles.NFT_DIAMOND_ORNAMENT] = preload("res://assets/nftshape/Barroque_02.glb")
	type_to_asset[NFTFrameStyles.NFT_MINIMAL_WIDE] = preload("res://assets/nftshape/Basic_01.glb")
	type_to_asset[NFTFrameStyles.NFT_MINIMAL_GREY] = preload("res://assets/nftshape/Basic_02.glb")
	type_to_asset[NFTFrameStyles.NFT_BLOCKY] = preload("res://assets/nftshape/Blocky_01.glb")
	type_to_asset[NFTFrameStyles.NFT_GOLD_EDGES] = preload("res://assets/nftshape/Golden_01.glb")
	type_to_asset[NFTFrameStyles.NFT_GOLD_CARVED] = preload("res://assets/nftshape/Golden_02.glb")
	type_to_asset[NFTFrameStyles.NFT_GOLD_WIDE] = preload("res://assets/nftshape/Golden_03.glb")
	type_to_asset[NFTFrameStyles.NFT_GOLD_ROUNDED] = preload("res://assets/nftshape/Golden_04.glb")
	type_to_asset[NFTFrameStyles.NFT_METAL_MEDIUM] = preload("res://assets/nftshape/Metal_01.glb")
	type_to_asset[NFTFrameStyles.NFT_METAL_WIDE] = preload("res://assets/nftshape/Metal_02.glb")
	type_to_asset[NFTFrameStyles.NFT_METAL_SLIM] = preload("res://assets/nftshape/Metal_03.glb")
	type_to_asset[NFTFrameStyles.NFT_METAL_ROUNDED] = preload("res://assets/nftshape/Metal_04.glb")
	type_to_asset[NFTFrameStyles.NFT_PINS] = preload("res://assets/nftshape/Pin.glb")
	type_to_asset[NFTFrameStyles.NFT_MINIMAL_BLACK] = preload("res://assets/nftshape/SimpleBlack.glb")
	type_to_asset[NFTFrameStyles.NFT_MINIMAL_WHITE] = preload("res://assets/nftshape/SimpleWhite.glb")
	type_to_asset[NFTFrameStyles.NFT_TAPE] = preload("res://assets/nftshape/Tapper.glb")
	type_to_asset[NFTFrameStyles.NFT_WOOD_SLIM] = preload("res://assets/nftshape/Wood.glb")
	type_to_asset[NFTFrameStyles.NFT_WOOD_WIDE] = preload("res://assets/nftshape/Wood_02.glb")
	type_to_asset[NFTFrameStyles.NFT_WOOD_TWIGS] = preload("res://assets/nftshape/WoodSticks.glb")
	type_to_asset[NFTFrameStyles.NFT_CANVAS] = preload("res://assets/nftshape/SimpleCanvas.glb")
	type_to_asset[NFTFrameStyles.NFT_NONE] = null # Asumiendo que no hay activo para NFT_NONE
