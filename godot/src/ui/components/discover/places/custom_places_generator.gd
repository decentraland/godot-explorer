class_name CustomPlacesGenerator
extends CarrouselGenerator

const CUSTOM_PLACES: Array[Dictionary] = [
	{
		"title": "Genesis Plaza",
		"description":
		"Jump in to strike up a chat with other visitors, retake the commands tutorial with a cute floating robot, or dive into the swirling portal to get to Decentraland's visitor center.",
		"base_position": "0,0",
		"contact_name": "Decentraland Foundation",
		"image":
		"https://peer.decentraland.org/content/contents/bafybeientwwufhkn6yyfgehqqg64rp2u4pxkqfwsgfj2kldvarretredlm",
	},
	{
		"title": "Exodus Town",
		"description":
		"Exodus Town is an experiment on Decentraland Worlds, continuous issuance, and DAOs. Originating from the 0,0 coordinate, it expands in a never-ending spiral, growing one parcel per day, forever. Every 24 hours, a TOWN token is auctioned off in exchange for MANA, granting the holder the ability to publish content to Exodus Town. Importantly, all auction proceeds flow directly into the on-chain governed Exodus DAO, steered exclusively by TOWN token holders.",
		"image": "https://i.ibb.co/9s704vF/exodus-Town-Rings.jpg",
		"base_position": "0,0",
		"contact_name": "ExodusTown",
		"world": true,
		"world_name": "https://exodus.town/city-loader/"
	},
	{
		"title": "Goerli Plaza ZONE",
		"description": "SDK7 Scenes for testing",
		"image": "https://i.imgur.com/uuWymQh.png",
		"base_position": "72,-10",
		"contact_name": "SDK Team",
		"world": true,
		"world_name": "https://sdk-test-scenes.decentraland.zone"
	},
	{
		"title": "Goerli Plaza IPFS",
		"description": "SDK7 Scenes for testing",
		"base_position": "72,-10",
		"contact_name": "SDK Team",
		"image": "https://i.imgur.com/Zsl1r2d.png",
		"world": true,
		"world_name":
		"https://sdk-team-cdn.decentraland.org/ipfs/goerli-plaza-update-asset-pack-lib"
	}
]

const DISCOVER_CARROUSEL_ITEM = preload(
	"res://src/ui/components/discover/carrousel/discover_carrousel_item.tscn"
)


func add_item(item_data: Dictionary):
	var item = DISCOVER_CARROUSEL_ITEM.instantiate()
	item_container.add_child(item)

	item.set_data(item_data)
	item.item_pressed.connect(discover.on_item_pressed)


func on_request(_offset: int, _limit: int) -> void:
	for custom_place in CUSTOM_PLACES:
		add_item(custom_place)

	set_consumer_visible.emit(true)
