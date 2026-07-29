## text_layout.gd
## Shared TextShape sizing/alignment/color resolution, ported verbatim from the
## previous Rust implementation in `lib/src/scene_runner/components/text_shape.rs`.
## Rust now only forwards the raw PbTextShape fields (with `has_*` flags for the
## optional ones); every default and bit of TMP-matching math lives here so all
## three renderers size and color identically.
class_name TextLayout
extends RefCounted

## Unity TMP `fontSize` -> Godot glyph pixels, per font face (keyed by .ttf filename).
## ONE combined number per face — it folds the old base conversion (22) x pitch-match factor
## (0.887) = 19.514, AND a per-font glyph-size correction, into a single multiplier. Calibrated
## on the measurement grid (issue #2371): at the shared 19.514, Godot rendered glyphs OVERSIZED
## vs Unity (sans +7.0%, serif +3.7%, mono +2.0%), so each face is divided by its ratio:
## 19.514 / 1.070, / 1.0373, / 1.0205.
##
## This matches GLYPH SIZE, which also shrinks the line pitch — restored per-face by
## TEXTSHAPE_LINE_SPACING_EM below.
const TMP_TO_LABEL3D_FONT_SIZE := {
	"Inter-SemiBold.ttf": 18.242,
	"NotoSerif.ttf": 18.812,
	"AtkinsonHyperlegibleMono.ttf": 19.122,
}
const TMP_TO_LABEL3D_FONT_SIZE_DEFAULT: float = 19.514

## Unity TMP autosize bounds in TMP `fontSize` units.
const UNITY_TMP_FONT_SIZE_MIN: float = 18.0
const UNITY_TMP_FONT_SIZE_MAX: float = 72.0

## Scales the Unity TMP outline width to Godot's pixel `outline_size`. Calibrated by eye against
## the Label3D tier vs the Unity reference (0.2 base x the 10% tuning that was dialed in).
const TMP_TO_LABEL3D_OUTLINE_WIDTH: float = 0.02

## Extra per-glyph spacing as per-mille of the em (scales with font size), per font face.
## Godot was rendering text WIDER than Unity (Sans ~2.3%, Serif ~6.2%, Mono ~6.5% on the
## "Hamburgefonstiv" width test, issue #2371), i.e. too much tracking — so the old global 75
## is reduced per face by the measured excess. Mono needed a second pass (still ~0.7% wide at
## 33) down to 28. Final: Sans 61, Serif 38, Mono 28.
const GLYPH_SPACING_PER_MILLE := {
	"Inter-SemiBold.ttf": 61.0,
	"NotoSerif.ttf": 38.0,
	"AtkinsonHyperlegibleMono.ttf": 28.0,
}
const GLYPH_SPACING_PER_MILLE_DEFAULT: float = 75.0

## Unity TMP rect width (meters) -> Godot Label3D `width` (wrap width).
const TMP_TO_LABEL3D_WIDTH: float = 200.0

## Drop-shadow conversion (Label3D fork: shadow_offset is px, shadow_outline_size is px spread).
## Offsets/blur are expressed as a fraction of the em, scaled by font size like the outline.
## Calibrated by eye against the Unity reference (offset = base x the 5% tuning that was dialed in).
const TMP_TO_LABEL3D_SHADOW_OFFSET: float = 0.05
const TMP_TO_LABEL3D_SHADOW_BLUR: float = 0.02

## Meters per glyph pixel. Matches the Label3D `pixel_size` the renderers set, so
## the MultiMesh/Viewport quads land at the same world size as the Label3D tier.
const PIXEL_SIZE: float = 0.005

## MSDF atlas generation params — must match the fonts' .import (msdf_size / msdf_pixel_range).
## Pixel range raised from 8 -> 32 so thick outlines / shadow blur don't saturate the field
## (the SDF can represent a wider distance from the glyph edge).
const MSDF_SIZE: float = 48.0
const MSDF_PIXEL_RANGE: float = 32.0

## Per-face vertical anchor correction (fraction of the em) between Godot's Label3D and
## Unity's TMP, applied to the first line so the two clients position TextShape blocks at the
## same height (issue #2371).
##
## Why it's needed: with the font matched (Inter v3.3) and the line pitch matched, a
## measurement grid showed Unity's TOP-anchored text still hangs a little LOWER than Godot's —
## a difference in how each engine's TOP alignment places the first
## line (TMP anchors via the rect/pivot, Label3D hangs from the ascent line). It's a constant
## (non-accumulating) offset, so not a drift bug, but it forces a different startY per client.
## We shift Godot down by it to match Unity. Values below are keyed by the face's .ttf filename
## and were calibrated on the grid across sizes 12 & 24 (Unity-minus-Godot startY / world-per-em,
## averaged over the two sizes). Re-derived after the per-font glyph-size shrink changed the
## font size: sans ~0.088, serif ~0.064, mono ~0.029 em.
##
## RELATED, SEPARATE effect (documented here so this isn't rediscovered): Godot quantizes the
## font's ascent/descent to whole pixels at MSDF_SIZE when it bakes the MSDF atlas —
## get_ascent(size) == ceil(true_em * MSDF_SIZE) / MSDF_SIZE * size — inflating the line box
## ~2-3% vs the font's true hhea metrics (Inter: ascent 0.962 -> 47/48 = 0.979 em, descent
## 0.234 -> 12/48 = 0.250 em at MSDF_SIZE 48). Unity/TMP uses the TRUE metrics. That inflation
## is real, but it is SMALL and pushes Godot's anchor the OPPOSITE way from the gap above (it
## makes Godot hang slightly lower, not higher), so it is not the cause and is not corrected
## here. Raising MSDF_SIZE would shrink it but costs ~(new/48)^2 atlas memory (rejected).
const TEXTSHAPE_ANCHOR_OFFSET_EM := {
	"Inter-SemiBold.ttf": 0.088,
	"NotoSerif.ttf": 0.064,
	"AtkinsonHyperlegibleMono.ttf": 0.029,
}
const TEXTSHAPE_ANCHOR_OFFSET_DEFAULT: float = 0.088

## Per-face line-spacing (fraction of the em) ADDED to Godot's line advance so the line PITCH
## matches Unity after the per-font glyph-size shrink (which shrank the pitch too). Verified on
## the live client that Label3D.line_spacing works (adds space BELOW each line; the first line's
## top is unaffected, so this is independent of the anchor offset above). Calibrated on the grid
## (issue #2371) across sizes 12 & 24: sans ~0.086, serif ~0.056, mono ~0.031 em.
const TEXTSHAPE_LINE_SPACING_EM := {
	"Inter-SemiBold.ttf": 0.086,
	"NotoSerif.ttf": 0.056,
	"AtkinsonHyperlegibleMono.ttf": 0.031,
}
const TEXTSHAPE_LINE_SPACING_DEFAULT: float = 0.0

# TextShape faces, matching the Unity reference's Font List (the SDF font asset each Font enum
# maps to): F_SANS_SERIF -> Inter SemiBold, F_SERIF -> Noto Serif, F_MONOSPACE -> Atkinson
# Hyperlegible Mono. Dedicated TextShape-only files under assets/themes/fonts/text_shape/, all
# imported as MSDF and FLATTENED (contours unioned via FontForge removeOverlap) so MSDF
# generation doesn't choke on overlapping contours. Referenced by UID so they survive file
# moves/renames; the comment is the source filename for reference.
#
# F_SANS_SERIF uses Inter Semi Bold v3.3 (UPM 2816), matched to the exact build the Unity
# reference bakes its "Inter-SemiBold SDF" atlas from — Inter's legacy "Inter UI"-era release.
# We previously shipped the Inter v4 "24pt" optical cut (wrong letterforms) and then Inter
# v3.019 (right family, later version, subtly different glyphs); v3.3 is the version-accurate
# match (issue #2371).
const FONT_SANS: String = "uid://dsp1v3gwqgdvu"  # Inter-SemiBold.ttf (v3.3, flattened)
const FONT_SERIF: String = "uid://d4f8lc838jeet"  # NotoSerif.ttf
const FONT_MONO: String = "uid://4ldm8tygdnbc"  # AtkinsonHyperlegibleMono.ttf

# Real bold faces per family (flattened, MSDF). Used as Label3D's `bold_font` so [b] markup
# renders a true bold weight instead of synthetic embolden (which mangles MSDF glyphs).
const FONT_SANS_BOLD: String = "uid://ru8tau3y1nqn"  # Inter-Bold.ttf (v3.3, flattened)
const FONT_SERIF_BOLD: String = "uid://1vmt0d4x5gr1"  # NotoSerif-Bold.ttf
const FONT_MONO_BOLD: String = "uid://bfqjyubth6dkc"  # AtkinsonHyperlegibleMono-Bold.ttf


## Looks up a per-face value in `table` by the font's .ttf filename, returning `fallback` for
## an unknown or null face. Shared by all the per-font correction tables above.
static func _per_font(table: Dictionary, font: Font, fallback: float) -> float:
	if font != null:
		var key := String(font.resource_path).get_file()
		if table.has(key):
			return table[key]
	return fallback


## font == null uses the shared default multiplier (e.g. inline <size> markup, which has no
## face in scope); the per-font glyph correction only applies when a face is passed.
static func unity_to_godot_font_size(tmp_font_size: float, font: Font = null) -> float:
	return (
		tmp_font_size * _per_font(TMP_TO_LABEL3D_FONT_SIZE, font, TMP_TO_LABEL3D_FONT_SIZE_DEFAULT)
	)


## Local-space (pre-entity-scale) Y offset in meters to add to the Label3D position so its
## anchored text lands where Unity TMP places it. Only the TOP anchor is grid-calibrated (the
## ranking-board case); BOTTOM gets the mirror correction and MIDDLE is left unchanged
## (centering matches closely). Positive Y is up.
static func anchor_offset(font: Font, godot_font_size: int, v_align: int) -> float:
	var em := _per_font(TEXTSHAPE_ANCHOR_OFFSET_EM, font, TEXTSHAPE_ANCHOR_OFFSET_DEFAULT)
	var dy: float = em * float(godot_font_size) * PIXEL_SIZE
	match v_align:
		VERTICAL_ALIGNMENT_TOP:
			return -dy  # Unity hangs lower -> move Godot down to match.
		VERTICAL_ALIGNMENT_BOTTOM:
			return dy
		_:
			return 0.0


## Extra line spacing (glyph pixels) added to Label3D so the line pitch matches Unity after the
## per-font glyph-size shrink. Keyed by the face's .ttf filename.
static func line_spacing_px(font: Font, godot_font_size: int) -> float:
	var em := _per_font(TEXTSHAPE_LINE_SPACING_EM, font, TEXTSHAPE_LINE_SPACING_DEFAULT)
	return em * float(godot_font_size)


static func unity_to_godot_outline_size(godot_font_size: float, tmp_outline_width: float) -> float:
	return godot_font_size * tmp_outline_width * TMP_TO_LABEL3D_OUTLINE_WIDTH


static func unity_to_godot_label_width(tmp_width: float) -> float:
	return tmp_width * TMP_TO_LABEL3D_WIDTH


## Unity TMP shadow offset (em fraction) -> Godot Label3D `shadow_offset` px component.
static func unity_to_godot_shadow_offset(godot_font_size: float, tmp_offset: float) -> float:
	return godot_font_size * tmp_offset * TMP_TO_LABEL3D_SHADOW_OFFSET


## Unity TMP shadow blur -> Godot Label3D `shadow_outline_size` px (the shadow's spread).
static func unity_to_godot_shadow_blur(godot_font_size: float, tmp_blur: float) -> float:
	return godot_font_size * tmp_blur * TMP_TO_LABEL3D_SHADOW_BLUR


## Half-width of the edge-centered MSDF outline, in signed-distance units (0..1, edge=0.5).
## The outline thickness is the same em fraction as the Label3D px outline
## (outline_width * TMP_TO_LABEL3D_OUTLINE_WIDTH), converted into the MSDF distance domain
## via the atlas em (MSDF_SIZE texels) and field range (MSDF_PIXEL_RANGE texels).
static func unity_to_msdf_outline_half(tmp_outline_width: float) -> float:
	var outline_em := tmp_outline_width * TMP_TO_LABEL3D_OUTLINE_WIDTH
	return 0.5 * outline_em * MSDF_SIZE / MSDF_PIXEL_RANGE


## World meters -> glyph pixels (PIXEL_SIZE is meters per pixel).
static func world_to_pixels(meters: float) -> float:
	return meters / PIXEL_SIZE


## Glyph pixels -> world meters.
static func pixels_to_world(pixels: float) -> float:
	return pixels * PIXEL_SIZE


## Regular face for a Font enum value (0 sans, 1 serif, 2 monospace), matching the
## Unity reference's Font List mapping.
static func load_font(font_index: int) -> Font:
	match font_index:
		1:
			return load(FONT_SERIF) as Font
		2:
			return load(FONT_MONO) as Font
		_:
			return load(FONT_SANS) as Font


## Real bold face for a Font enum value (0 sans, 1 serif, 2 monospace), used for [b] markup.
static func load_bold_font(font_index: int) -> Font:
	match font_index:
		1:
			return load(FONT_SERIF_BOLD) as Font
		2:
			return load(FONT_MONO_BOLD) as Font
		_:
			return load(FONT_SANS_BOLD) as Font


## Resolve the raw params + font into a flat Dictionary the renderers consume.
## Keys: godot_font_size:int, outline_size:int, fill_color:Color, outline_color:Color,
## plain_text:String, raw_text:String, h_align/v_align (Godot alignment enums),
## x_pos/y_pos (offset fractions), width_meter/height_meter:float, label_width:float,
## text_wrapping:bool, line_spacing:float, pixel_size:float.
static func resolve(params: Dictionary, font: Font) -> Dictionary:
	var raw_text: String = params.get("text", "")

	var opacity := 1.0
	var text_color: Color = Color.WHITE
	if params.get("has_text_color", false):
		text_color = params.get("text_color", Color.WHITE)
		opacity = text_color.a

	# Strip tags for the plain (Label3D) text and pull the first inline color.
	var stripped := TextMarkup.strip_to_plain(raw_text)
	var plain_text: String = stripped.text
	var tag_color = stripped.color

	var fill_rgb: Color = tag_color if tag_color != null else text_color
	var fill_color := Color(fill_rgb.r, fill_rgb.g, fill_rgb.b, opacity)

	var outline_rgb: Color = Color.WHITE
	if params.get("has_outline_color", false):
		outline_rgb = params.get("outline_color", Color.WHITE)
	var outline_color := Color(outline_rgb.r, outline_rgb.g, outline_rgb.b, opacity)

	var text_align: int = params.get("text_align", 4) if params.get("has_text_align", false) else 4
	var text_wrapping: bool = (
		params.get("text_wrapping", false) if params.get("has_text_wrapping", false) else false
	)
	var font_auto_size: bool = (
		params.get("font_auto_size", false) if params.get("has_font_auto_size", false) else false
	)

	var godot_font_size: float
	if font_auto_size:
		# Replicates Unity's reference client (TMPProSdkExtensions.cs): autosize fits
		# text inside (width, height) only when text_wrapping is set; otherwise the
		# rect is degenerate and TMP falls back to fontSizeMin.
		var fit_w: float = (
			(params.get("width", 1.0) if params.get("has_width", false) else 1.0)
			if text_wrapping
			else 0.0
		)
		var fit_h: float = (
			(params.get("height", 1.0) if params.get("has_height", false) else 1.0)
			if text_wrapping
			else 0.0
		)
		if fit_w <= 0.0 or fit_h <= 0.0:
			godot_font_size = unity_to_godot_font_size(UNITY_TMP_FONT_SIZE_MIN, font)
		else:
			godot_font_size = float(
				_compute_auto_font_size(
					font,
					plain_text,
					fit_w,
					fit_h,
					int(unity_to_godot_font_size(UNITY_TMP_FONT_SIZE_MIN, font)),
					int(unity_to_godot_font_size(UNITY_TMP_FONT_SIZE_MAX, font)),
				)
			)
	else:
		var tmp_size: float = (
			params.get("font_size", 3.0) if params.get("has_font_size", false) else 3.0
		)
		godot_font_size = maxf(unity_to_godot_font_size(tmp_size, font), 1.0)

	var outline_w: float = (
		params.get("outline_width", 0.0) if params.get("has_outline_width", false) else 0.0
	)
	var outline_size := unity_to_godot_outline_size(godot_font_size, outline_w)
	var outline_half := unity_to_msdf_outline_half(outline_w)

	var width_meter: float = (
		(params.get("width", 0.0) if params.get("has_width", false) else 0.0)
		if text_wrapping
		else 0.0
	)
	var height_meter: float = (
		(params.get("height", 0.0) if params.get("has_height", false) else 0.0)
		if text_wrapping
		else 0.0
	)
	var label_width := unity_to_godot_label_width(
		params.get("width", 16.0) if params.get("has_width", false) else 16.0
	)

	var row := text_align / 3  # 0 top, 1 middle, 2 bottom
	var col := text_align % 3  # 0 left, 1 center, 2 right
	var v_align := VERTICAL_ALIGNMENT_TOP
	var y_pos := 0.5
	if row == 1:
		v_align = VERTICAL_ALIGNMENT_CENTER
		y_pos = 0.0
	elif row == 2:
		v_align = VERTICAL_ALIGNMENT_BOTTOM
		y_pos = -0.5
	var h_align := HORIZONTAL_ALIGNMENT_LEFT
	var x_pos := -0.5
	if col == 1:
		h_align = HORIZONTAL_ALIGNMENT_CENTER
		x_pos = 0.0
	elif col == 2:
		h_align = HORIZONTAL_ALIGNMENT_RIGHT
		x_pos = 0.5

	var line_spacing: float = (
		params.get("line_spacing", 0.0) if params.get("has_line_spacing", false) else 0.0
	)

	# Drop shadow (Label3D fork). Enabled when any shadow field is present; uses the given
	# shadow_color (Color3, alpha = the text opacity) or black if only offset/blur were set.
	# When no shadow field is present the alpha is 0 so the renderer draws no shadow.
	var has_shadow: bool = (
		params.get("has_shadow_color", false)
		or params.get("has_shadow_offset_x", false)
		or params.get("has_shadow_offset_y", false)
		or params.get("has_shadow_blur", false)
	)
	var shadow_rgb: Color = (
		params.get("shadow_color", Color.BLACK)
		if params.get("has_shadow_color", false)
		else Color.BLACK
	)
	var shadow_alpha := opacity if has_shadow else 0.0
	var shadow_color := Color(shadow_rgb.r, shadow_rgb.g, shadow_rgb.b, shadow_alpha)
	var shadow_off_x: float = params.get("shadow_offset_x", 0.0) if has_shadow else 0.0
	var shadow_off_y: float = params.get("shadow_offset_y", 0.0) if has_shadow else 0.0
	var shadow_blur: float = params.get("shadow_blur", 0.0) if has_shadow else 0.0
	# Y is inverted to match Unity's shadow direction (positive shadow_offset_y = downward).
	var shadow_offset := Vector2(
		unity_to_godot_shadow_offset(godot_font_size, shadow_off_x),
		-unity_to_godot_shadow_offset(godot_font_size, shadow_off_y)
	)
	var shadow_outline_size := unity_to_godot_shadow_blur(godot_font_size, shadow_blur)

	var spacing_permille := _per_font(
		GLYPH_SPACING_PER_MILLE, font, GLYPH_SPACING_PER_MILLE_DEFAULT
	)
	var glyph_spacing_px := int(round(spacing_permille / 1000.0 * godot_font_size))

	return {
		"godot_font_size": int(godot_font_size),
		"outline_size": int(outline_size),
		"outline_size_f": outline_size,
		"outline_half": outline_half,
		"fill_color": fill_color,
		"outline_color": outline_color,
		"plain_text": plain_text,
		"raw_text": raw_text,
		"h_align": h_align,
		"v_align": v_align,
		"x_pos": x_pos,
		"y_pos": y_pos,
		"width_meter": width_meter,
		"height_meter": height_meter,
		"label_width": label_width,
		"text_wrapping": text_wrapping,
		"line_spacing": line_spacing,
		"pixel_size": PIXEL_SIZE,
		"shadow_color": shadow_color,
		"shadow_offset": shadow_offset,
		"shadow_outline_size": shadow_outline_size,
		"glyph_spacing_px": glyph_spacing_px,
	}


# Largest font_size (px) for which `text` fits in width x height meters, clamped to
# [label_min, label_max]. Word wrap off (only explicit \n). Mirrors the Rust binary
# search in text_shape.rs.
static func _compute_auto_font_size(
	font: Font,
	text: String,
	width_world: float,
	height_world: float,
	label_min: int,
	label_max: int,
) -> int:
	var rect_w_px := maxf(world_to_pixels(width_world), 1.0)
	var rect_h_px := maxf(world_to_pixels(height_world), 1.0)

	var lo := label_min
	var hi := label_max
	var best := label_min
	while lo <= hi:
		var mid := lo + (hi - lo) / 2
		var measured := (
			font
			. get_multiline_string_size(
				text,
				HORIZONTAL_ALIGNMENT_LEFT,
				-1.0,
				mid,
				-1,
				TextServer.BREAK_MANDATORY,
				TextServer.JUSTIFICATION_NONE,
			)
		)
		if measured.x <= rect_w_px and measured.y <= rect_h_px:
			best = mid
			lo = mid + 1
		else:
			hi = mid - 1
	return best
