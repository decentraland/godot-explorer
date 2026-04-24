extends Node3D

var nft_shape = preload("res://src/decentraland_components/nft_shape.tscn")
var nft_fetcher: OpenSeaFetcher = OpenSeaFetcher.new()


# Called when the node enters the scene tree for the first time.
# gdlint:ignore = async-function-name
func _ready():
	var nft: NftShape = nft_shape.instantiate()
	var urn = DclUrn.new(
		"urn:decentraland:ethereum:erc721:0x06012c8cf97bead5deae237070f9587f8e7a266d:558536"
	)
	var promise = nft_fetcher.fetch_nft(urn)
	var asset: OpenSeaFetcher.Asset = await PromiseUtils.async_awaiter(promise)
	nft.set_opensea_nft(
		NftFrameStyleLoader.NFTFrameStyles.NFT_GOLD_EDGES, asset, Color(1.0, 1.0, 1.0)
	)
	add_child(nft)
