## glyph_map.gd
## Shared text-layout core for the MultiMesh renderer (ported from the
## improved-label3d-tests benchmark). Layout is done ONCE on the CPU via
## TextServer (exact glyph positions); the renderer turns that per-glyph result
## into MultiMesh instances. We reuse Godot's existing font atlas pages and bind
## each page's live RID directly (never snapshot — Godot fills pages lazily, so a
## cached image goes stale the moment a later string needs a new glyph).
class_name GlyphMap
extends RefCounted

# Shear applied to synthesize italics — the reference faces ship no italic, and the
# Unity reference faux-italicizes too.
const ITALIC_SHEAR := Transform2D(Vector2(1, 0), Vector2(0.25, 1), Vector2.ZERO)
# Embolden strength for synthetic bold (serif / monospace ship no Bold weight).
const FAUX_BOLD_EMBOLDEN := 0.6


## One positioned glyph, in font pixels (image-style, y-down from top of box).
class Glyph:
	var atlas_rid: RID  # LIVE atlas-page RID (bind directly; never snapshot)
	var uv_px: Rect2  # pixel rect of the glyph inside the atlas page
	var tex_size: Vector2  # atlas page size in px (for normalizing uv_px)
	var pos: Vector2  # top-left of the glyph quad, px, y-down from top (left-aligned)
	var size: Vector2  # glyph quad size, px
	var color: Color  # per-glyph color (inline color)
	var line: int  # which line this glyph belongs to (index into line_widths)


# Cache of styled faces, keyed by base face path + bold/italic flags + glyph spacing.
static var _styled_cache: Dictionary = {}


static func _ts() -> TextServer:
	return TextServerManager.get_primary_interface()


## Styled face for `base`: bold (synthetic embolden — no Bold face ships, and the shared
## Inter-Bold must stay raster for the UI; the reference faux-bolds too), italic (synthetic
## shear), and extra per-glyph spacing (px). All three are set on a SINGLE FontVariation —
## nesting variations drops the inner one's properties (spacing/embolden don't propagate
## through base_font). Returns `base` unchanged when nothing is requested.
static func styled_font(base: Font, bold: bool, italic: bool, glyph_spacing: int = 0) -> Font:
	if not bold and not italic and glyph_spacing == 0:
		return base
	var key := "%s_%d_%d_%d" % [base.resource_path, int(bold), int(italic), glyph_spacing]
	if _styled_cache.has(key):
		return _styled_cache[key]
	var fv := FontVariation.new()
	fv.base_font = base
	if bold:
		fv.variation_embolden = FAUX_BOLD_EMBOLDEN
	if italic:
		fv.variation_transform = ITALIC_SHEAR
	if glyph_spacing != 0:
		fv.set_spacing(TextServer.SPACING_GLYPH, glyph_spacing)
	_styled_cache[key] = fv
	return fv


## Shape `markup` and return per-glyph layout. Glyphs are positioned LEFT-aligned
## within their line; horizontal alignment is the renderer's job (each glyph carries
## its `line` index, and `line_widths` gives every line's width). Inline color / size
## / bold / italic are applied per span (TextMarkup.parse_spans). Honours explicit
## "\n" always; if `wrap_width > 0`, word-wraps to that width. Returns
## { glyphs: Array[Glyph], line_widths: PackedFloat32Array, width (widest line),
## height, ascent, descent } (sizes in font px).
static func build(
	markup: String,
	font: Font,
	font_size: int,
	color: Color = Color.WHITE,
	wrap_width: float = 0.0,
	line_spacing: float = 0.0,
	max_lines: int = 0,
	glyph_spacing: int = 0
) -> Dictionary:
	var ts := _ts()

	# Split styled spans into paragraphs at explicit "\n". TextServer's mandatory
	# line-breaking collapses consecutive "\n", dropping blank lines, so we handle
	# the hard breaks ourselves and only let TextServer do soft (word) wrapping.
	var paragraphs: Array = [[]]
	for span: TextMarkup.Span in TextMarkup.parse_spans(markup, color, font_size):
		var parts := span.text.split("\n")
		for pi in parts.size():
			if pi > 0:
				paragraphs.append([])
			if parts[pi] != "":
				var sub := TextMarkup.Span.new()
				sub.text = parts[pi]
				sub.color = span.color
				sub.size = span.size
				sub.bold = span.bold
				sub.italic = span.italic
				(paragraphs[-1] as Array).append(sub)
	# Trim each paragraph's edge spaces (matches TextServer's BREAK_TRIM_EDGE_SPACES).
	for para: Array in paragraphs:
		if not para.is_empty():
			para[0].text = (para[0].text as String).lstrip(" \t")
			para[-1].text = (para[-1].text as String).rstrip(" \t")

	var default_ascent := font.get_ascent(font_size)
	var default_descent := font.get_descent(font_size)

	var glyphs: Array[Glyph] = []
	var line_widths := PackedFloat32Array()
	var max_w := 0.0
	var cursor_y := 0.0
	var first_ascent := 0.0
	var last_descent := 0.0
	var line_index := 0
	var done := false

	for para: Array in paragraphs:
		if done:
			break
		# Build this paragraph's line RIDs (word-wrapped when wrap_width > 0). A blank
		# paragraph yields a single empty line that still occupies a row.
		var lines: Array[RID] = []
		var char_colors := PackedColorArray()
		var shaped := RID()
		if not para.is_empty():
			shaped = ts.create_shaped_text()
			var para_len := 0
			for sub: TextMarkup.Span in para:
				var sf := styled_font(font, sub.bold, sub.italic, glyph_spacing)
				ts.shaped_text_add_string(shaped, sub.text, sf.get_rids(), sub.size)
				for _k in (sub.text as String).length():
					char_colors.append(sub.color)
				para_len += (sub.text as String).length()
			if wrap_width > 0.0:
				var flags := TextServer.BREAK_WORD_BOUND | TextServer.BREAK_TRIM_EDGE_SPACES
				var breaks := ts.shaped_text_get_line_breaks(shaped, wrap_width, 0, flags)
				for i in range(0, breaks.size(), 2):
					lines.append(
						ts.shaped_text_substr(shaped, breaks[i], breaks[i + 1] - breaks[i])
					)
			if lines.is_empty():
				lines.append(ts.shaped_text_substr(shaped, 0, para_len))
		else:
			lines.append(RID())  # blank line sentinel

		for line: RID in lines:
			if max_lines > 0 and line_index >= max_lines:
				if line.is_valid():
					ts.free_rid(line)
				done = true
				continue
			var blank := not line.is_valid()
			var l_ascent := default_ascent if blank else ts.shaped_text_get_ascent(line)
			var l_descent := default_descent if blank else ts.shaped_text_get_descent(line)
			var l_width := 0.0 if blank else ts.shaped_text_get_width(line)
			line_widths.append(l_width)
			max_w = maxf(max_w, l_width)
			if line_index == 0:
				first_ascent = l_ascent
			last_descent = l_descent

			if not blank:
				var baseline := cursor_y + l_ascent
				var pen_x := 0.0
				for g in ts.shaped_text_get_glyphs(line):
					var frid: RID = g["font_rid"]
					var gindex: int = g["index"]
					var advance: float = g["advance"]

					if not frid.is_valid() or gindex == 0:
						pen_x += advance
						continue

					var fsize := Vector2i(g["font_size"], 0)
					var tex_rid := ts.font_get_glyph_texture_rid(frid, fsize, gindex)
					if not tex_rid.is_valid():
						pen_x += advance
						continue

					var gsize := ts.font_get_glyph_size(frid, fsize, gindex)
					if gsize.x <= 0.0 or gsize.y <= 0.0:
						pen_x += advance
						continue

					var shape_off: Vector2 = g["offset"]
					var ci: int = g["start"]
					var gl := Glyph.new()
					gl.atlas_rid = tex_rid
					gl.uv_px = ts.font_get_glyph_uv_rect(frid, fsize, gindex)
					gl.tex_size = ts.font_get_glyph_texture_size(frid, fsize, gindex)
					gl.pos = (
						Vector2(pen_x, baseline)
						+ shape_off
						+ ts.font_get_glyph_offset(frid, fsize, gindex)
					)
					gl.size = gsize
					gl.color = char_colors[ci] if ci >= 0 and ci < char_colors.size() else color
					gl.line = line_index
					glyphs.append(gl)

					pen_x += advance

				ts.free_rid(line)

			cursor_y += l_ascent + l_descent + line_spacing
			line_index += 1

		if shaped.is_valid():
			ts.free_rid(shaped)

	var height := cursor_y - line_spacing if line_index > 0 else 0.0
	return {
		"glyphs": glyphs,
		"line_widths": line_widths,
		"width": max_w,
		"height": height,
		"ascent": first_ascent,
		"descent": last_descent,
	}
