extends Button

@onready var label_coords = %Label_Coords
@onready var label_scene_title = %Label_SceneTitle


func _ready():
	Global.change_parcel.connect(self._on_change_parcel)
	Global.scene_runner.on_change_scene_id.connect(self._on_change_scene_id)


func _on_change_parcel(position: Vector2i):
	label_coords.text = "%d,%d" % [position.x, position.y]


func _on_change_scene_id(scene_id: int):
	var scene_data = Global.scene_fetcher.get_scene_data(scene_id)
	var scene_title = scene_data.get("entity", {}).get("metadata", {}).get("display", {}).get("title", "Unknown")
	label_scene_title.text = scene_title

