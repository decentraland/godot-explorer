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
var _font_src: String = ""
var _custom_font: FontFile
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
	var src: String = _params.get("font_src", "")
	if fi == _font_index and src == _font_src and _font != null:
		return
	_font_index = fi
	if src != _font_src:
		_font_src = src
		_custom_font = null
		if not src.is_empty():
			_request_custom_font(src)
	_apply_fonts()


## The scene TTF named by `font_src` once loaded (built-in face as glyph fallback),
## otherwise the built-in face for the `font` enum.
func _apply_fonts() -> void:
	if _custom_font != null:
		var fallbacks: Array[Font] = [TextLayout.load_font(_font_index)]
		var variation := FontVariation.new()
		variation.base_font = _custom_font
		variation.fallbacks = fallbacks
		variation.set_meta(TextLayout.CUSTOM_FONT_META, true)
		_font = variation
		# A scene TTF has a single face, and faux bold mangles MSDF glyphs.
		_bold_font = variation
	else:
		_font = TextLayout.load_font(_font_index)
		_bold_font = TextLayout.load_bold_font(_font_index)
	# Re-seed the renderer with the new font + bold face if it already exists.
	if _renderer != null:
		_renderer.setup(_font, _bold_font)


func _request_custom_font(src: String) -> void:
	var promise: Promise = Global.content_provider.fetch_font(src, _params["content_mapping"], true)
	if promise.is_resolved():
		_custom_font = promise.get_data() as FontFile
	else:
		var on_loaded := _on_custom_font_loaded.bind(promise, src)
		if not promise.on_resolved.is_connected(on_loaded):
			promise.on_resolved.connect(on_loaded, CONNECT_ONE_SHOT)


func _on_custom_font_loaded(promise: Promise, src: String) -> void:
	if src != _font_src:
		return
	_custom_font = promise.get_data() as FontFile
	if _custom_font == null:
		return
	_apply_fonts()
	if _renderer != null:
		_resolved = TextLayout.resolve(_params, _font)
		_renderer.apply(_resolved)
