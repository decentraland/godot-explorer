extends CarrouselGenerator

const DISCOVER_CARROUSEL_ITEM = preload(
	"res://src/ui/components/discover/carrousel/discover_carrousel_item.tscn"
)


func add_item(item_data: Dictionary):
	var item = DISCOVER_CARROUSEL_ITEM.instantiate()
	item_container.add_child(item)

	item.set_data(item_data)
	item.item_pressed.connect(discover.on_item_pressed)


func on_request(_offset: int, _limit: int) -> void:
	add_item(
		{
			"title": "Goerli Plaza ZONE",
			"description": "SDK7 Scenes for testing",
			"image": "https://i.imgur.com/uuWymQh.png",
			"base_position": "72,-10",
			"contact_name": "SDK Team",
			"world": true,
			"world_name": "https://sdk-test-scenes.decentraland.zone"
		}
	)

	add_item(
		{
			"title": "Goerli Plaza IPFS",
			"description": "SDK7 Scenes for testing",
			"base_position": "72,-10",
			"contact_name": "SDK Team",
			"image": "https://i.imgur.com/Zsl1r2d.png",
			"world": true,
			"world_name": "https://sdk-team-cdn.decentraland.org/ipfs/goerli-plaza-main"
		}
	)

	add_item(
		{
			"title": "Mannakia World",
			"description": "Explore what a guy from Neuquen, Argentina can do!",
			"base_position": "0,0",
			"contact_name": "Mannakia",
			"image": "https://i.imgur.com/BpdERb4.png",
			"world": true,
			"world_name": "mannakia.dcl.eth"
		}
	)

	set_consumer_visible.emit(true)
