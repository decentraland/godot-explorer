class_name SeasonalSkinManager
extends Node

## Seasonal reskin template (issue #3025), starting with Halloween. Reads whether a season is
## active off the already-fetched FeatureFlags payload (godot/src/feature_flags.gd) rather than
## its own network round-trip, then fetches its two assets — a Discover background overlay image
## and an ambient audio track — through ContentProvider, same as every other remote-by-URL asset
## in this client (profile pictures, notification images, place thumbnails): the image goes
## through fetch_texture_by_url (works with any URL, not just catalyst content — see
## notifications_manager.gd's own image download), the audio through fetch_file_by_url +
## get_cache_file_path, since ContentProvider's typed audio fetch is catalyst-content_mapping-only
## with no by-URL equivalent. Both are disk-cached by ContentProvider itself, so a relaunch
## doesn't redownload unless the URL (hash) changes.
##
## Fail-open throughout: a disabled flag, an empty/missing URL, or a fetch/decode failure just
## leaves the season inactive — Discover looks and sounds exactly as it does the rest of the year.

signal seasonal_assets_ready

# Flag names exactly as served by the mobile-bff payload.
const FLAG_ENABLED := "seasonal-skin-enabled"
const FLAG_DISCOVER_OVERLAY_URL := "seasonal-skin-discover-overlay-url"
const FLAG_DISCOVER_AUDIO_URL := "seasonal-skin-discover-audio-url"

var _active := false
var _discover_overlay_texture: Texture2D = null
var _discover_audio_stream: AudioStream = null


func _ready() -> void:
	if Global.feature_flags.is_loaded():
		_async_load_config()
	else:
		Global.feature_flags.flags_loaded.connect(_async_load_config, CONNECT_ONE_SHOT)


func is_seasonal_skin_active() -> bool:
	return _active


func get_discover_overlay_texture() -> Texture2D:
	return _discover_overlay_texture


func get_discover_audio_stream() -> AudioStream:
	return _discover_audio_stream


# gdlint:ignore = async-function-name
func _async_load_config() -> void:
	_active = Global.feature_flags.is_enabled(FLAG_ENABLED, false)
	if not _active:
		return
	var overlay_url := Global.feature_flags.get_text(FLAG_DISCOVER_OVERLAY_URL, "")
	var audio_url := Global.feature_flags.get_text(FLAG_DISCOVER_AUDIO_URL, "")
	if overlay_url.is_empty() and audio_url.is_empty():
		_active = false
		return
	if not overlay_url.is_empty():
		_discover_overlay_texture = await _async_fetch_overlay_texture(overlay_url)
	if not is_instance_valid(self):
		return
	if not audio_url.is_empty():
		_discover_audio_stream = await _async_fetch_audio_stream(audio_url)
	if not is_instance_valid(self):
		return
	seasonal_assets_ready.emit()


## Same fetch_texture_by_url + hash-from-url pattern notifications_manager.gd's own
## _async_download_image_as_base64() uses for its (also arbitrary, backoffice-ish) notification
## images — ContentProvider caches/dedups by this hash on its own.
# gdlint:ignore = async-function-name
func _async_fetch_overlay_texture(url: String) -> Texture2D:
	var url_hash := NotificationUtils.get_hash_from_url(url)
	var promise: Promise = Global.content_provider.fetch_texture_by_url(url_hash, url)
	var result = await PromiseUtils.async_awaiter(promise)
	if result is PromiseError:
		push_warning("SeasonalSkinManager: overlay image fetch failed: ", result.get_error())
		return null
	return result.texture


## fetch_file_by_url only guarantees the file lands on disk (ContentProvider's typed audio fetch
## wants a catalyst content_mapping, which this backoffice URL doesn't have) — get_cache_file_path
## resolves the same hash to where it was downloaded, same two-step scene_fetcher.gd already uses
## for scene scripts/manifests fetched by URL.
# gdlint:ignore = async-function-name
func _async_fetch_audio_stream(url: String) -> AudioStream:
	var url_hash := NotificationUtils.get_hash_from_url(url)
	var local_path := Global.content_provider.get_cache_file_path(url_hash)
	if not FileAccess.file_exists(local_path):
		var promise: Promise = Global.content_provider.fetch_file_by_url(url_hash, url)
		var result = await PromiseUtils.async_awaiter(promise)
		if result is PromiseError:
			push_warning("SeasonalSkinManager: audio fetch failed: ", result.get_error())
			return null
	var file := FileAccess.open(local_path, FileAccess.READ)
	if file == null:
		push_warning("SeasonalSkinManager: could not open cached audio file: ", local_path)
		return null
	var stream := AudioStreamMP3.new()
	stream.data = file.get_buffer(file.get_length())
	return stream
