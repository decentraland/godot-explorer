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

# Longest edge, matching ImagePickerService so gallery picks and captures are
# sized alike before they reach the 3MB evidence cap.
const MAX_DIMENSION := 1920

static var _latest: Image = null


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

	var longest: int = maxi(image.get_width(), image.get_height())
	if longest > MAX_DIMENSION:
		var scale := float(MAX_DIMENSION) / float(longest)
		image.resize(
			maxi(1, int(image.get_width() * scale)),
			maxi(1, int(image.get_height() * scale)),
			Image.INTERPOLATE_BILINEAR
		)

	_latest = image


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


## The last capture, or null. Not consumed — reopening the form reuses it.
static func latest() -> Image:
	return _latest


static func clear() -> void:
	_latest = null
