## glyph_map.gd
## Shared font-styling helper for the TextShape renderer: produces a `FontVariation`
## with synthetic bold (embolden), synthetic italic (shear), and extra per-glyph
## spacing, caching the result per base face + flags. Used by the Label3D renderer
## to style [b]/[i] markup when no real bold/italic face is available.
class_name GlyphMap
extends RefCounted

# Shear applied to synthesize italics — the reference faces ship no italic, and the
# Unity reference faux-italicizes too.
const ITALIC_SHEAR := Transform2D(Vector2(1, 0), Vector2(0.25, 1), Vector2.ZERO)
# Embolden strength for synthetic bold (serif / monospace ship no Bold weight).
const FAUX_BOLD_EMBOLDEN := 0.6

# Cache of styled faces, keyed by base face path + bold/italic flags + glyph spacing.
static var _styled_cache: Dictionary = {}


## Styled face for `base`: bold (synthetic embolden — no Bold face ships, and the shared
## Inter-Bold must stay raster for the UI; the reference faux-bolds too), italic (synthetic
## shear), and extra per-glyph spacing (px). All three are set on a SINGLE FontVariation —
## nesting variations drops the inner one's properties (spacing/embolden don't propagate
## through base_font). Returns `base` unchanged when nothing is requested.
static func styled_font(base: Font, bold: bool, italic: bool, glyph_spacing: int = 0) -> Font:
	if not bold and not italic and glyph_spacing == 0:
		return base
	# A scene font arrives as a plain FontVariation (no resource_path) over a shared FontFile:
	# style that FontFile directly and keep the variation's glyph fallbacks.
	var source := base
	var face_id: String = base.resource_path
	if base.has_meta(TextLayout.CUSTOM_FONT_META):
		source = (base as FontVariation).base_font
		# The cached variation holds `source`, so its instance id stays unique while cached.
		# Never evicted, like the content provider's font promises: it grows with each scene
		# visited that styles a custom font (at most that scene's font budget).
		face_id = "%d_%s" % [source.get_instance_id(), base.fallbacks[0].resource_path]
	var key := "%s_%d_%d_%d" % [face_id, int(bold), int(italic), glyph_spacing]
	if _styled_cache.has(key):
		return _styled_cache[key]
	var fv := FontVariation.new()
	fv.base_font = source
	if source != base:
		fv.fallbacks = base.fallbacks
		fv.set_meta(TextLayout.CUSTOM_FONT_META, true)
	if bold:
		fv.variation_embolden = FAUX_BOLD_EMBOLDEN
	if italic:
		fv.variation_transform = ITALIC_SHEAR
	if glyph_spacing != 0:
		fv.set_spacing(TextServer.SPACING_GLYPH, glyph_spacing)
	_styled_cache[key] = fv
	return fv
