extends DclAudioStream

## Audio-only sibling of `video_player.gd`. It drives the very same native
## players (ExoPlayer on Android, AVPlayer on iOS) with the video surface left
## out: `init_texture()` is the only video-specific call, and source/volume/
## playback control and state polling never touch it.

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
	stream_state = STREAM_STATE_LOADING

	native_player = load(scene_path).instantiate()
	add_child(native_player)

	await get_tree().process_frame

	# No init_texture() here: that call is the video surface, and nothing else
	# in the wrapper depends on it.
	if not (_source.begins_with("http://") or _source.begins_with("https://")):
		push_error("AudioStream: only remote sources are supported, got ", _source)
		stream_state = STREAM_STATE_ERROR
		_free_native_player()
		return

	if not native_player.set_source_url(_source):
		push_error("AudioStream: failed to set source ", _source)
		stream_state = STREAM_STATE_ERROR
		_free_native_player()
		return

	native_player.set_looping(false)

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


func _update_stream_state():
	var duration: float = native_player.get_duration()
	var is_native_playing: bool = native_player.is_playing()

	if is_native_playing:
		stream_state = STREAM_STATE_PLAYING
	elif duration <= 0:
		stream_state = STREAM_STATE_LOADING
	elif _is_playing:
		stream_state = STREAM_STATE_READY
	else:
		stream_state = STREAM_STATE_PAUSED


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
	_free_native_player()
	current_backend = BackendType.NOOP
	_source = ""
	_is_playing = false
	stream_state = STREAM_STATE_NONE
	_last_effective_volume = -1.0
	_pending_play_state = -1
