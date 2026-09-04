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
