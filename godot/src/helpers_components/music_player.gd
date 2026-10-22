class_name MusicPlayer
extends Node

var _current_music: String = ""

var _last_music: String = ""

# True while _current_music names a stream fetched at runtime (play_stream()) rather than a
# baked res://assets/sfx/ambient/ resource — stop()'s _last_music bookkeeping must never copy
# one of these names, or a later restore_music() tries to load() a resource that doesn't exist.
# restore_music() has no caller yet (discover.gd hand-rolls its own restore instead, since it
# also needs "stop if nothing was playing before"), so this is defending a path nothing
# exercises today — kept because the next caller of restore_music() shouldn't have to rediscover
# why a dynamic stream's name can't go through play().
var _current_is_dynamic := false

@onready var _audio_stream: AudioStreamPlayer = AudioStreamPlayer.new()


func _ready():
	_audio_stream.bus = &"Music"
	add_child(_audio_stream)


func play(music_name: StringName):
	if !is_instance_valid(_audio_stream):
		return

	var new_stream = load("res://assets/sfx/ambient/%s.ogg" % music_name)
	if is_instance_valid(new_stream):
		if not _current_music.is_empty() and not _current_is_dynamic:
			_last_music = _current_music

		_audio_stream.stream = new_stream
		_audio_stream.play()
		_current_music = music_name
		_current_is_dynamic = false


func restore_music():
	play(_last_music)


## Plays a stream fetched at runtime (e.g. a seasonal track) instead of a baked
## res://assets/sfx/ambient/ resource — play()/restore_music() only know how to load() the
## latter by name, so a dynamic stream goes through this instead, sharing the same
## AudioStreamPlayer/bus and _last_music bookkeeping.
func play_stream(stream: AudioStream, stream_name: String) -> void:
	if !is_instance_valid(_audio_stream) or !is_instance_valid(stream):
		return
	if not _current_music.is_empty() and not _current_is_dynamic:
		_last_music = _current_music
	_audio_stream.stream = stream
	_audio_stream.play()
	_current_music = stream_name
	_current_is_dynamic = true


func stop():
	if not _current_is_dynamic:
		_last_music = _current_music
	_current_music = ""
	_current_is_dynamic = false
	_audio_stream.stop()


func get_current_music() -> String:
	return _current_music
