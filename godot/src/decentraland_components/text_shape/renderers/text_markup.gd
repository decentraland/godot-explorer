## text_markup.gd
## Unity TextMeshPro markup handling for the TextShape renderers, ported from the
## Rust reference in `lib/src/dcl/ui_text_tags.rs` so every renderer agrees on tag
## semantics. Supports <b>, <i>, <color=...> (named / #RGB / #RRGGBB / #RRGGBBAA,
## quoted or spaced) and <size=...> with TMP units (bare = font units * TMP->Godot factor,
## N px, N em, N %, and a leading +/- for relative), all resolved to px since BBCode only
## takes px; unknown tags (<cspace>, ...) are stripped.
##
## Consumers:
##   - to_bbcode(s, base_size) -> RichTextLabel/Label3D BBCode (the live render path)
##   - strip_to_plain(s)       -> { text, color } plain text + first color
##   - parse_spans(s, ...)     -> styled runs for glyph layout (legacy helper)
class_name TextMarkup
extends RefCounted


class Span:
	var text: String
	var color: Color
	var size: int
	var bold: bool
	var italic: bool


## A styled run produced by parse_spans. `color`/`size` default to the caller's
## base values when no inline tag overrides them.
static func parse_spans(s: String, default_color: Color, default_size: int) -> Array:
	var spans: Array = []
	var col := default_color
	var has_col := false
	var siz := default_size
	var bold := false
	var italic := false
	var buf := ""

	for tok in _tokenize(s):
		if tok.kind == "text":
			buf += tok.value
			continue
		# A tag boundary flushes the current run.
		if buf != "":
			var sp := Span.new()
			sp.text = buf
			sp.color = col if has_col else default_color
			sp.size = siz
			sp.bold = bold
			sp.italic = italic
			spans.append(sp)
			buf = ""
		match tok.kind:
			"b_open":
				bold = true
			"b_close":
				bold = false
			"i_open":
				italic = true
			"i_close":
				italic = false
			"color_open":
				col = parse_color(tok.value, default_color)
				has_col = true
			"color_close":
				col = default_color
				has_col = false
			"size_open":
				siz = _resolve_size_px(tok.value, default_size)
			"size_close":
				siz = default_size
	if buf != "":
		var sp := Span.new()
		sp.text = buf
		sp.color = col if has_col else default_color
		sp.size = siz
		sp.bold = bold
		sp.italic = italic
		spans.append(sp)
	return spans


## Convert Unity markup to RichTextLabel BBCode. `base_size` (the resolved font size in px) is
## needed to turn percentage `<size=N%>` tags into the absolute px BBCode only understands.
static func to_bbcode(s: String, base_size: int = 0) -> String:
	var out := ""
	for tok in _tokenize(s):
		match tok.kind:
			"text":
				out += tok.value
			"b_open":
				out += "[b]"
			"b_close":
				out += "[/b]"
			"i_open":
				out += "[i]"
			"i_close":
				out += "[/i]"
			"color_open":
				out += "[color=%s]" % tok.value
			"color_close":
				out += "[/color]"
			"size_open":
				out += "[font_size=%d]" % _resolve_size_px(tok.value, base_size)
			"size_close":
				out += "[/font_size]"
	return out


## Resolve a Unity <size=...> value to absolute pixels (what BBCode/Label3D need). Mirrors TMP:
## optional sign (+/-) then a number then an optional unit:
##   - bare number / N px -> font-size units (TMP fontSize ~= px), scaled by the TMP->Godot factor
##   - N em               -> multiples of the current size (base_size)
##   - N %                -> percentage of base_size; sign ignored (<size=+50%> == 50% == smaller)
## For bare/px/em a leading +/- is relative (base_size +/- the value); no sign = absolute.
## Unparseable values fall back to base_size; the result is clamped to >= 1 px.
static func _resolve_size_px(value: String, base_size: int) -> int:
	var v := value.strip_edges().to_lower()
	if v.is_empty():
		return base_size
	var sign := 0
	if v.begins_with("+"):
		sign = 1
		v = v.substr(1)
	elif v.begins_with("-"):
		sign = -1
		v = v.substr(1)
	var unit := "fu"  # font units (TMP default)
	if v.ends_with("px"):
		unit = "px"
		v = v.substr(0, v.length() - 2)
	elif v.ends_with("em"):
		unit = "em"
		v = v.substr(0, v.length() - 2)
	elif v.ends_with("%"):
		unit = "pct"
		v = v.substr(0, v.length() - 1)
	v = v.strip_edges()
	if not v.is_valid_float():
		return base_size
	var num := v.to_float()
	# Percentage is always an absolute fraction of the base size; TMP ignores the +/- sign
	# (so <size=+50%> is just 50% of base, i.e. smaller — not base + 50%).
	if unit == "pct":
		return maxi(1, int(round(num / 100.0 * float(base_size))))
	# px and bare numbers are both "font-size units" in TMP (fontSize is ~points = px at the
	# default scale), so both go through the TMP->Godot font factor; em multiplies the base.
	var contribution := (
		num * float(base_size) if unit == "em" else TextLayout.unity_to_godot_font_size(num)
	)
	var result := contribution
	if sign == 1:
		result = float(base_size) + contribution
	elif sign == -1:
		result = float(base_size) - contribution
	return maxi(1, int(round(result)))


## Strip every tag, returning { "text": String, "color": Variant }. `color` is the
## first <color=...> found (as a Color) or null when none was present.
static func strip_to_plain(s: String) -> Dictionary:
	var text := ""
	var color = null
	for tok in _tokenize(s):
		if tok.kind == "text":
			text += tok.value
		elif tok.kind == "color_open" and color == null:
			color = parse_color(tok.value, Color.WHITE)
	return {"text": text, "color": color}


## Tokenize Unity markup into a flat list of { kind, value } entries. `kind` is one
## of: text, b_open/b_close, i_open/i_close, color_open(value)/color_close,
## size_open(value)/size_close. Unknown <...> tags are dropped.
static func _tokenize(s: String) -> Array:
	var tokens: Array = []
	var n := s.length()
	var i := 0
	var text_start := 0

	while i < n:
		if s[i] != "<":
			i += 1
			continue
		var tok := _parse_tag(s, i)
		if tok.is_empty():
			i += 1
			continue
		# Emit pending text, then the tag (skip emitting for unknown tags).
		if i > text_start:
			tokens.append({"kind": "text", "value": s.substr(text_start, i - text_start)})
		if tok.kind != "unknown":
			tokens.append(tok)
		i += tok.length
		text_start = i

	if text_start < n:
		tokens.append({"kind": "text", "value": s.substr(text_start, n - text_start)})
	return tokens


# Parse one tag starting at `start` (s[start] == '<'). Returns { kind, value?, length }
# or {} when the text is not a tag at all.
static func _parse_tag(s: String, start: int) -> Dictionary:
	var n := s.length()
	if start + 2 >= n or s[start] != "<":
		return {}

	# Closing tags.
	if s[start + 1] == "/":
		if s.substr(start, 4) == "</b>":
			return {"kind": "b_close", "length": 4}
		if s.substr(start, 4) == "</i>":
			return {"kind": "i_close", "length": 4}
		if s.substr(start, 8) == "</color>":
			return {"kind": "color_close", "length": 8}
		if s.substr(start, 7) == "</size>":
			return {"kind": "size_close", "length": 7}
		return _skip_unknown(s, start)

	if s.substr(start, 3) == "<b>":
		return {"kind": "b_open", "length": 3}
	if s.substr(start, 3) == "<i>":
		return {"kind": "i_open", "length": 3}
	if s.substr(start, 6) == "<color":
		return _parse_kv_tag(s, start, "color")
	if s.substr(start, 5) == "<size":
		return _parse_kv_tag(s, start, "size")
	return _skip_unknown(s, start)


# Parse <name=value>, <name = value> or <name="value">. `name` is "color" or "size".
static func _parse_kv_tag(s: String, start: int, name: String) -> Dictionary:
	var n := s.length()
	var i := start + 1 + name.length()  # skip "<color" / "<size"

	while i < n and s[i] == " ":
		i += 1
	if i >= n or s[i] != "=":
		return _skip_unknown(s, start)
	i += 1
	while i < n and s[i] == " ":
		i += 1

	var has_quotes := i < n and s[i] == '"'
	if has_quotes:
		i += 1
	var value_start := i
	var end_char := '"' if has_quotes else ">"
	while i < n and s[i] != end_char:
		i += 1
	if i >= n:
		return _skip_unknown(s, start)
	var value := s.substr(value_start, i - value_start)
	i += 1  # skip end_char
	if has_quotes:
		while i < n and s[i] == " ":
			i += 1
		if i >= n or s[i] != ">":
			return _skip_unknown(s, start)
		i += 1

	var kind := "color_open" if name == "color" else "size_open"
	var value_out := _convert_color_value(value) if name == "color" else value.strip_edges()
	return {"kind": kind, "value": value_out, "length": i - start}


# Skip an unrecognized <...> tag. Returns an "unknown" token (dropped by the
# tokenizer) or {} when the '<' does not begin a tag-like sequence.
static func _skip_unknown(s: String, start: int) -> Dictionary:
	var n := s.length()
	var i := start + 1
	if i < n and s[i] == "/":
		i += 1
	if i >= n or not _is_alpha(s[i]):
		return {}
	while i < n:
		if s[i] == ">":
			return {"kind": "unknown", "length": i + 1 - start}
		if s[i] == "\n" or s[i] == "\r":
			return {}
		i += 1
	return {}


static func _is_alpha(c: String) -> bool:
	return (c >= "a" and c <= "z") or (c >= "A" and c <= "Z")


# Unity uses #RRGGBBAA; Godot uses #RRGGBB. Strip the alpha from 9-char hex.
static func _convert_color_value(color: String) -> String:
	color = color.strip_edges()
	if color.begins_with("#") and color.length() == 9:
		return color.substr(0, 7)
	return color


## Parse a color string (named or hex) into a Color, mirroring the Rust
## `parse_color` named-color table. Returns `fallback` when unrecognized.
static func parse_color(value: String, fallback: Color) -> Color:
	var c := value.strip_edges().to_lower()
	match c:
		"red":
			return Color(1, 0, 0)
		"green":
			return Color(0, 0.5, 0)
		"blue":
			return Color(0, 0, 1)
		"white":
			return Color(1, 1, 1)
		"black":
			return Color(0, 0, 0)
		"yellow":
			return Color(1, 1, 0)
		"cyan", "aqua":
			return Color(0, 1, 1)
		"magenta", "fuchsia":
			return Color(1, 0, 1)
		"gray", "grey":
			return Color(0.5, 0.5, 0.5)
		"orange":
			return Color(1, 0.65, 0)
		"purple":
			return Color(0.5, 0, 0.5)
		"pink":
			return Color(1, 0.75, 0.8)
		"brown":
			return Color(0.65, 0.16, 0.16)
		"lime":
			return Color(0, 1, 0)
		"navy":
			return Color(0, 0, 0.5)
		"teal":
			return Color(0, 0.5, 0.5)
		"olive":
			return Color(0.5, 0.5, 0)
		"maroon":
			return Color(0.5, 0, 0)
		"silver":
			return Color(0.75, 0.75, 0.75)
	if c.begins_with("#") and Color.html_is_valid(c):
		return Color.html(c)
	return fallback
