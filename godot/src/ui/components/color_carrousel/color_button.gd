extends CustomTouchButton

@export var color: Color:
	set(value):
		_dirty = true
		color = value

@export var is_color_palette := false:
	set(value):
		_dirty = true
		is_color_palette = value

var _dirty := false


func _ready() -> void:
	self.toggled.connect(_on_toggled)
	$PressedBorder.hide()
	$SelectedBorder.hide()


func _on_toggled(is_toggled: bool) -> void:
	if is_toggled:
		$SelectedBorder.show()
	else:
		$SelectedBorder.hide()


func _process(_delta: float) -> void:
	if not _dirty:
		return

	$ColorButton.modulate = color
	%ColorSwatch.visible = is_color_palette
