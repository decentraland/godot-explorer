extends Button

@onready var label_coords = %Label_Coords
@onready var label_scene_title = %Label_SceneTitle
@onready var texture_rect_sdk6 = %TextureRect_Sdk6


func _ready():
	texture_rect_sdk6.hide()
	Global.change_parcel.connect(self._on_change_parcel)
	Global.scene_runner.on_change_scene_id.connect(self._on_change_scene_id)


func _on_change_parcel(_position: Vector2i):
	label_coords.text = "%d,%d" % [_position.x, _position.y]


func _on_change_scene_id(scene_id: int):
	texture_rect_sdk6.hide()
	if scene_id == -1:
		label_scene_title.text = ""
		label_scene_title.hide()
		return

	var scene = Global.scene_fetcher.get_scene_data_by_scene_id(scene_id)
	if scene != null:
		texture_rect_sdk6.set_visible(not scene.scene_entity_definition.is_sdk7())
		label_scene_title.text = scene.scene_entity_definition.get_title()
		label_scene_title.show()
	else:
		label_scene_title.text = ""
		label_scene_title.hide()
