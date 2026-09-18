class_name ImagePickerService
extends RefCounted

## Native photo-gallery picker for iOS and Android.
##
## Lets the user pick one image from their device's photo library and returns it
## as a decoded `Image`. Enables the screenshot attachments of the native bug
## reporting flow (issue #2652).
##
## The native side does the decode, EXIF rotation, downscale and JPEG re-encode,
## so GDScript never has to hold a full-resolution 12MP photo in memory. What
## comes back over the bridge is a small, already-encoded JPEG buffer.
##
## Dispatch pattern (mirrors AttestationService): call the singleton, await the
## `image_picked` signal, decode. The native side guarantees exactly one signal
## emission per call — success, cancel, or error.
##
## Usage:
##     if ImagePickerService.is_supported():
##         var image := await ImagePickerService.async_pick_image()
##         if image != null:
##             texture_rect.texture = ImageTexture.create_from_image(image)
##
## Desktop is unsupported: `is_supported()` returns false and
## `async_pick_image()` returns null immediately without touching a singleton.

# Longest edge of the returned image, in pixels. The native side scales down to
# fit; smaller images are never upscaled.
const MAX_DIMENSION := 1920

# JPEG quality of the re-encoded buffer, 1-100.
const JPEG_QUALITY := 85

const IOS_SINGLETON := "DclGodotiOS"
const ANDROID_SINGLETON := "dcl-godot-android"

const SIGNAL_NAME := "image_picked"

# Watchdog for a native side that never reports back. Generous, because the
# user may browse their library for a while before choosing — this exists to
# stop a permanent hang, not to bound normal use.
const TIMEOUT_MS := 300000

# Only one picker can be on screen at a time. A second concurrent call would
# race on the shared `image_picked` signal and hand both awaiters the same
# result, so it is rejected instead.
static var _picking := false


## True when a native gallery picker is reachable on this platform.
static func is_supported() -> bool:
	return _get_plugin() != null


## Opens the native gallery picker and returns the chosen image as the JPEG
## buffer the native side produced.
##
## Returns an empty buffer when the user cancelled, the platform is unsupported,
## another pick is already in flight, or the native side never answered. The
## bytes are handed over undecoded so a caller that only forwards them — the bug
## report attachment path — never pays a decode/re-encode round trip.
static func async_pick_image_bytes() -> PackedByteArray:
	var plugin := _get_plugin()
	if plugin == null:
		push_warning("ImagePickerService: not supported on %s" % OS.get_name())
		return PackedByteArray()

	if _picking:
		push_warning("ImagePickerService: a pick is already in flight, ignoring")
		return PackedByteArray()

	_picking = true
	# Capture the result via an explicit one-shot connection rather than
	# `await plugin.image_picked`. A bare await parks the coroutine forever if
	# the native side never emits, which strands both this flag and whatever UI
	# is waiting on the call — exactly the hang seen when a picker sheet was
	# dismissed by swiping instead of tapping Cancel.
	# `received` must be MUTATED, never reassigned: GDScript lambdas capture by
	# value, so `received = [...]` inside the callback would only rebind the
	# lambda's private copy and this coroutine would spin until the watchdog.
	# Array is a reference type, so assign() writes through to the same object.
	# Params stay untyped so a Variant that doesn't exactly match the signal's
	# declared types can't make the call bind fail silently.
	var received: Array = []
	var on_picked := func(picked_bytes, picked_error) -> void:
		received.assign([picked_bytes, picked_error])
	plugin.connect(SIGNAL_NAME, on_picked, CONNECT_ONE_SHOT)

	if OS.get_name() == "Android":
		plugin.pickImageFromGallery(MAX_DIMENSION, JPEG_QUALITY)
	else:
		plugin.pick_image_from_gallery(MAX_DIMENSION, JPEG_QUALITY)

	var tree := Engine.get_main_loop() as SceneTree
	var started_ms := Time.get_ticks_msec()
	while received.is_empty() and Time.get_ticks_msec() - started_ms < TIMEOUT_MS:
		await tree.process_frame

	# CONNECT_ONE_SHOT only disconnects if the signal actually fired.
	if plugin.is_connected(SIGNAL_NAME, on_picked):
		plugin.disconnect(SIGNAL_NAME, on_picked)
	_picking = false

	if received.is_empty():
		push_warning("ImagePickerService: native did not respond within %ds" % (TIMEOUT_MS / 1000))
		return PackedByteArray()

	var bytes: PackedByteArray = received[0]
	var error: String = received[1]

	if not error.is_empty():
		# "cancelled" is the normal way out of a picker, not a failure worth
		# reporting to Sentry.
		if error != "cancelled":
			push_warning("ImagePickerService: pick failed: %s" % error)
		return PackedByteArray()

	if bytes.is_empty():
		push_warning("ImagePickerService: native returned an empty buffer")
		return PackedByteArray()

	return bytes


## Decodes a buffer returned by async_pick_image_bytes(), or null when it isn't
## an image we can read.
static func decode(bytes: PackedByteArray) -> Image:
	if bytes.is_empty():
		return null
	var image := Image.new()
	# The native side always encodes JPEG, but fall back to PNG so a future
	# native change can't silently break this path.
	if image.load_jpg_from_buffer(bytes) != OK:
		if image.load_png_from_buffer(bytes) != OK:
			push_warning("ImagePickerService: could not decode the returned buffer")
			return null
	return image


## Opens the native gallery picker and returns the decoded image, or null.
## Prefer async_pick_image_bytes() when the buffer is going to be transmitted:
## decoding and re-encoding costs quality and time for nothing.
static func async_pick_image() -> Image:
	return decode(await async_pick_image_bytes())


# Resolves the platform plugin singleton, or null when there isn't one.
#
# has_singleton() is checked first: a bare get_singleton() on a missing name
# logs a Godot ERROR, which Sentry then picks up as a real failure.
static func _get_plugin() -> Object:
	match OS.get_name():
		"iOS":
			if Engine.has_singleton(IOS_SINGLETON):
				return Engine.get_singleton(IOS_SINGLETON)
		"Android":
			if Engine.has_singleton(ANDROID_SINGLETON):
				return Engine.get_singleton(ANDROID_SINGLETON)
	return null
