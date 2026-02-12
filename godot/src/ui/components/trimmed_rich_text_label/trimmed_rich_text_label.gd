class_name TrimmedRichTextLabel
extends RichTextLabel

## RichTextLabel that limits text to [param max_lines] lines and adjusts [member custom_minimum_size]
## based on content. Assign the text with [method set_text_trimmed] to apply trimming and height.

const _ELLIPSIS := "…"

@export var max_lines: int = 2


func set_text_trimmed(p_text: String) -> void:
	text = p_text
	call_deferred("_apply_trim_and_size", p_text)


func _apply_trim_and_size(p_text: String) -> void:
	var font := get_theme_font("normal_font")
	var font_size := get_theme_font_size("normal_font_size")
	var line_height := font.get_height(font_size)
	var max_width := size.x

	if max_width <= 0:
		return

	var full_size := font.get_multiline_string_size(
		p_text, HORIZONTAL_ALIGNMENT_LEFT, max_width, font_size
	)

	var one_line_h := line_height * 1.2
	var max_lines_h := line_height * (0.2 + max_lines)

	if full_size.y <= one_line_h:
		custom_minimum_size.y = one_line_h
		return

	if full_size.y <= max_lines_h:
		custom_minimum_size.y = max_lines_h
		return

	# More than max_lines → trim and set fixed height
	text = _trim_to_max_lines(p_text)
	custom_minimum_size.y = max_lines_h


func _trim_to_max_lines(p_text: String) -> String:
	var font := get_theme_font("normal_font")
	var font_size := get_theme_font_size("normal_font_size")
	var max_width := size.x
	var max_height := font.get_height(font_size) * (0.2 + max_lines)

	var best := ""
	var current := ""

	for i in p_text.length():
		current += p_text[i]
		var measured := font.get_multiline_string_size(
			current + _ELLIPSIS, HORIZONTAL_ALIGNMENT_LEFT, max_width, font_size
		)
		if measured.y > max_height:
			break
		best = current

	return best.rstrip(" ") + _ELLIPSIS
