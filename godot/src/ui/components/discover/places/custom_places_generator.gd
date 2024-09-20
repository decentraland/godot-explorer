class_name CustomPlacesGenerator
extends CarrouselGenerator

const CUSTOM_PLACES: Array[Dictionary] = [
	{
		"title": "Custom Rat Scape",
		"description":
		"Exodus Town is an experiment on Decentraland Worlds, continuous issuance, and DAOs. Originating from the 0,0 coordinate, it expands in a never-ending spiral, growing one parcel per day, forever. Every 24 hours, a TOWN token is auctioned off in exchange for MANA, granting the holder the ability to publish content to Exodus Town. Importantly, all auction proceeds flow directly into the on-chain governed Exodus DAO, steered exclusively by TOWN token holders.",
		"image":
		"https://worlds-content-server.decentraland.org/contents/bafybeidm7nyllf7d5j4nvdrtvggk2nyizvwtccqzvpztc7fhmyjxsaouza",
		"base_position": "0,0",
		"contact_name": "SDK",
		"world": true,
		"world_name": "kuruk.dcl.eth"
	},
	{
		"title": "SDK7 Unity Cafe",
		"description": "Dancing, Music, Networking, Partying, Social!",
		"base_position": "16,102",
		"contact_name": "Carl Fravel",
		"image":
		"https://peer.decentraland.org/content/contents/bafybeibixxqiejsizqvf2rs6z6l4wt6q3mjlzuzabvjbkxxl57qusnz4ii",
	},
	{
		"title": "Exodus Town",
		"description":
		"Exodus Town is an experiment on Decentraland Worlds, continuous issuance, and DAOs. Originating from the 0,0 coordinate, it expands in a never-ending spiral, growing one parcel per day, forever. Every 24 hours, a TOWN token is auctioned off in exchange for MANA, granting the holder the ability to publish content to Exodus Town. Importantly, all auction proceeds flow directly into the on-chain governed Exodus DAO, steered exclusively by TOWN token holders.",
		"image": "https://i.ibb.co/9s704vF/exodus-Town-Rings.jpg",
		"base_position": "0,0",
		"contact_name": "ExodusTown",
		"world": true,
		"world_name": "https://exodus.town/"
	},
	{
		"title": "Goerli Plaza",
		"description": "SDK7 Scenes for testing",
		"base_position": "72,-10",
		"contact_name": "SDK Team",
		"image": "https://i.imgur.com/Zsl1r2d.png",
		"world": true,
		"world_name": "https://sdk-team-cdn.decentraland.org/ipfs/goerli-plaza-main-latest/"
	},
	{
		"title": "In World Builder",
		"description": "SDK7 Scene for In World Builder",
		"base_position": "0,0",
		"contact_name": "In World Builder",
		"image": "https://pbs.twimg.com/profile_images/1715100604002193408/y8HcKT6j_400x400.jpg",
		"world": true,
		"world_name": "https://worlds.dcl-iwb.co/world/BuilderWorld.dcl.eth/"
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

	if CUSTOM_PLACES.is_empty():
		report_loading_status.emit(CarrouselGenerator.LoadingStatus.OkWithoutResults)
	else:
		report_loading_status.emit(CarrouselGenerator.LoadingStatus.OkWithResults)
