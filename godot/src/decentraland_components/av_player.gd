class_name AVPlayer
extends Node

## AVPlayer wrapper for iOS video playback with GPU texture support
##
## This class provides a high-level interface to the iOS AVPlayer through
## the dcl-godot-ios plugin.
##
## GPU Mode: Uses ExternalTexture with IOSurface
##    for zero-copy GPU texture sharing. Video frames go directly from the
##    video decoder to the GPU without any CPU readback.
##
## The texture is automatically resized when the actual video dimensions become
## known after loading a video source.

## Emitted when the video size changes and the texture has been resized
signal video_size_changed(width: int, height: int)

# Debug logging - set to true for verbose output during development
const DEBUG_LOGGING: bool = false

# Plugin reference
var plugin: Object = null

# Player ID from native side
var player_id: int = -1

# GPU mode: ExternalTexture for zero-copy GPU rendering
var external_texture: ExternalTexture = null

# Video properties
var video_width: int = 0
var video_height: int = 0

var _frame_count: int = 0
var _last_iosurface_ptr: int = 0

# Flag to skip texture updates during surface reinitialization
# This prevents grey flickering when transitioning between surfaces
var _reinitializing_surface: bool = false

## GPU mode texture update - zero-copy path
## Track if we've received a frame after reinitialization (for debugging)
var _frames_after_reinit: int = 0
var _waiting_for_first_frame: bool = false


func _ready():
	if OS.get_name() != "iOS":
		push_warning("AVPlayer: Only supported on iOS platform")
		return

	if not Engine.has_singleton("DclGodotiOS"):
		push_error("AVPlayer: DclGodotiOS plugin not found!")
		return

	plugin = Engine.get_singleton("DclGodotiOS")
	player_id = plugin.createAVPlayer()
	if player_id <= 0:
		push_error("AVPlayer: Failed to create player")


func _exit_tree():
	if plugin and player_id > 0:
		plugin.avPlayerRelease(player_id)
		player_id = -1


## Initialize the video texture with specified dimensions
## This should be called before setting the video source
func init_texture(width: int, height: int) -> bool:
	if not plugin or player_id <= 0:
		push_error("AVPlayer: Player not initialized")
		return false

	video_width = width
	video_height = height

	# Initialize surface - returns: 1=success, 0=error
	var result = plugin.avPlayerInitSurface(player_id, width, height)
	if result <= 0:
		push_error("AVPlayer: Failed to initialize surface")
		return false

	# GPU mode: Create ExternalTexture for zero-copy rendering
	external_texture = ExternalTexture.new()
	external_texture.set_size(Vector2i(width, height))
	print("AVPlayer: Initialized in GPU mode (%dx%d)" % [width, height])

	return true


## Set video source from URL (http, https, etc.)
func set_source_url(url: String) -> bool:
	if not plugin or player_id <= 0:
		push_error("AVPlayer: Player not initialized")
		return false

	var result = plugin.avPlayerSetSourceUrl(player_id, url)
	if not result:
		push_error("AVPlayer: Failed to set source URL: ", url)
	return result


## Set video source from local file path
func set_source_local(file_path: String) -> bool:
	if not plugin or player_id <= 0:
		push_error("AVPlayer: Player not initialized")
		return false

	var result = plugin.avPlayerSetSourceLocal(player_id, file_path)
	if not result:
		push_error("AVPlayer: Failed to set local source: ", file_path)
	return result


## Start or resume video playback
func play() -> void:
	if not plugin or player_id <= 0:
		push_error("AVPlayer: Player not initialized")
		return
	plugin.avPlayerPlay(player_id)


## Pause video playback
func pause() -> void:
	if not plugin or player_id <= 0:
		push_error("AVPlayer: Player not initialized")
		return
	plugin.avPlayerPause(player_id)


## Stop video playback and reset to beginning
func stop() -> void:
	if not plugin or player_id <= 0:
		push_error("AVPlayer: Player not initialized")
		return
	plugin.avPlayerStop(player_id)


## Seek to a specific position in the video
## @param position_sec: Position in seconds
func set_position(position_sec: float) -> void:
	if not plugin or player_id <= 0:
		push_error("AVPlayer: Player not initialized")
		return

	plugin.avPlayerSetPosition(player_id, position_sec)


## Get current playback position in seconds
func get_position() -> float:
	if not plugin or player_id <= 0:
		return 0.0

	return plugin.avPlayerGetPosition(player_id)


## Get video duration in seconds
func get_duration() -> float:
	if not plugin or player_id <= 0:
		return 0.0

	return plugin.avPlayerGetDuration(player_id)


## Check if video is currently playing
func is_playing() -> bool:
	if not plugin or player_id <= 0:
		return false

	return plugin.avPlayerIsPlaying(player_id)


## Get video width in pixels
func get_video_width() -> int:
	if not plugin or player_id <= 0:
		return 0

	return plugin.avPlayerGetVideoWidth(player_id)


## Get video height in pixels
func get_video_height() -> int:
	if not plugin or player_id <= 0:
		return 0

	return plugin.avPlayerGetVideoHeight(player_id)


## Set playback volume (0.0 to 1.0)
func set_volume(volume: float) -> void:
	if not plugin or player_id <= 0:
		push_error("AVPlayer: Player not initialized")
		return

	plugin.avPlayerSetVolume(player_id, clamp(volume, 0.0, 1.0))


## Get current volume (0.0 to 1.0)
func get_volume() -> float:
	if not plugin or player_id <= 0:
		return 1.0

	return plugin.avPlayerGetVolume(player_id)


## Set whether video should loop
func set_looping(loop: bool) -> void:
	if not plugin or player_id <= 0:
		push_error("AVPlayer: Player not initialized")
		return

	plugin.avPlayerSetLooping(player_id, loop)


## Set playback rate (1.0 = normal speed)
func set_playback_rate(rate: float) -> void:
	if not plugin or player_id <= 0:
		push_error("AVPlayer: Player not initialized")
		return

	plugin.avPlayerSetPlaybackRate(player_id, clamp(rate, 0.1, 10.0))


## Get player information for debugging
func get_player_info() -> String:
	if not plugin or player_id <= 0:
		return "Player not initialized"

	return plugin.avPlayerGetInfo(player_id)


## Get the video texture for rendering
## Returns ExternalTexture in GPU mode
func get_texture() -> Texture2D:
	return external_texture


## Update the video texture (call this every frame when playing)
## Returns true if a new frame was available
func update_texture() -> bool:
	if not plugin or player_id <= 0:
		return false

	# Check if video size has changed and we need to reinitialize the surface
	if plugin.avPlayerHasVideoSizeChanged(player_id):
		var new_width = plugin.avPlayerGetVideoWidth(player_id)
		var new_height = plugin.avPlayerGetVideoHeight(player_id)
		if (
			new_width > 0
			and new_height > 0
			and (new_width != video_width or new_height != video_height)
		):
			_reinitialize_surface(new_width, new_height)

	return _update_texture_gpu()


func _update_texture_gpu() -> bool:
	# Skip updates during surface reinitialization to prevent flickering
	if _reinitializing_surface:
		return false

	# Check if plugin and player are valid
	if not plugin or player_id <= 0:
		return false

	# Check if new frame available
	if not plugin.avPlayerHasNewPixelBuffer(player_id):
		return false

	# Get the native IOSurface pointer
	var iosurface_ptr: int = plugin.avPlayerAcquireIOSurfacePtr(player_id)
	if iosurface_ptr == 0:
		if _frame_count < 10:
			print("AVPlayer: acquireIOSurfacePtr returned 0")
		return false

	_frame_count += 1

	# Debug: log first few frames and when IOSurface pointer changes
	if DEBUG_LOGGING and (_frame_count <= 5 or iosurface_ptr != _last_iosurface_ptr):
		print(
			(
				"AVPlayer: Frame #%d, IOSurface=0x%x, size=%dx%d"
				% [_frame_count, iosurface_ptr, video_width, video_height]
			)
		)
	_last_iosurface_ptr = iosurface_ptr

	# Debug: log first frame after reinitialization
	if _waiting_for_first_frame:
		_waiting_for_first_frame = false
		_frames_after_reinit += 1
		if DEBUG_LOGGING:
			print(
				(
					"AVPlayer: First frame received after reinit #%d (iosurface=0x%x)"
					% [_frames_after_reinit, iosurface_ptr]
				)
			)

	# Update the ExternalTexture with the new IOSurface
	# This passes the IOSurfaceRef to Godot's Metal renderer
	external_texture.set_external_buffer_id(iosurface_ptr)
	return true


## Reinitialize the surface and texture with new dimensions
## Called automatically when video size changes
func _reinitialize_surface(new_width: int, new_height: int) -> void:
	if DEBUG_LOGGING:
		print(
			(
				"AVPlayer: Reinitializing surface from %dx%d to %dx%d"
				% [video_width, video_height, new_width, new_height]
			)
		)

	# Set flag to prevent texture updates during transition
	# This prevents grey flickering from invalid/stale buffers
	_reinitializing_surface = true

	video_width = new_width
	video_height = new_height

	# IMPORTANT: Clear the external buffer reference BEFORE reinitializing the surface.
	# This prevents memory allocation failures that occur when set_size() is called
	# with the old IOSurface still referenced.
	if external_texture:
		external_texture.set_external_buffer_id(0)

	# Reinitialize the native surface with new dimensions
	var result = plugin.avPlayerInitSurface(player_id, new_width, new_height)
	if result <= 0:
		push_error("AVPlayer: Failed to reinitialize surface with new dimensions")
		_reinitializing_surface = false
		return

	# GPU mode: Update ExternalTexture size
	# Now safe to resize since the old IOSurface has been cleared
	if not external_texture:
		external_texture = ExternalTexture.new()
	external_texture.set_size(Vector2i(new_width, new_height))

	# Clear the reinitialization flag - next frame from new surface will be valid
	_reinitializing_surface = false

	# Debug: mark that we're waiting for first frame after reinit
	_waiting_for_first_frame = true

	# Emit signal to notify listeners of the size change
	video_size_changed.emit(new_width, new_height)


func _process(_delta):
	update_texture()
