@tool
class_name SafeMarginContainer
extends MarginContainer

@export var default_margin: int = 0
@export var use_left: bool = true
@export var use_right: bool = true
@export var use_top: bool = true
@export var use_bottom: bool = true

@export_group("Minimum Margin")
@export var min_margin_top: int = 0
@export var min_margin_bottom: int = 0
@export var min_margin_left: int = 0
@export var min_margin_right: int = 0

## Optional shared inset rectangle. When assigned, its four values override the
## min_margin_* above so several containers can be aligned from a single resource.
@export var margin_profile: SafeAreaMargins:
	set(value):
		margin_profile = value
		if Engine.is_editor_hint() and is_inside_tree():
			_update_margins_editor()

## When true, left and right margins are kept symmetric: both take the larger of the two
## resolved insets, so a one-sided device inset (e.g. a landscape camera notch) widens the
## opposite side to match and the framed rectangle stays centered. Floors still apply first.
@export var symmetric_horizontal: bool = false:
	set(value):
		symmetric_horizontal = value
		if Engine.is_editor_hint() and is_inside_tree():
			_update_margins_editor()


func _ready() -> void:
	if Engine.is_editor_hint():
		set_process(true)
		_update_margins_editor()
		return
	get_window().size_changed.connect(self._on_size_changed)
	Global.orientation_changed.connect(_on_orientation_changed)
	_on_size_changed()


func _on_orientation_changed(_portrait: bool) -> void:
	_on_size_changed()


## Portrait selects the margin_profile.portrait_* set. In the editor it follows the
## _mobile_preview settings; at runtime it follows the live orientation.
func _is_portrait() -> bool:
	if Engine.is_editor_hint():
		if ProjectSettings.get_setting("_mobile_preview/active", false):
			return ProjectSettings.get_setting("_mobile_preview/is_portrait", true)
		return false
	return Global.is_orientation_portrait()


func _exit_tree() -> void:
	if Engine.is_editor_hint():
		set_process(false)


func _process(_delta: float) -> void:
	if Engine.is_editor_hint():
		_update_margins_editor()


func _notification(what: int) -> void:
	if not Engine.is_editor_hint():
		return
	if what == NOTIFICATION_EDITOR_PRE_SAVE:
		if use_top:
			remove_theme_constant_override("margin_top")
		if use_left:
			remove_theme_constant_override("margin_left")
		if use_right:
			remove_theme_constant_override("margin_right")
		if use_bottom:
			remove_theme_constant_override("margin_bottom")
	elif what == NOTIFICATION_EDITOR_POST_SAVE:
		_update_margins_editor()


func _update_margins_editor() -> void:
	var active: bool = ProjectSettings.get_setting("_mobile_preview/active", false)
	if not active:
		_apply_margins(default_margin, default_margin, default_margin, default_margin)
		return

	var is_ios: bool = ProjectSettings.get_setting("_mobile_preview/is_ios", true)
	var is_portrait: bool = ProjectSettings.get_setting("_mobile_preview/is_portrait", true)
	var vp_width: int = ProjectSettings.get_setting("_mobile_preview/viewport_width", 720)
	var vp_height: int = ProjectSettings.get_setting("_mobile_preview/viewport_height", 720)
	var window_size := Vector2i(vp_width, vp_height)

	var presets_script: GDScript = load("res://assets/no-export/safe_area_presets.gd")
	var safe_area: Rect2i
	if is_ios:
		safe_area = presets_script.get_ios_safe_area(is_portrait, window_size)
	else:
		safe_area = presets_script.get_android_safe_area(is_portrait, window_size)

	var top: int = max(default_margin, safe_area.position.y)
	var left: int = max(default_margin, safe_area.position.x)
	var bottom: int = max(default_margin, abs(safe_area.end.y - window_size.y))
	var right: int = max(default_margin, abs(safe_area.end.x - window_size.x))

	_apply_margins(top, left, bottom, right)


func _apply_margins(top: int, left: int, bottom: int, right: int) -> void:
	var floor_top: int = min_margin_top
	var floor_left: int = min_margin_left
	var floor_right: int = min_margin_right
	var floor_bottom: int = min_margin_bottom
	if margin_profile:
		if _is_portrait():
			floor_top = margin_profile.portrait_top
			floor_left = margin_profile.portrait_left
			floor_right = margin_profile.portrait_right
			floor_bottom = margin_profile.portrait_bottom
		else:
			floor_top = margin_profile.top
			floor_left = margin_profile.left
			floor_right = margin_profile.right
			floor_bottom = margin_profile.bottom
	var applied_left: int = maxi(left, floor_left)
	var applied_right: int = maxi(right, floor_right)
	if symmetric_horizontal:
		var side: int = maxi(applied_left, applied_right)
		applied_left = side
		applied_right = side
	if use_top:
		add_theme_constant_override("margin_top", maxi(top, floor_top))
	if use_left:
		add_theme_constant_override("margin_left", applied_left)
	if use_right:
		add_theme_constant_override("margin_right", applied_right)
	if use_bottom:
		add_theme_constant_override("margin_bottom", maxi(bottom, floor_bottom))


func _on_size_changed():
	if not is_inside_tree() or get_viewport() == null:
		return
	var safe_area: Rect2i = Global.get_safe_area()
	var window_size: Vector2i = DisplayServer.window_get_size()
	var viewport_size = get_viewport().get_visible_rect().size

	# BASE MARGINS
	var top: int = default_margin
	var left: int = default_margin
	var bottom: int = default_margin
	var right: int = default_margin

	var x_factor: float = viewport_size.x / window_size.x
	var y_factor: float = viewport_size.y / window_size.y

	if Global.is_mobile() or Global.is_emulating_safe_area():
		top = max(top, safe_area.position.y * y_factor)
		left = max(left, safe_area.position.x * x_factor)
		bottom = max(bottom, abs(safe_area.end.y - window_size.y) * y_factor)
		right = max(right, abs(safe_area.end.x - window_size.x) * x_factor)

	_apply_margins(top, left, bottom, right)


## Recompute margins from the current window size and safe area (e.g. after temporary overrides).
func refresh_margins() -> void:
	_on_size_changed()
