extends DclVideoPlayer

## Unified video player that handles multiple backends:
## - LiveKit: Streaming video from livekit-video:// URLs
## - ExoPlayer: Android video playback with GPU acceleration
## - AVPlayer: iOS video playback (future)
## - Noop: Fallback when no backend is available

enum BackendType { LIVEKIT = 0, EXO_PLAYER = 1, AV_PLAYER = 2, NOOP = 3 }

# Video state constants (matching Rust VIDEO_STATE_* constants)
const VIDEO_STATE_NONE = 0
const VIDEO_STATE_LOADING = 1
const VIDEO_STATE_READY = 2
const VIDEO_STATE_PLAYING = 3
const VIDEO_STATE_BUFFERING = 4
const VIDEO_STATE_SEEKING = 5
const VIDEO_STATE_PAUSED = 6
const VIDEO_STATE_ERROR = 7

var current_backend: BackendType = BackendType.NOOP
var exo_player: Node = null  # ExoPlayer child node when using ExoPlayer backend
var av_player: Node = null  # AVPlayer child node when using AVPlayer backend
var _source: String = ""
var _is_playing: bool = false
var _is_looping: bool = false

# Volume tracking for efficient updates
var _last_effective_volume: float = -1.0  # -1 means uninitialized


# Called from Rust DclVideoPlayer::init_backend
func _init_backend_impl(backend_type: int, source: String, playing: bool, looping: bool):
	# Clean up previous backend if any
	_backend_dispose()

	current_backend = backend_type as BackendType
	_source = source
	_is_playing = playing
	_is_looping = looping

	match current_backend:
		BackendType.LIVEKIT:
			_init_livekit_backend()
		BackendType.EXO_PLAYER:
			_async_init_exo_player_backend()
		BackendType.AV_PLAYER:
			_init_av_player_backend()
		_:
			_init_noop_backend()


func _init_livekit_backend():
	print("VideoPlayer: Initializing LiveKit backend for ", _source)

	# Set initial state to loading (will be updated to PLAYING when frames arrive)
	video_state = VIDEO_STATE_LOADING

	# LiveKit backend uses AudioStreamGenerator for audio
	# Video frames are pushed directly to texture from Rust
	var audio_stream_generator = AudioStreamGenerator.new()
	audio_stream_generator.mix_rate = 48000.0
	audio_stream_generator.buffer_length = 1.5
	self.set_stream(audio_stream_generator)
	self.play()


func _async_init_exo_player_backend():
	if OS.get_name() != "Android":
		push_warning("ExoPlayer backend only available on Android, falling back to Noop")
		current_backend = BackendType.NOOP
		_init_noop_backend()
		return

	print("VideoPlayer: Initializing ExoPlayer backend for ", _source)

	# Set initial state to loading
	video_state = VIDEO_STATE_LOADING

	# Create ExoPlayer child node
	var exo_player_scene = load("res://src/decentraland_components/exo_player.tscn")
	exo_player = exo_player_scene.instantiate()
	add_child(exo_player)

	# Wait for ExoPlayer to be ready
	await get_tree().process_frame

	# Initialize texture with initial size (will be resized when video loads)
	if not exo_player.init_texture(640, 360):
		push_error("VideoPlayer: Failed to initialize ExoPlayer texture")
		video_state = VIDEO_STATE_ERROR
		return

	# Set the video source
	var success: bool
	if _source.begins_with("http://") or _source.begins_with("https://"):
		success = exo_player.set_source_url(_source)
	else:
		# For local files, we need to fetch them first
		await _async_fetch_and_set_local_source()
		return

	if not success:
		push_error("VideoPlayer: Failed to set ExoPlayer source: ", _source)
		video_state = VIDEO_STATE_ERROR
		return

	exo_player.set_looping(_is_looping)

	if _is_playing:
		exo_player.play()


func _async_fetch_and_set_local_source():
	# This is a local file reference, need to fetch it first
	var content_mapping := Global.scene_runner.get_scene_content_mapping(dcl_scene_id)
	var file_hash = content_mapping.get_hash(_source)

	if file_hash.is_empty():
		push_error("VideoPlayer: Could not find hash for local file: ", _source)
		video_state = VIDEO_STATE_ERROR
		return

	var promise = Global.content_provider.fetch_video(file_hash, content_mapping)
	var res = await PromiseUtils.async_awaiter(promise)
	if res is PromiseError:
		printerr("VideoPlayer: Error fetching video: ", res.get_error())
		video_state = VIDEO_STATE_ERROR
		return

	var local_video_path = "user://content/" + file_hash
	var absolute_file_path = local_video_path.replace("user:/", OS.get_user_data_dir())

	if exo_player:
		var success = exo_player.set_source_local(absolute_file_path)
		if not success:
			push_error("VideoPlayer: Failed to set ExoPlayer local source: ", absolute_file_path)
			video_state = VIDEO_STATE_ERROR
			return

		exo_player.set_looping(_is_looping)
		if _is_playing:
			exo_player.play()


func _async_fetch_and_set_local_source_av_player():
	# This is a local file reference, need to fetch it first
	var content_mapping := Global.scene_runner.get_scene_content_mapping(dcl_scene_id)
	var file_hash = content_mapping.get_hash(_source)

	if file_hash.is_empty():
		push_error("VideoPlayer: Could not find hash for local file: ", _source)
		video_state = VIDEO_STATE_ERROR
		return

	var promise = Global.content_provider.fetch_video(file_hash, content_mapping)
	var res = await PromiseUtils.async_awaiter(promise)
	if res is PromiseError:
		printerr("VideoPlayer: Error fetching video: ", res.get_error())
		video_state = VIDEO_STATE_ERROR
		return

	# Get the file extension from the original source (AVPlayer needs it to determine format)
	var extension = _source.get_extension()
	var local_video_path = "user://content/" + file_hash
	var absolute_file_path = local_video_path.replace("user:/", OS.get_user_data_dir())

	# If the source has an extension, create a copy with extension for AVPlayer
	# AVPlayer needs the file extension to determine the video format
	# TODO: Optimize by using hard links or storing with extension originally
	if not extension.is_empty():
		var path_with_ext = absolute_file_path + "." + extension
		var user_path_with_ext = "user://content/" + file_hash + "." + extension
		# Check if we need to create a copy with extension
		if FileAccess.file_exists(user_path_with_ext):
			absolute_file_path = path_with_ext
		else:
			var err = DirAccess.copy_absolute(absolute_file_path, path_with_ext)
			if err == OK:
				print("VideoPlayer: Created file copy with extension: ", path_with_ext)
				absolute_file_path = path_with_ext
			else:
				push_warning(
					"VideoPlayer: Failed to create file with extension, trying without: ", err
				)

	print("VideoPlayer: AVPlayer local source path: ", absolute_file_path)

	if av_player:
		var success = av_player.set_source_local(absolute_file_path)
		if not success:
			push_error("VideoPlayer: Failed to set AVPlayer local source: ", absolute_file_path)
			video_state = VIDEO_STATE_ERROR
			return

		av_player.set_looping(_is_looping)
		if _is_playing:
			av_player.play()


func _init_av_player_backend():
	if OS.get_name() != "iOS":
		push_warning("AVPlayer backend only available on iOS, falling back to Noop")
		current_backend = BackendType.NOOP
		_init_noop_backend()
		return

	print("VideoPlayer: Initializing AVPlayer backend for ", _source)

	# Set initial state to loading
	video_state = VIDEO_STATE_LOADING

	# Create AVPlayer child node
	var av_player_scene = load("res://src/decentraland_components/av_player.tscn")
	av_player = av_player_scene.instantiate()
	add_child(av_player)

	# Wait for AVPlayer to be ready
	await get_tree().process_frame

	# Initialize texture with initial size (will be resized when video loads)
	if not av_player.init_texture(640, 360):
		push_error("VideoPlayer: Failed to initialize AVPlayer texture")
		video_state = VIDEO_STATE_ERROR
		return

	# Set the video source
	var success: bool
	if _source.begins_with("http://") or _source.begins_with("https://"):
		success = av_player.set_source_url(_source)
	else:
		# For local files, we need to fetch them first
		await _async_fetch_and_set_local_source_av_player()
		return

	if not success:
		push_error("VideoPlayer: Failed to set AVPlayer source: ", _source)
		video_state = VIDEO_STATE_ERROR
		return

	av_player.set_looping(_is_looping)

	if _is_playing:
		av_player.play()


func _init_noop_backend():
	print("VideoPlayer: Using Noop backend (video playback not available)")
	video_state = VIDEO_STATE_NONE
	video_position = 0.0
	video_length = -1.0


func _process(_delta):
	_update_effective_volume()
	_update_video_state()


## Calculate and apply effective volume for each backend
## ExoPlayer/AVPlayer: effective_volume = master * scene * video_volume (bypasses Godot audio)
## LiveKit: Only apply video_volume (Godot buses handle master/scene)
func _update_effective_volume():
	match current_backend:
		BackendType.EXO_PLAYER:
			_update_exo_player_volume()
		BackendType.AV_PLAYER:
			_update_av_player_volume()
		BackendType.LIVEKIT:
			_update_livekit_volume()
		_:
			pass


## ExoPlayer bypasses Godot's audio system, so we calculate full effective volume
func _update_exo_player_volume():
	if not exo_player:
		return

	var effective_volume: float = 0.0

	if not dcl_muted:
		var config = Global.get_config()
		var master_volume: float = config.audio_general_volume / 100.0
		var scene_volume: float = config.audio_scene_volume / 100.0
		effective_volume = master_volume * scene_volume * dcl_volume

	if absf(effective_volume - _last_effective_volume) < 0.001:
		return

	_last_effective_volume = effective_volume
	exo_player.set_volume(effective_volume)


## AVPlayer bypasses Godot's audio system, so we calculate full effective volume
func _update_av_player_volume():
	if not av_player:
		return

	var effective_volume: float = 0.0

	if not dcl_muted:
		var config = Global.get_config()
		var master_volume: float = config.audio_general_volume / 100.0
		var scene_volume: float = config.audio_scene_volume / 100.0
		effective_volume = master_volume * scene_volume * dcl_volume

	if absf(effective_volume - _last_effective_volume) < 0.001:
		return

	_last_effective_volume = effective_volume
	av_player.set_volume(effective_volume)


## LiveKit uses Godot's AudioStreamPlayer which goes through audio buses
## Godot buses handle master/scene volume, we only apply video's own volume
func _update_livekit_volume():
	var effective_volume: float = 0.0 if dcl_muted else dcl_volume

	if absf(effective_volume - _last_effective_volume) < 0.001:
		return

	_last_effective_volume = effective_volume
	var db_volume: float = -80.0 if effective_volume <= 0.0 else 20.0 * log(effective_volume)
	self.volume_db = db_volume


## Update video state variables based on current backend state
## These variables are polled by Rust to generate CRDT events
func _update_video_state():
	match current_backend:
		BackendType.EXO_PLAYER:
			_update_exo_player_state()
		BackendType.AV_PLAYER:
			_update_av_player_state()
		BackendType.LIVEKIT:
			_update_livekit_state()
		_:
			pass


## Poll ExoPlayer state and update video_state/position/length
func _update_exo_player_state():
	if not exo_player:
		return

	var duration: float = exo_player.get_duration()
	var position: float = exo_player.get_position()
	var is_playing: bool = exo_player.is_playing()

	# Update position and length
	video_position = position
	if duration > 0:
		video_length = duration

	# Determine state based on ExoPlayer status
	if duration <= 0:
		# Still loading/buffering
		video_state = VIDEO_STATE_LOADING
	elif is_playing:
		video_state = VIDEO_STATE_PLAYING
	else:
		# Not playing - could be paused or ready
		if video_state == VIDEO_STATE_LOADING:
			video_state = VIDEO_STATE_READY
		elif video_state != VIDEO_STATE_READY:
			video_state = VIDEO_STATE_PAUSED


## Poll AVPlayer state and update video_state/position/length
func _update_av_player_state():
	if not av_player:
		return

	var duration: float = av_player.get_duration()
	var position: float = av_player.get_position()
	var is_playing: bool = av_player.is_playing()

	# Update position and length
	video_position = position
	if duration > 0:
		video_length = duration

	# Determine state based on AVPlayer status
	if duration <= 0:
		# Still loading/buffering
		video_state = VIDEO_STATE_LOADING
	elif is_playing:
		video_state = VIDEO_STATE_PLAYING
	else:
		# Not playing - could be paused or ready
		if video_state == VIDEO_STATE_LOADING:
			video_state = VIDEO_STATE_READY
		elif video_state != VIDEO_STATE_READY:
			video_state = VIDEO_STATE_PAUSED


## Poll LiveKit state
## Note: LiveKit state is primarily managed by Rust when frames arrive
## This function detects buffering/paused when frames stop arriving
const LIVEKIT_BUFFERING_THRESHOLD: float = 2.0  # seconds without frames = buffering
const LIVEKIT_STOPPED_THRESHOLD: float = 10.0  # seconds without frames = stopped/paused


func _update_livekit_state():
	# LiveKit state is set to PLAYING by Rust when frames arrive
	# Here we detect buffering/stopped when frames haven't arrived for a while

	# Only check if we've received at least one frame
	if last_frame_time <= 0:
		return

	var current_time: float = Time.get_ticks_msec() / 1000.0
	var time_since_last_frame: float = current_time - last_frame_time

	if video_state == VIDEO_STATE_PLAYING:
		# If no frames for a short while, we're buffering
		if time_since_last_frame > LIVEKIT_BUFFERING_THRESHOLD:
			video_state = VIDEO_STATE_BUFFERING
	elif video_state == VIDEO_STATE_BUFFERING:
		# If no frames for a long time, stream is likely stopped
		if time_since_last_frame > LIVEKIT_STOPPED_THRESHOLD:
			video_state = VIDEO_STATE_PAUSED


# Backend control methods called from Rust
func _backend_play():
	_is_playing = true
	match current_backend:
		BackendType.EXO_PLAYER:
			if exo_player:
				exo_player.play()
		BackendType.AV_PLAYER:
			if av_player:
				av_player.play()
		BackendType.LIVEKIT:
			pass  # LiveKit is always "playing" when receiving frames
		_:
			pass


func _backend_pause():
	_is_playing = false
	match current_backend:
		BackendType.EXO_PLAYER:
			if exo_player:
				exo_player.pause()
		BackendType.AV_PLAYER:
			if av_player:
				av_player.pause()
		BackendType.LIVEKIT:
			pass  # LiveKit doesn't support pause
		_:
			pass


func _backend_set_looping(looping: bool):
	_is_looping = looping
	match current_backend:
		BackendType.EXO_PLAYER:
			if exo_player:
				exo_player.set_looping(looping)
		BackendType.AV_PLAYER:
			if av_player:
				av_player.set_looping(looping)
		_:
			pass


func _backend_dispose():
	match current_backend:
		BackendType.EXO_PLAYER:
			if exo_player:
				exo_player.queue_free()
				exo_player = null
		BackendType.AV_PLAYER:
			if av_player:
				av_player.queue_free()
				av_player = null
		_:
			pass

	current_backend = BackendType.NOOP
	_source = ""
	# Reset state when disposing - will trigger state change event on next source
	video_state = VIDEO_STATE_NONE
	video_position = 0.0
	video_length = -1.0
	_last_effective_volume = -1.0


func _get_backend_texture() -> Texture2D:
	match current_backend:
		BackendType.EXO_PLAYER:
			if exo_player:
				return exo_player.get_texture()
		BackendType.AV_PLAYER:
			if av_player:
				return av_player.get_texture()
		BackendType.LIVEKIT:
			# LiveKit uses dcl_texture which is set from Rust
			return dcl_texture
		_:
			pass
	return dcl_texture


# LiveKit audio streaming methods
func init_livekit_audio(sample_rate: int, _num_channels: int, _samples_per_channel: int):
	if current_backend != BackendType.LIVEKIT:
		return

	var stream = self.get_stream() as AudioStreamGenerator
	if stream:
		print("VideoPlayer: Setting LiveKit audio sample_rate=", sample_rate)
		stream.mix_rate = sample_rate


func stream_buffer(data: PackedVector2Array):
	if current_backend != BackendType.LIVEKIT:
		return

	if not self.playing:
		self.play()

	var playback = self.get_stream_playback() as AudioStreamGeneratorPlayback
	if playback:
		playback.push_buffer(data)


# Legacy methods for backward compatibility with existing code
func async_request_video(file_hash):
	var content_mapping := Global.scene_runner.get_scene_content_mapping(dcl_scene_id)

	var promise = Global.content_provider.fetch_video(file_hash, content_mapping)
	var res = await PromiseUtils.async_awaiter(promise)
	if res is PromiseError:
		printerr("Error on fetching video: ", res.get_error())
	else:
		_on_video_loaded(file_hash)


func _on_video_loaded(file_hash: String):
	var local_video_path = "user://content/" + file_hash
	var absolute_file_path = local_video_path.replace("user:/", OS.get_user_data_dir())
	self.resolve_resource(absolute_file_path)


func init_audio(frame_rate, frames, length, format, bit_rate, frame_size, channels):
	print(
		"audio_stream debug init ",
		frame_rate,
		" - ",
		frames,
		" - ",
		length,
		" - ",
		format,
		" - ",
		bit_rate,
		" - ",
		frame_size,
		" - ",
		channels
	)
