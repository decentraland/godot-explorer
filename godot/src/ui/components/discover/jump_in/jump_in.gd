extends PlaceItem

signal jump_in(position: Vector2i, realm: String)

@onready var label_location := %Label_Location

@onready var label_realm := %Label_Realm

@export var location: Vector2i = Vector2i(0, 0)

@export var realm: String = "https://realm-provider.decentraland.org/main"

@export var realm_title: String = "Genesis City"

func _ready():
	super()
	set_location(location)


func set_location(_location: Vector2i):
	location = _location
	label_location.text = "%s, %s" % [_location.x, _location.y]
	
func set_realm(_realm: String, _realm_title: String):
	label_realm.text = realm_title
	realm = _realm
	
func set_data(item_data):
	super(item_data)
	
	var location_vector = item_data.get("base_position", "0,0").split(",")
	if location_vector.size() == 2:
		set_location(Vector2i(int(location_vector[0]), int(location_vector[1])))
	
	var world = item_data.get("world", false)
	if world:
		var world_name = item_data.get("world_name")
		set_realm(world_name, world_name)
	

func _on_button_jump_in_pressed():
	jump_in.emit(location, realm)
