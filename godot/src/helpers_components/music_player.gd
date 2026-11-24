class_name MusicPlayer
extends Node

## The lobby's track, the only music Discover plays (besides a seasonal track of its own).
const LOBBY_MUSIC := &"music_builder"
const _SCENE_BUS := &"Scene"

var _current_music: String = ""

# While Discover shows, the world is not heard: the lobby track replaces the world's music and the
# scenes' audio bus is muted. What the world asks to play meanwhile (the empty-parcel ambience
# when the player's scene changes) waits here for Discover to go.
var _in_discover := false
var _music_after_discover: String = ""

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
	if _in_discover and music_name != LOBBY_MUSIC:
		_music_after_discover = music_name
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


## Discover is showing: only the lobby track plays over it, whether the world is still up behind
## it (opened in-world) or being torn down (back from the world).
func enter_discover() -> void:
	if _in_discover:
		return
	_music_after_discover = "" if _current_is_dynamic else _current_music
	_in_discover = true
	_set_scene_audio_muted(true)
	if _current_music != LOBBY_MUSIC:
		play(LOBBY_MUSIC)


## Discover is gone: the scenes are heard again and the music from before it (or what the world
## asked for meanwhile) resumes, or nothing when there was none.
func exit_discover() -> void:
	if not _in_discover:
		return
	_in_discover = false
	_set_scene_audio_muted(false)
	var music := _music_after_discover
	_music_after_discover = ""
	if music == _current_music:
		return
	if music.is_empty():
		stop()
	else:
		play(music)


func _set_scene_audio_muted(muted: bool) -> void:
	var bus := AudioServer.get_bus_index(_SCENE_BUS)
	if bus >= 0:
		AudioServer.set_bus_mute(bus, muted)
