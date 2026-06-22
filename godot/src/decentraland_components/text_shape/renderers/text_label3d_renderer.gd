## text_label3d_renderer.gd
## The original TextShape renderer: a built-in Label3D wrapped in the common
## renderer interface (setup / apply / set_active). On a stock engine it renders PLAIN
## text (Unity tags stripped, first inline color applied to the whole label). On our
## custom Godot fork, Label3D gained a `bbcode_enabled` property + inline [b]/[i]/
## [color]/[font_size] parsing — when that's available this tier feeds it the full
## converted markup, so it renders rich text natively (no SubViewport, no MultiMesh).
extends Node3D

var _label: Label3D
var _font: Font
# Real bold face for [b] markup (multi-font approach), set as Label3D.bold_font on the fork so
# bold uses a true bold weight instead of synthetic embolden (which mangles MSDF glyphs).
var _bold_font: Font
var _bold_font_supported := false
# True when the fork exposes the float outline (set_outline_size_float), which overrides the
# integer outline_size and avoids whole-pixel stepping / MSDF saturation.
var _outline_float_supported := false
# True when the running engine is our fork with inline-BBCode Label3D (feature-detected
# so the same code still works on a stock 4.6.2 that lacks the property).
var _bbcode_supported := false
# True when the fork exposes the native Label3D `character_spacing` (px) property. When present
# we use it instead of baking glyph spacing into a FontVariation (the engine applies the same
# SPACING_GLYPH internally, so doing both would double the spacing).
var _char_spacing_supported := false
# True when the fork exposes Label3D drop-shadow (shadow_color / shadow_offset /
# shadow_outline_size). The shadow draws behind text+outline (ordered by the fork's per-priority
# Z-shift under DISCARD).
var _shadow_supported := false


func setup(font: Font, bold_font: Font = null) -> void:
	_font = font
	_bold_font = bold_font
	if _label != null:
		# Re-seed of an existing label (font family changed): update the bold face live.
		if _bold_font_supported:
			_label.bold_font = _bold_font
		return
	_label = Label3D.new()
	_label.name = "Label3D"
	# Matches the validated BetterLabel3D test scene: DISCARD + outline_render_priority=1 +
	# outline_size_float. The fork orders the shadow/outline/fill layers under DISCARD via a
	# small per-priority Z-shift (label_3d.cpp), so outline_render_priority=1 lifts the outline
	# ring correctly relative to the fill (no z-fight, no outline-over-glyph).
	_label.alpha_cut = Label3D.ALPHA_CUT_DISCARD
	# Hard-cutoff alpha: keep text visible as long as possible as opacity drops (low scissor
	# threshold), then let it vanish entirely once the per-pixel alpha (glyph coverage x opacity)
	# falls below it — i.e. when the TextShape opacity is very low (< ~0.1).
	_label.alpha_scissor_threshold = 0.1
	_bbcode_supported = _label.has_method("set_bbcode_enabled")
	if _bbcode_supported:
		_label.set_bbcode_enabled(true)
	_char_spacing_supported = _label.has_method("set_character_spacing")
	_shadow_supported = _label.has_method("set_shadow_color")
	_bold_font_supported = _label.has_method("set_bold_font")
	if _bold_font_supported:
		_label.bold_font = _bold_font
	# Float outline (fork): smooth sub-pixel width, no integer stepping / saturation.
	_outline_float_supported = _label.has_method("set_outline_size_float")
	_label.outline_render_priority = 1
	# Disable Label3D's edge-space trimming: with it on, a "\n\n" blank line is
	# trimmed away. We do our own per-line trimming below so trailing spaces still
	# don't skew alignment, staying consistent with the MultiMesh/Viewport tiers.
	_label.autowrap_trim_flags = 0
	add_child(_label)


func apply(resolved: Dictionary) -> void:
	if _label == null:
		setup(_font)
	_label.pixel_size = resolved.pixel_size
	var sp: int = resolved.get("glyph_spacing_px", 0)
	if _font != null:
		# Glyph spacing: prefer the fork's native character_spacing; otherwise bake it into a
		# FontVariation (styled_font with 0 returns _font as-is).
		_label.font = GlyphMap.styled_font(
			_font, false, false, 0 if _char_spacing_supported else sp
		)
	if _char_spacing_supported:
		_label.character_spacing = sp
	# Drop shadow (transparent shadow_color = off). Offset px, outline_size = blur spread.
	if _shadow_supported:
		_label.shadow_color = resolved.get("shadow_color", Color(0, 0, 0, 0))
		_label.shadow_offset = resolved.get("shadow_offset", Vector2.ZERO)
		_label.shadow_outline_size = resolved.get("shadow_outline_size", 0.0)
	# Rich markup when the fork supports it (per-line edge-trimmed so trailing spaces don't
	# skew alignment, matching the other tiers); plain stripped text otherwise.
	if _bbcode_supported:
		_label.text = _trim_line_edges(
			TextMarkup.to_bbcode(resolved.raw_text, resolved.godot_font_size)
		)
	else:
		_label.text = _trim_line_edges(resolved.plain_text)
	_label.modulate = resolved.fill_color
	_label.outline_modulate = resolved.outline_color
	_label.font_size = resolved.godot_font_size
	# Prefer the fork's float outline (smooth, no integer stepping); fall back to the integer
	# outline_size on stock.
	var ow: float = resolved.get("outline_size_f", float(resolved.outline_size))
	if _outline_float_supported:
		# Zero the int outline: the fork falls back to outline_size when outline_size_float == 0
		# (label_3d.cpp), so a lingering int value would draw an outline on the no-outline case.
		_label.outline_size = 0
		_label.outline_size_float = ow
	else:
		_label.outline_size = int(round(ow))
	_label.autowrap_mode = (
		TextServer.AUTOWRAP_WORD_SMART if resolved.text_wrapping else TextServer.AUTOWRAP_OFF
	)
	_label.width = resolved.label_width
	_label.horizontal_alignment = resolved.h_align
	_label.vertical_alignment = resolved.v_align
	_label.position = Vector3(
		resolved.width_meter * resolved.x_pos, resolved.height_meter * resolved.y_pos, 0.0
	)


# Trim each line's edge whitespace ourselves (Label3D's own trimming is disabled so
# blank lines survive), so trailing padding spaces don't bias alignment. Blank lines stay
# blank and keep their row height.
func _trim_line_edges(text: String) -> String:
	var lines := text.split("\n")
	for i in lines.size():
		lines[i] = lines[i].strip_edges()
	return "\n".join(lines)
