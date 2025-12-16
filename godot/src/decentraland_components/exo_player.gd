class_name ExoPlayer
extends Node

## ExoPlayer wrapper for Android video playback with GPU texture support
##
## This class provides a high-level interface to the Android ExoPlayer through
## the dcl-godot-android plugin. It supports two rendering modes:
##
## 1. GPU Mode (preferred, API 29+): Uses ExternalTexture with AHardwareBuffer
##    for zero-copy GPU texture sharing. Video frames go directly from the
##    video decoder to the GPU without any CPU readback.
##
## 2. CPU Mode (fallback): Uses ImageTexture with CPU-based YUV to RGBA
##    conversion. This is slower but works on all devices.
##
## The texture is automatically resized when the actual video dimensions become
## known after loading a video source.

## Emitted when the video size changes and the texture has been resized
signal video_size_changed(width: int, height: int)

# Plugin reference
var plugin: Object = null

# Player ID from native side
var player_id: int = -1

# Rendering mode
var is_gpu_mode: bool = false

# GPU mode: ExternalTexture for zero-copy GPU rendering
var external_texture: ExternalTexture = null

# CPU mode: ImageTexture for fallback rendering
var video_texture: ImageTexture = null
var video_image: Image = null

# Video properties
var video_width: int = 0
var video_height: int = 0


func _ready():
	if OS.get_name() != "Android":
		push_warning("ExoPlayer: Only supported on Android platform")
		return

	if not Engine.has_singleton("dcl-godot-android"):
		push_error("ExoPlayer: dcl-godot-android plugin not found!")
		return

	plugin = Engine.get_singleton("dcl-godot-android")
	player_id = plugin.createExoPlayer()
	if player_id <= 0:
		push_error("ExoPlayer: Failed to create player")


func _exit_tree():
	if plugin and player_id > 0:
		plugin.exoPlayerRelease(player_id)
		player_id = -1


## Initialize the video texture with specified dimensions
## This should be called before setting the video source
## If cpu_fallback_texture is provided, it will be used for CPU mode fallback
func init_texture(width: int, height: int, cpu_fallback_texture: ImageTexture = null) -> bool:
	if not plugin or player_id <= 0:
		push_error("ExoPlayer: Player not initialized")
		return false

	video_width = width
	video_height = height

	# Initialize surface - returns: 1=CPU mode, 2=GPU mode, -1=error
	var result = plugin.exoPlayerInitSurface(player_id, width, height)
	if result <= 0:
		push_error("ExoPlayer: Failed to initialize surface")
		return false

	# Check which mode we're in
	is_gpu_mode = plugin.exoPlayerIsGpuMode(player_id)

	if is_gpu_mode:
		# GPU mode: Create ExternalTexture for zero-copy rendering
		external_texture = ExternalTexture.new()
		external_texture.set_size(Vector2i(width, height))
		print("ExoPlayer: Initialized in GPU mode (%dx%d)" % [width, height])
	else:
		# CPU mode: Create ImageTexture for fallback
		video_image = Image.create(width, height, false, Image.FORMAT_RGBA8)
		if cpu_fallback_texture:
			video_texture = cpu_fallback_texture
			video_texture.set_image(video_image)
		else:
			video_texture = ImageTexture.create_from_image(video_image)
		print("ExoPlayer: Initialized in CPU mode (%dx%d)" % [width, height])

	return true


## Set video source from URL (http, https, etc.)
func set_source_url(url: String) -> bool:
	if not plugin or player_id <= 0:
		push_error("ExoPlayer: Player not initialized")
		return false

	var result = plugin.exoPlayerSetSourceUrl(player_id, url)
	if not result:
		push_error("ExoPlayer: Failed to set source URL: ", url)
	return result


## Set video source from local file path
func set_source_local(file_path: String) -> bool:
	if not plugin or player_id <= 0:
		push_error("ExoPlayer: Player not initialized")
		return false

	var result = plugin.exoPlayerSetSourceLocal(player_id, file_path)
	if not result:
		push_error("ExoPlayer: Failed to set local source: ", file_path)
	return result


## Start or resume video playback
func play() -> void:
	if not plugin or player_id <= 0:
		push_error("ExoPlayer: Player not initialized")
		return
	plugin.exoPlayerPlay(player_id)


## Pause video playback
func pause() -> void:
	if not plugin or player_id <= 0:
		push_error("ExoPlayer: Player not initialized")
		return
	plugin.exoPlayerPause(player_id)


## Stop video playback and reset to beginning
func stop() -> void:
	if not plugin or player_id <= 0:
		push_error("ExoPlayer: Player not initialized")
		return
	plugin.exoPlayerStop(player_id)


## Seek to a specific position in the video
## @param position_sec: Position in seconds
func set_position(position_sec: float) -> void:
	if not plugin or player_id <= 0:
		push_error("ExoPlayer: Player not initialized")
		return

	var position_ms = int(position_sec * 1000.0)
	plugin.exoPlayerSetPosition(player_id, position_ms)


## Get current playback position in seconds
func get_position() -> float:
	if not plugin or player_id <= 0:
		return 0.0

	var position_ms = plugin.exoPlayerGetPosition(player_id)
	return float(position_ms) / 1000.0


## Get video duration in seconds
func get_duration() -> float:
	if not plugin or player_id <= 0:
		return 0.0

	var duration_ms = plugin.exoPlayerGetDuration(player_id)
	return float(duration_ms) / 1000.0


## Check if video is currently playing
func is_playing() -> bool:
	if not plugin or player_id <= 0:
		return false

	return plugin.exoPlayerIsPlaying(player_id)


## Get video width in pixels
func get_video_width() -> int:
	if not plugin or player_id <= 0:
		return 0

	return plugin.exoPlayerGetVideoWidth(player_id)


## Get video height in pixels
func get_video_height() -> int:
	if not plugin or player_id <= 0:
		return 0

	return plugin.exoPlayerGetVideoHeight(player_id)


## Set playback volume (0.0 to 1.0)
func set_volume(volume: float) -> void:
	if not plugin or player_id <= 0:
		push_error("ExoPlayer: Player not initialized")
		return

	plugin.exoPlayerSetVolume(player_id, clamp(volume, 0.0, 1.0))


## Get current volume (0.0 to 1.0)
func get_volume() -> float:
	if not plugin or player_id <= 0:
		return 1.0

	return plugin.exoPlayerGetVolume(player_id)


## Set whether video should loop
func set_looping(loop: bool) -> void:
	if not plugin or player_id <= 0:
		push_error("ExoPlayer: Player not initialized")
		return

	plugin.exoPlayerSetLooping(player_id, loop)


## Get player information for debugging
func get_player_info() -> String:
	if not plugin or player_id <= 0:
		return "Player not initialized"

	return plugin.exoPlayerGetInfo(player_id)


## Get the video texture for rendering
## Returns ExternalTexture in GPU mode, ImageTexture in CPU mode
func get_texture() -> Texture2D:
	if is_gpu_mode:
		return external_texture
	return video_texture


## Update the video texture (call this every frame when playing)
## Returns true if a new frame was available
func update_texture() -> bool:
	if not plugin or player_id <= 0:
		return false

	# Check if video size has changed and we need to reinitialize the surface
	if plugin.exoPlayerHasVideoSizeChanged(player_id):
		var new_width = plugin.exoPlayerGetVideoWidth(player_id)
		var new_height = plugin.exoPlayerGetVideoHeight(player_id)
		if new_width > 0 and new_height > 0 and (new_width != video_width or new_height != video_height):
			_reinitialize_surface(new_width, new_height)

	if is_gpu_mode:
		# GPU mode: Update ExternalTexture with HardwareBuffer
		return _update_texture_gpu()
	else:
		# CPU mode: Update ImageTexture with pixel data
		return _update_texture_cpu()


## GPU mode texture update - zero-copy path
func _update_texture_gpu() -> bool:
	# Check if new frame available
	if not plugin.exoPlayerHasNewHardwareBuffer(player_id):
		return false

	# Get the native AHardwareBuffer pointer
	var hw_buffer_ptr: int = plugin.exoPlayerAcquireHardwareBufferPtr(player_id)
	if hw_buffer_ptr == 0:
		return false

	# Update the ExternalTexture with the new hardware buffer
	# This passes the AHardwareBuffer* to Godot's Vulkan renderer
	external_texture.set_external_buffer_id(hw_buffer_ptr)
	return true


## CPU mode texture update - fallback path with YUV->RGBA conversion
func _update_texture_cpu() -> bool:
	# Update the SurfaceTexture with the latest video frame from ExoPlayer
	var success = plugin.exoPlayerUpdateTexture(player_id)
	if not success:
		return false

	# Get pixel data from the plugin (CPU readback)
	var pixel_data: PackedByteArray = plugin.exoPlayerGetPixelData(player_id)
	if pixel_data.size() == 0:
		return false

	# Update the image with the pixel data
	if video_image and pixel_data.size() == video_width * video_height * 4:
		video_image.set_data(video_width, video_height, false, Image.FORMAT_RGBA8, pixel_data)
		video_texture.update(video_image)
		return true

	return false


## Reinitialize the surface and texture with new dimensions
## Called automatically when video size changes
func _reinitialize_surface(new_width: int, new_height: int) -> void:
	print("ExoPlayer: Reinitializing surface from %dx%d to %dx%d" % [video_width, video_height, new_width, new_height])

	video_width = new_width
	video_height = new_height

	# Reinitialize the native surface with new dimensions
	var result = plugin.exoPlayerInitSurface(player_id, new_width, new_height)
	if result <= 0:
		push_error("ExoPlayer: Failed to reinitialize surface with new dimensions")
		return

	# Check if mode changed
	var new_gpu_mode = plugin.exoPlayerIsGpuMode(player_id)

	if new_gpu_mode:
		# GPU mode: Update ExternalTexture size
		if not external_texture:
			external_texture = ExternalTexture.new()
		external_texture.set_size(Vector2i(new_width, new_height))
		is_gpu_mode = true
	else:
		# CPU mode: Create new image and texture with proper dimensions
		video_image = Image.create(new_width, new_height, false, Image.FORMAT_RGBA8)
		if video_texture:
			video_texture.set_image(video_image)
		else:
			video_texture = ImageTexture.create_from_image(video_image)
		is_gpu_mode = false

	# Emit signal to notify listeners of the size change
	video_size_changed.emit(new_width, new_height)
