class_name SeasonalSkinManager
extends Node

## Seasonal reskin template (issue #3025), starting with Halloween. The asset fetch is lazy:
## async_request_assets() does nothing until something actually needs it (Discover's first
## show), not at boot — most sessions never open Discover, and downloading both files for every
## user with the flag on regardless would just compete with cold-start traffic for no reason.
## Reads the active season key off the already-fetched FeatureFlags payload
## (godot/src/feature_flags.gd) rather than its own network round-trip, then fetches its two
## assets — a Discover background overlay image and an ambient audio track — through
## ContentProvider, same as every other remote-by-URL asset in this client (profile pictures,
## notification images, place thumbnails): the image goes through fetch_texture_by_url (works
## with any URL, not just catalyst content — see notifications_manager.gd's own image download),
## the audio through fetch_file_by_url + get_cache_file_path, since ContentProvider's typed audio
## fetch is catalyst-content_mapping-only with no by-URL equivalent. Both are disk-cached by
## ContentProvider itself, so a relaunch doesn't redownload unless the URL (hash) changes.
##
## The flag's value is a season key (e.g. "halloween"), not a URL — it names a folder under
## dcl-regenesislabs/mobile-ota's seasons/ (see that repo's README), and the two asset URLs are
## built from it by convention, same filenames every season.
##
## Fail-open throughout: an empty key, or a fetch/decode failure, just leaves the season inactive
## — Discover looks and sounds exactly as it does the rest of the year. This manager is never
## freed (it lives as long as Global, godot/src/global.gd), so there's no need to guard an await
## against the node going away mid-fetch.

signal seasonal_assets_ready

# Flag name exactly as served by the mobile-bff payload; value is a season key, empty = off.
const FLAG_SEASON_KEY := "seasonal-skin-key"
const R2_BASE_URL := "https://mobile-ota.dclregenesislabs.xyz/seasons/"
# fetch_file_by_url still downloads the whole file to disk first — this only caps what gets
# decoded into memory afterward, not the download itself. Generous for a looped ambient track;
# a season folder holding anything this large is a mistake in the bucket.
const MAX_AUDIO_FILE_BYTES := 15 * 1024 * 1024

var _active := false
var _assets_requested := false
var _discover_overlay_texture: Texture2D = null
var _discover_audio_stream: AudioStream = null


func is_seasonal_skin_active() -> bool:
	return _active


func get_discover_overlay_texture() -> Texture2D:
	return _discover_overlay_texture


func get_discover_audio_stream() -> AudioStream:
	return _discover_audio_stream


## Idempotent — only the first call after the season is known to be active fetches anything;
## every call after that (including ones made before the flags payload arrives) is a no-op,
## UNLESS a previous attempt left an asset null while the season is still active, in which case
## it retries. Discover calls this on every re-show, so that retry is effectively free. Note the
## retry only ever helps the audio half: fetch_file_by_url (content_provider.rs) drops a resolved
## promise from its cache either way, so a fresh call re-downloads, while fetch_texture_by_url
## caches a promise permanently once settled — a failed overlay fetch returns the same rejection
## for the rest of the session no matter how many times this runs.
# gdlint:ignore = async-function-name
func async_request_assets() -> void:
	if (
		_assets_requested
		and _active
		and (
			not is_instance_valid(_discover_overlay_texture)
			or not is_instance_valid(_discover_audio_stream)
		)
	):
		_assets_requested = false
	if _assets_requested:
		return
	# Set before the await below, not after: a second call arriving while this one is still
	# waiting on flags_loaded must not treat itself as the first request too.
	_assets_requested = true
	if not Global.feature_flags.is_loaded():
		await Global.feature_flags.flags_loaded
	var season_key := Global.feature_flags.get_text(FLAG_SEASON_KEY, "").strip_edges()
	_active = not season_key.is_empty()
	if not _active:
		return
	var season_base := R2_BASE_URL + season_key + "/"
	# Emitted once per asset, not once at the end: the audio file is the larger download, and
	# Discover has no reason to wait for it before showing the overlay it already has.
	_discover_overlay_texture = await _async_fetch_overlay_texture(
		season_base + "discover-overlay.png"
	)
	seasonal_assets_ready.emit()
	_discover_audio_stream = await _async_fetch_audio_stream(season_base + "discover-ambient.mp3")
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
	var file_length := file.get_length()
	if file_length > MAX_AUDIO_FILE_BYTES:
		push_warning(
			"SeasonalSkinManager: audio file too large (", file_length, " bytes): ", local_path
		)
		return null
	var stream := AudioStreamMP3.new()
	stream.data = file.get_buffer(file_length)
	if stream.data.is_empty():
		# set_data() leaves data empty (and raises an engine error) when the buffer isn't a
		# decodable mp3 — a wrong file in the bucket, not something to swap the music for.
		push_warning("SeasonalSkinManager: audio file is not a decodable mp3: ", local_path)
		return null
	stream.loop = true
	return stream
