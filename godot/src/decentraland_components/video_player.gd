extends DclVideoPlayer

## Unified video player that handles multiple backends:
## - LiveKit: Streaming video from livekit-video:// URLs
## - ExoPlayer: Android video playback with GPU acceleration
## - AVPlayer: iOS video playback (future)
## - Noop: Fallback when no backend is available

enum BackendType { LIVEKIT = 0, EXO_PLAYER = 1, AV_PLAYER = 2, NOOP = 3 }

var current_backend: BackendType = BackendType.NOOP
var exo_player: Node = null  # ExoPlayer child node when using ExoPlayer backend
var _source: String = ""
var _is_playing: bool = false
var _is_looping: bool = false


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
			_init_exo_player_backend()
		BackendType.AV_PLAYER:
			_init_av_player_backend()
		_:
			_init_noop_backend()


func _init_livekit_backend():
	print("VideoPlayer: Initializing LiveKit backend for ", _source)
	# LiveKit backend uses AudioStreamGenerator for audio
	# Video frames are pushed directly to texture from Rust
	var audio_stream_generator = AudioStreamGenerator.new()
	audio_stream_generator.mix_rate = 48000.0
	audio_stream_generator.buffer_length = 1.5
	self.set_stream(audio_stream_generator)
	self.play()


func _init_exo_player_backend():
	if OS.get_name() != "Android":
		push_warning("ExoPlayer backend only available on Android, falling back to Noop")
		current_backend = BackendType.NOOP
		_init_noop_backend()
		return

	print("VideoPlayer: Initializing ExoPlayer backend for ", _source)

	# Create ExoPlayer child node
	var exo_player_scene = load("res://src/decentraland_components/exo_player.tscn")
	exo_player = exo_player_scene.instantiate()
	add_child(exo_player)

	# Wait for ExoPlayer to be ready
	await get_tree().process_frame

	# Initialize texture with initial size (will be resized when video loads)
	if not exo_player.init_texture(640, 360):
		push_error("VideoPlayer: Failed to initialize ExoPlayer texture")
		return

	# Set the video source
	var success: bool
	if _source.begins_with("http://") or _source.begins_with("https://"):
		success = exo_player.set_source_url(_source)
	else:
		# For local files, we need to fetch them first
		await _fetch_and_set_local_source()
		return

	if not success:
		push_error("VideoPlayer: Failed to set ExoPlayer source: ", _source)
		return

	exo_player.set_looping(_is_looping)

	if _is_playing:
		exo_player.play()


func _fetch_and_set_local_source():
	# This is a local file reference, need to fetch it first
	var content_mapping := Global.scene_runner.get_scene_content_mapping(dcl_scene_id)
	var file_hash = content_mapping.get_hash(_source)

	if file_hash.is_empty():
		push_error("VideoPlayer: Could not find hash for local file: ", _source)
		return

	var promise = Global.content_provider.fetch_video(file_hash, content_mapping)
	var res = await PromiseUtils.async_awaiter(promise)
	if res is PromiseError:
		printerr("VideoPlayer: Error fetching video: ", res.get_error())
		return

	var local_video_path = "user://content/" + file_hash
	var absolute_file_path = local_video_path.replace("user:/", OS.get_user_data_dir())

	if exo_player:
		var success = exo_player.set_source_local(absolute_file_path)
		if not success:
			push_error("VideoPlayer: Failed to set ExoPlayer local source: ", absolute_file_path)
			return

		exo_player.set_looping(_is_looping)
		if _is_playing:
			exo_player.play()


func _init_av_player_backend():
	# TODO: Implement AVPlayer backend for iOS
	push_warning("AVPlayer backend not yet implemented, falling back to Noop")
	current_backend = BackendType.NOOP
	_init_noop_backend()


func _init_noop_backend():
	print("VideoPlayer: Using Noop backend (video playback not available)")


# Backend control methods called from Rust
func _backend_play():
	_is_playing = true
	match current_backend:
		BackendType.EXO_PLAYER:
			if exo_player:
				exo_player.play()
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
		_:
			pass


func _backend_dispose():
	match current_backend:
		BackendType.EXO_PLAYER:
			if exo_player:
				exo_player.queue_free()
				exo_player = null
		_:
			pass

	current_backend = BackendType.NOOP
	_source = ""


func _get_backend_texture() -> Texture2D:
	match current_backend:
		BackendType.EXO_PLAYER:
			if exo_player:
				return exo_player.get_texture()
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
