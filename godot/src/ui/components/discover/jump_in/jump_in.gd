extends PlaceItem

signal jump_in(position: Vector2i, realm: String)

@export var location: Vector2i = Vector2i(0, 0)

@export var realm: String = Realm.MAIN_REALM

@export var realm_title: String = "Genesis City"

@onready var label_location := %Label_Location

@onready var label_realm := %Label_Realm

@onready var label_creator := %Label_Creator

@onready var container_creator := %HBoxContainer_Creator

@onready var panel_jump_in: PanelContainer = %PanelJumpIn

func _ready():
	super()
	set_location(location)


func set_location(_location: Vector2i):
	location = _location
	label_location.text = "%s, %s" % [_location.x, _location.y]


func set_realm(_realm: String, _realm_title: String):
	label_realm.text = _realm_title
	realm = _realm


func set_creator(_creator: String):
	container_creator.visible = not _creator.is_empty()
	label_creator.text = _creator


func set_data(item_data):
	super(item_data)

	var location_vector = item_data.get("base_position", "0,0").split(",")
	if location_vector.size() == 2:
		set_location(Vector2i(int(location_vector[0]), int(location_vector[1])))

	set_creator(_get_or_empty_string(item_data, "contact_name"))

	var world = item_data.get("world", false)
	if world:
		var world_name = item_data.get("world_name")
		set_realm(world_name, world_name)
	else:
		set_realm(Realm.MAIN_REALM, "Genesis City")


func _on_button_jump_in_pressed():
	jump_in.emit(location, realm)


func _on_gui_input(event):
	if event is InputEventScreenTouch:
		if !event.pressed:
			self.hide()
			UiSounds.play_sound("mainmenu_widget_close")


func _on_visibility_changed() -> void:
	if visible and panel_jump_in != null:
		var _animation_target_y = panel_jump_in.position.y
		# Place the menu off-screen above (its height above the target position)
		panel_jump_in.position.y = panel_jump_in.position.y + panel_jump_in.size.y
		
		create_tween().tween_property(panel_jump_in, "position:y", _animation_target_y, 0.5).set_trans(Tween.TRANS_SINE).set_ease(Tween.EASE_OUT)
