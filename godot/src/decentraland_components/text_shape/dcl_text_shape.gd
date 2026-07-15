## dcl_text_shape.gd
## TextShape orchestrator. Rust instantiates this scene per TextShape entity and calls
## `apply(params)` with the raw PbTextShape fields. Rendering uses a single Label3D-based
## renderer; all sizing / markup / color resolution lives in TextLayout. (There is no LOD /
## tier system — Label3D is the only renderer.)
class_name DclTextShape
extends Node3D

const LABEL3D_RENDERER := preload(
	"res://src/decentraland_components/text_shape/renderers/text_label3d_renderer.gd"
)

var _params: Dictionary = {}
var _resolved: Dictionary = {}
var _font: Font
var _bold_font: Font
var _font_index: int = -1
var _renderer: Node3D


## Called from Rust each time the TextShape component changes. `params` carries the
## raw PbTextShape fields plus `has_*` flags for the optional ones.
func apply(params: Dictionary) -> void:
	_params = params
	_resolve_font()
	_resolved = TextLayout.resolve(_params, _font)
	_ensure_renderer()
	_renderer.apply(_resolved)


func _ensure_renderer() -> void:
	if _renderer != null:
		return
	_renderer = LABEL3D_RENDERER.new()
	_renderer.name = "Label3DRenderer"
	add_child(_renderer)
	_renderer.setup(_font, _bold_font)


func _resolve_font() -> void:
	var fi: int = _params.get("font", 0) if _params.get("has_font", false) else 0
	if fi == _font_index and _font != null:
		return
	_font_index = fi
	_font = TextLayout.load_font(fi)
	_bold_font = TextLayout.load_bold_font(fi)
	# Re-seed the renderer with the new font + bold face if it already exists.
	if _renderer != null:
		_renderer.setup(_font, _bold_font)
