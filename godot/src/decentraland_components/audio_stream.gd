extends DclAudioStream

## Audio-only sibling of `video_player.gd`. It drives the very same native
## players (ExoPlayer on Android, AVPlayer on iOS) with the video surface left
## out: `init_texture()` is the only video-specific call we make, and source/
## volume/playback control and state polling never touch it. The wrapper also
## drives the surface from its own `_process`, which is why we disable it.

enum BackendType { EXO_PLAYER = 1, AV_PLAYER = 2, NOOP = 3 }

# Stream state constants, matching the Rust STREAM_STATE_* values.
const STREAM_STATE_NONE = 0
const STREAM_STATE_LOADING = 1
const STREAM_STATE_READY = 2
const STREAM_STATE_PLAYING = 3
const STREAM_STATE_PAUSED = 6
const STREAM_STATE_ERROR = 7

const PLAY_PAUSE_DEBOUNCE_MS: float = 100.0

var current_backend: BackendType = BackendType.NOOP
var native_player: Node = null
var _source: String = ""
var _is_playing: bool = false
var _last_effective_volume: float = -1.0
var _last_play_pause_time: float = 0.0
var _pending_play_state: int = -1  # -1=none, 0=pause, 1=play
# Bumped by `_backend_dispose()` so an in-flight `_async_init_native_backend`
# can tell it was superseded while awaiting a frame.
var _init_epoch: int = 0
# The native player only carries a state worth reporting once it has a source.
var _source_ready: bool = false
# Separates "paused" from "never started", which the native player cannot.
var _has_played: bool = false


# Called from Rust DclAudioStream::init_backend
func _init_backend_impl(backend_type: int, source: String, playing: bool):
	_backend_dispose()

	current_backend = backend_type as BackendType
	_source = source
	_is_playing = playing

	match current_backend:
		BackendType.EXO_PLAYER:
			_async_init_native_backend("res://src/decentraland_components/exo_player.tscn")
		BackendType.AV_PLAYER:
			_async_init_native_backend("res://src/decentraland_components/av_player.tscn")
		_:
			_init_noop_backend()


func _async_init_native_backend(scene_path: String):
	var epoch := _init_epoch
	stream_state = STREAM_STATE_LOADING

	native_player = load(scene_path).instantiate()
	# The wrapper's own `_process` calls `update_texture()`, which polls the
	# plugin every frame and allocates a video surface as soon as the source
	# reports a video size — a scene can point `AudioStream.url` at an HLS/MP4.
	# Nothing here renders that texture, so keep the wrapper idle.
	native_player.set_process(false)
	add_child(native_player)

	await get_tree().process_frame

	# `_backend_dispose()` — entity removal, or a second `url` on the same
	# entity — frees the player and bumps the epoch. Either way this coroutine
	# no longer owns `native_player` and must not drive it.
	if epoch != _init_epoch or not is_instance_valid(native_player):
		return

	# No init_texture() here: that call is the video surface, and nothing else
	# in the wrapper depends on it.
	if not (_source.begins_with("http://") or _source.begins_with("https://")):
		push_warning("AudioStream: only remote sources are supported, got ", _source)
		stream_state = STREAM_STATE_ERROR
		_free_native_player()
		return

	if not native_player.set_source_url(_source):
		push_warning("AudioStream: failed to set source ", _source)
		stream_state = STREAM_STATE_ERROR
		_free_native_player()
		return

	native_player.set_looping(false)
	_source_ready = true

	if _is_playing:
		native_player.play()


func _init_noop_backend():
	stream_state = STREAM_STATE_NONE


func _free_native_player():
	if native_player:
		native_player.queue_free()
		native_player = null


func _process(_delta):
	if current_backend == BackendType.NOOP or not native_player:
		return

	_process_pending_play_state()
	_update_volume()
	_update_stream_state()


func _process_pending_play_state():
	if _pending_play_state == -1:
		return
	if Time.get_ticks_msec() - _last_play_pause_time < PLAY_PAUSE_DEBOUNCE_MS:
		return

	var pending := _pending_play_state
	_pending_play_state = -1
	_last_play_pause_time = Time.get_ticks_msec()
	if pending == 1:
		_apply_play()
	else:
		_apply_pause()


## Native players bypass Godot's buses, so every level is applied here — the
## same product `video_player.gd` uses, which already matches Unity.
func _calculate_effective_volume() -> float:
	if dcl_muted:
		return 0.0
	var config = Global.get_config()
	var master_volume: float = config.audio_general_volume / 100.0
	var scene_volume: float = config.audio_scene_volume / 100.0
	return master_volume * scene_volume * dcl_volume


func _update_volume():
	var effective_volume: float = _calculate_effective_volume()
	if absf(effective_volume - _last_effective_volume) < 0.001:
		return

	_last_effective_volume = effective_volume
	native_player.set_volume(effective_volume)


## `get_duration()` cannot separate loading from stopped: a live stream —
## Icecast/SHOUTcast, endless HLS, the main use case for this component —
## reports 0 for its whole life, so a duration test reads it as loading
## forever and it never reaches `STREAM_STATE_PAUSED`. What we asked the
## player for (`_is_playing`) tells the two apart on every kind of source.
func _update_stream_state():
	if not _source_ready:
		return

	if native_player.is_playing():
		_has_played = true
		stream_state = STREAM_STATE_PLAYING
	elif _is_playing:
		# Asked to play and the player has not started: still buffering.
		stream_state = STREAM_STATE_LOADING
	elif _has_played:
		stream_state = STREAM_STATE_PAUSED
	else:
		stream_state = STREAM_STATE_READY


func _backend_play():
	var now := Time.get_ticks_msec()
	if now - _last_play_pause_time < PLAY_PAUSE_DEBOUNCE_MS:
		_pending_play_state = 1
		return

	_last_play_pause_time = now
	_pending_play_state = -1
	_apply_play()


func _apply_play():
	_is_playing = true
	_last_effective_volume = -1.0
	if native_player:
		native_player.play()


func _backend_pause():
	var now := Time.get_ticks_msec()
	if now - _last_play_pause_time < PLAY_PAUSE_DEBOUNCE_MS:
		_pending_play_state = 0
		return

	_last_play_pause_time = now
	_pending_play_state = -1
	_apply_pause()


func _apply_pause():
	_is_playing = false
	_last_effective_volume = -1.0
	if native_player:
		native_player.pause()


func _backend_dispose():
	_init_epoch += 1
	_free_native_player()
	current_backend = BackendType.NOOP
	_source = ""
	_is_playing = false
	_source_ready = false
	_has_played = false
	stream_state = STREAM_STATE_NONE
	_last_effective_volume = -1.0
	_pending_play_state = -1
