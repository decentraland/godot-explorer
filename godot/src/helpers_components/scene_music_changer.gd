extends Node

const _RANDOM_MUSICS = [
	"ambience_woods_day_nowind01",
	"ambience_woods_day_nowind02",
	"ambience_woods_day_nowind03",
]


# Called when the node enters the scene tree for the first time.
func _ready():
	Global.scene_runner.on_change_scene_id.connect(self._on_change_scene_id)


func _on_change_scene_id(scene_id: int):
	if scene_id == -1:  # Empty parcel
		Global.music_player.play(_RANDOM_MUSICS.pick_random())
	else:
		var current_music = Global.music_player.get_current_music()
		if _RANDOM_MUSICS.has(current_music):
			Global.music_player.stop()  # We dont play music on an empty parcel
