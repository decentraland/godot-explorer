class_name MusicPlayer
extends Node

var _current_music: String = ""

var _last_music: String = ""

@onready var _audio_stream: AudioStreamPlayer = AudioStreamPlayer.new()


func _ready():
	_audio_stream.bus = &"Music"
	add_child(_audio_stream)


func play(music_name: StringName):
	var new_stream = load("res://assets/sfx/ambient/%s.ogg" % music_name)
	if is_instance_valid(new_stream):
		if not _current_music.is_empty():
			_last_music = _current_music

		_audio_stream.stream = new_stream
		_audio_stream.play()
		_current_music = music_name


func restore_music():
	play(_last_music)


func stop():
	_last_music = _current_music
	_current_music = ""
	_audio_stream.stop()


func get_current_music() -> String:
	return _current_music
