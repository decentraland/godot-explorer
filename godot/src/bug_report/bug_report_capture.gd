class_name BugReportCapture
extends RefCounted

## Holds the most recent in-world screenshot for the bug report form (#2652).
##
## The capture happens when the Settings panel opens, NOT when the Report a Bug
## form opens: by the time the form is up, the viewport shows Settings (and then
## the form itself) rather than the bug the player wants to report. Grabbing it
## at Settings-open time means the image is still the game.
##
## Only one capture is kept — the form pre-fills slot 0 from it, and the player
## can delete it or add more from their gallery.
##
## Stored as encoded JPEG bytes, not an `Image`. A raw 1920px frame is ~7MB of
## RGBA8 held for the process lifetime, and the encode had to happen anyway —
## doing it here, when Settings opens, keeps it off the submit path where the
## player is waiting on a spinner (PR #2779 review).

# Longest edge, matching ImagePickerService so gallery picks and captures are
# sized alike before they reach the 3MB evidence cap.
const MAX_DIMENSION := 1920

# Matches BugReportService and ImagePickerService, so every attachment reaching
# the 3MB evidence cap was encoded the same way.
const JPEG_QUALITY := 0.85

# Fallbacks for encode_within(), tried in order until an image fits its evidence
# budget. Starts below MAX_DIMENSION/JPEG_QUALITY: captures and gallery picks are
# already encoded that way, and only images over budget get here, so re-encoding
# at the same settings would always miss (PR #2906 review).
const SHRINK_STEPS := [
	{"dimension": 1440, "quality": 0.75},
	{"dimension": 1080, "quality": 0.7},
	{"dimension": 720, "quality": 0.6},
]

static var _latest_jpeg: PackedByteArray = PackedByteArray()


## Subscribes to both Settings-open signals on `global`, so the frame is grabbed
## before the panel covers it.
##
## Owned here rather than in global.gd because Settings is reachable from two
## places — `open_settings_panel` in-world and `open_settings` from the lobby menu,
## where no explorer exists — and because global.gd sits against its 1900-line lint
## cap. Global is an autoload, so it connects before either UI does and this runs
## first.
static func listen_for_settings(global: Node) -> void:
	var on_opened := Callable(BugReportCapture, "_on_settings_opened").bind(global)
	global.open_settings_panel.connect(on_opened)
	global.open_settings.connect(on_opened)


static func _on_settings_opened(global: Node) -> void:
	capture_for_settings(global.get_viewport())


## Grabs the currently rendered frame. Call before the covering UI is shown.
## Failures are silent: a missing screenshot must never block opening Settings.
static func capture(viewport: Viewport) -> void:
	if viewport == null:
		return
	var texture := viewport.get_texture()
	if texture == null:
		return
	var image := texture.get_image()
	if image == null or image.is_empty():
		return

	_fit_dimension(image, MAX_DIMENSION)
	var bytes := image.save_jpg_to_buffer(JPEG_QUALITY)
	if bytes.is_empty():
		push_warning("BugReportCapture: could not encode the captured frame")
		return
	_latest_jpeg = bytes


## Re-encodes `image` as JPEG, stepping down size and quality until it is at most
## `max_bytes`. Empty when even the smallest step doesn't fit. `image` is left
## untouched — each step works on a copy.
static func encode_within(image: Image, max_bytes: int) -> PackedByteArray:
	if image == null or image.is_empty():
		return PackedByteArray()
	for step in SHRINK_STEPS:
		var candidate: Image = image.duplicate()
		_fit_dimension(candidate, step["dimension"])
		var bytes := candidate.save_jpg_to_buffer(step["quality"])
		if not bytes.is_empty() and bytes.size() <= max_bytes:
			return bytes
	return PackedByteArray()


static func _fit_dimension(image: Image, max_dimension: int) -> void:
	var longest: int = maxi(image.get_width(), image.get_height())
	if longest <= max_dimension:
		return
	var scale := float(max_dimension) / float(longest)
	image.resize(
		maxi(1, int(image.get_width() * scale)),
		maxi(1, int(image.get_height() * scale)),
		Image.INTERPOLATE_BILINEAR
	)


## Capture from the Settings-opened path.
##
## In landscape the side navbar opens BEFORE Settings and covers the world, so
## navbar.gd takes the clean frame itself; capturing again here would overwrite
## it with a navbar-covered one. The navbar is landscape-only, so the portrait
## and lobby paths — which have no navbar — still capture here.
static func capture_for_settings(viewport: Viewport) -> void:
	if not Global.is_orientation_portrait() and Global.get_explorer() != null:
		return
	capture(viewport)


## The last capture as JPEG bytes, empty when there is none. Not consumed —
## reopening the form reuses it.
static func latest_jpeg() -> PackedByteArray:
	return _latest_jpeg


static func clear() -> void:
	_latest_jpeg = PackedByteArray()
