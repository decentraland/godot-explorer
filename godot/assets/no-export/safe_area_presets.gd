class_name SafeAreaPresets

# iPhone 14 Pro reference dimensions (native resolution)
const IOS_LANDSCAPE_SIZE := Vector2i(2556, 1179)
const IOS_PORTRAIT_SIZE := Vector2i(1179, 2556)

# iPhone 14 Pro safe area insets (pixels at native resolution)
# Landscape: Rect2i(177, 0, 2202, 1116) -> insets: left=177, top=0, right=177, bottom=63
# Portrait: Rect2i(0, 177, 1179, 2277) -> insets: left=0, top=177, right=0, bottom=102
const IOS_LANDSCAPE_INSETS := {"left": 177, "top": 0, "right": 177, "bottom": 63}
const IOS_PORTRAIT_INSETS := {"left": 0, "top": 177, "right": 0, "bottom": 102}

# Motorola Edge 60 Pro reference dimensions (native resolution)
const ANDROID_LANDSCAPE_SIZE := Vector2i(2712, 1220)
const ANDROID_PORTRAIT_SIZE := Vector2i(1220, 2712)

# Motorola Edge 60 Pro safe area insets (pixels at native resolution)
# Landscape: Rect2i(128, 0, 2584, 1220) -> insets: left=128 (camera notch), others=0
# Portrait: Rect2i(0, 128, 1220, 2584) -> insets: top=128 (camera/status bar), others=0
const ANDROID_LANDSCAPE_INSETS := {"left": 128, "top": 0, "right": 0, "bottom": 0}
const ANDROID_PORTRAIT_INSETS := {"left": 0, "top": 128, "right": 0, "bottom": 0}


static func get_ios_safe_area(is_portrait: bool, window_size: Vector2i) -> Rect2i:
	var ref_size: Vector2i
	var insets: Dictionary

	if is_portrait:
		ref_size = IOS_PORTRAIT_SIZE
		insets = IOS_PORTRAIT_INSETS
	else:
		ref_size = IOS_LANDSCAPE_SIZE
		insets = IOS_LANDSCAPE_INSETS

	# Scale insets proportionally to window size
	var scale_x := float(window_size.x) / ref_size.x
	var scale_y := float(window_size.y) / ref_size.y

	var left := int(insets["left"] * scale_x)
	var top := int(insets["top"] * scale_y)
	var right := int(insets["right"] * scale_x)
	var bottom := int(insets["bottom"] * scale_y)

	return Rect2i(left, top, window_size.x - left - right, window_size.y - top - bottom)


static func get_android_safe_area(is_portrait: bool, window_size: Vector2i) -> Rect2i:
	var ref_size: Vector2i
	var insets: Dictionary

	if is_portrait:
		ref_size = ANDROID_PORTRAIT_SIZE
		insets = ANDROID_PORTRAIT_INSETS
	else:
		ref_size = ANDROID_LANDSCAPE_SIZE
		insets = ANDROID_LANDSCAPE_INSETS

	# Scale insets proportionally to window size
	var scale_x := float(window_size.x) / ref_size.x
	var scale_y := float(window_size.y) / ref_size.y

	var left := int(insets["left"] * scale_x)
	var top := int(insets["top"] * scale_y)
	var right := int(insets["right"] * scale_x)
	var bottom := int(insets["bottom"] * scale_y)

	return Rect2i(left, top, window_size.x - left - right, window_size.y - top - bottom)


static func get_ios_window_size(is_portrait: bool) -> Vector2i:
	if is_portrait:
		# height=1280 -> width = 1280 * (1179/2556) = 590
		return Vector2i(590, 1280)
	# height=720 -> width = 720 * (2556/1179) = 1561
	return Vector2i(1561, 720)


static func get_android_window_size(is_portrait: bool) -> Vector2i:
	# Motorola Edge 60 Pro aspect ratio (1220 x 2712)
	if is_portrait:
		# height=1280 -> width = 1280 * (1220/2712) = 576
		return Vector2i(576, 1280)
	# height=720 -> width = 720 * (2712/1220) = 1600
	return Vector2i(1600, 720)
