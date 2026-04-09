@tool
class_name AnimatedTextureRect
extends TextureRect

@export var atlas: Texture2D:
	set(value):
		atlas = value
		_setup_texture()

@export var frame_count: int = 1:
	set(value):
		frame_count = max(1, value)
		_current_frame = 0
		_update_frame()

@export var x_offset: int = 0:
	set(value):
		x_offset = value
		_update_frame()

@export var y_offset: int = 0:
	set(value):
		y_offset = value
		_update_frame()

@export var fps: float = 12.0

var _atlas_tex: AtlasTexture
var _current_frame: int = 0
var _elapsed: float = 0.0
var _playing_once: bool = false


func _ready() -> void:
	_setup_texture()


func _setup_texture() -> void:
	if atlas == null:
		texture = null
		_atlas_tex = null
		return
	_atlas_tex = AtlasTexture.new()
	_atlas_tex.atlas = atlas
	texture = _atlas_tex
	_current_frame = 0
	_update_frame()


func _update_frame() -> void:
	if _atlas_tex == null or atlas == null:
		return
	var frame_w: int = x_offset if x_offset > 0 else atlas.get_width()
	var frame_h: int = y_offset if y_offset > 0 else atlas.get_height()
	_atlas_tex.region = Rect2(_current_frame * x_offset, _current_frame * y_offset, frame_w, frame_h)


func play() -> void:
	_current_frame = 0
	_elapsed = 0.0
	_playing_once = true
	_update_frame()


func _process(delta: float) -> void:
	if not _playing_once or _atlas_tex == null or fps <= 0.0:
		return
	_elapsed += delta
	if _elapsed >= 1.0 / fps:
		_elapsed -= 1.0 / fps
		if _playing_once:
			if _current_frame >= frame_count - 1:
				_playing_once = false
				return
		_current_frame = (_current_frame + 1) % frame_count
		_update_frame()
