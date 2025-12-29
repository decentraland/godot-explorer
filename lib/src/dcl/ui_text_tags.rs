#[derive(Debug, Clone, PartialEq)]
pub enum ConversionResult {
    NonModified,
    Modified(String),
}

/// Result of stripping Unity tags from text, optionally extracting the first color
#[derive(Debug, Clone, PartialEq)]
pub struct StripTagsResult {
    /// The text with all tags removed
    pub text: String,
    /// The first color found in the text (if any), in Godot-compatible format
    pub first_color: Option<String>,
}

pub fn convert_unity_to_godot(input_text: &str) -> ConversionResult {
    let bytes = input_text.as_bytes();
    let mut result = Vec::with_capacity(input_text.len());
    let mut modified = false;
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'<' {
            // Attempt to parse a tag
            if let Some((tag_type, tag_len)) = parse_tag(&bytes[i..]) {
                modified = true;
                match tag_type {
                    Tag::BoldOpen => result.extend_from_slice(b"[b]"),
                    Tag::BoldClose => result.extend_from_slice(b"[/b]"),
                    Tag::ItalicOpen => result.extend_from_slice(b"[i]"),
                    Tag::ItalicClose => result.extend_from_slice(b"[/i]"),
                    Tag::ColorOpen(color) => {
                        result.extend_from_slice(b"[color=");
                        result.extend_from_slice(color.as_bytes());
                        result.push(b']');
                    }
                    Tag::ColorClose => result.extend_from_slice(b"[/color]"),
                }
                i += tag_len;
                continue;
            }
        }

        result.push(bytes[i]);
        i += 1;
    }

    if modified {
        // The result should be valid UTF-8 because:
        // 1. Input is valid UTF-8 (&str)
        // 2. We only insert complete valid UTF-8 byte sequences
        // 3. Individual bytes are copied unchanged from valid UTF-8 input
        // However, we use unwrap_or_else to handle any edge cases gracefully
        ConversionResult::Modified(String::from_utf8(result).unwrap_or_else(|e| {
            // Fallback: use lossy conversion if UTF-8 validation fails
            String::from_utf8_lossy(&e.into_bytes()).into_owned()
        }))
    } else {
        ConversionResult::NonModified
    }
}

/// Strips all Unity tags from text and extracts the first color found.
/// Used for TextShape (3D labels) which cannot render rich text.
///
/// Returns the plain text with all tags removed, and optionally the first color
/// found which can be applied to the entire label.
/// Also strips unknown tags (like `<cspace>`, `<size>`, etc.).
pub fn strip_tags_extract_first_color(input_text: &str) -> Option<StripTagsResult> {
    let bytes = input_text.as_bytes();
    let mut result = Vec::with_capacity(input_text.len());
    let mut first_color: Option<String> = None;
    let mut found_any_tag = false;
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'<' {
            // Attempt to parse a known tag first
            if let Some((tag_type, tag_len)) = parse_tag(&bytes[i..]) {
                found_any_tag = true;
                // Extract first color if not already found
                if first_color.is_none() {
                    if let Tag::ColorOpen(color) = tag_type {
                        first_color = Some(color.to_string());
                    }
                }
                // Skip the tag entirely (don't add anything to result)
                i += tag_len;
                continue;
            }

            // Try to skip unknown tags (anything that looks like <...>)
            if let Some(tag_len) = skip_unknown_tag(&bytes[i..]) {
                found_any_tag = true;
                i += tag_len;
                continue;
            }
        }

        result.push(bytes[i]);
        i += 1;
    }

    if !found_any_tag {
        return None;
    }

    let text = String::from_utf8(result)
        .unwrap_or_else(|e| String::from_utf8_lossy(&e.into_bytes()).into_owned());

    Some(StripTagsResult { text, first_color })
}

/// Attempts to skip an unknown tag pattern like <tagname>, </tagname>, or <tagname=value>
/// Returns the length of the tag if found, None otherwise
fn skip_unknown_tag(bytes: &[u8]) -> Option<usize> {
    if bytes.len() < 3 || bytes[0] != b'<' {
        return None;
    }

    let mut i = 1;

    // Skip optional '/' for closing tags
    if i < bytes.len() && bytes[i] == b'/' {
        i += 1;
    }

    // Must start with a letter (tag name)
    if i >= bytes.len() || !bytes[i].is_ascii_alphabetic() {
        return None;
    }

    // Find the closing '>'
    while i < bytes.len() {
        if bytes[i] == b'>' {
            return Some(i + 1);
        }
        // Don't allow newlines inside tags
        if bytes[i] == b'\n' || bytes[i] == b'\r' {
            return None;
        }
        i += 1;
    }

    None
}

#[derive(Debug)]
enum Tag<'a> {
    BoldOpen,
    BoldClose,
    ItalicOpen,
    ItalicClose,
    ColorOpen(&'a str),
    ColorClose,
}

/// Parses a Unity HTML tag and returns its type and total length in bytes
fn parse_tag(bytes: &[u8]) -> Option<(Tag<'_>, usize)> {
    if bytes.len() < 3 || bytes[0] != b'<' {
        return None;
    }

    // Closing tag (starts with '</')
    if bytes[1] == b'/' {
        if bytes.len() >= 4 {
            // </b>
            if bytes[2] == b'b' && bytes[3] == b'>' {
                return Some((Tag::BoldClose, 4));
            }
            // </i>
            if bytes[2] == b'i' && bytes[3] == b'>' {
                return Some((Tag::ItalicClose, 4));
            }
        }
        // </color>
        if bytes.len() >= 8 && &bytes[2..8] == b"color>" {
            return Some((Tag::ColorClose, 8));
        }
        return None;
    }

    // Opening tags
    match bytes[1] {
        b'b' if bytes.get(2) == Some(&b'>') => Some((Tag::BoldOpen, 3)),
        b'i' if bytes.get(2) == Some(&b'>') => Some((Tag::ItalicOpen, 3)),
        b'c' => {
            // <color=...> o <color = ...>
            parse_color_tag(bytes)
        }
        _ => None,
    }
}

/// Parses a color tag in various formats: <color=value>, <color="value">, or <color = value>
fn parse_color_tag(bytes: &[u8]) -> Option<(Tag<'_>, usize)> {
    if bytes.len() < 9 || &bytes[0..6] != b"<color" {
        return None;
    }

    let mut i = 6;

    // Skip whitespace before '='
    while i < bytes.len() && bytes[i] == b' ' {
        i += 1;
    }

    // Must have an '=' sign
    if i >= bytes.len() || bytes[i] != b'=' {
        return None;
    }
    i += 1;

    // Skip whitespace after '='
    while i < bytes.len() && bytes[i] == b' ' {
        i += 1;
    }

    // Check if the value is quoted
    let has_quotes = i < bytes.len() && bytes[i] == b'"';
    if has_quotes {
        i += 1;
    }

    let value_start = i;

    // Find the end of the value
    let end_char = if has_quotes { b'"' } else { b'>' };
    while i < bytes.len() && bytes[i] != end_char {
        i += 1;
    }

    if i >= bytes.len() {
        return None;
    }

    let value_end = i;
    i += 1; // Skip the end_char

    // If we had quotes, we still need to find the closing '>'
    if has_quotes {
        while i < bytes.len() && bytes[i] == b' ' {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] != b'>' {
            return None;
        }
        i += 1;
    }

    // Extract and convert the color value
    let color_value = std::str::from_utf8(&bytes[value_start..value_end]).ok()?;
    let converted_color = convert_color_value(color_value);

    Some((Tag::ColorOpen(converted_color), i))
}

/// Converts Unity color values to Godot format
///
/// Unity uses #RRGGBBAA format (8 hex digits + #), while Godot uses #RRGGBB (6 hex digits + #).
/// This function strips the alpha channel from 8-digit hex colors.
fn convert_color_value(color: &str) -> &str {
    let color = color.trim();

    // If it's an 8-digit hex color (#RRGGBBAA), strip the alpha channel
    // We can return a slice because it's borrowed from the input with the same lifetime
    if color.starts_with('#') && color.len() == 9 {
        // Return only #RRGGBB (first 7 characters), discarding AA (alpha)
        &color[..7]
    } else {
        // Return the color as-is (named colors or already-valid hex colors)
        color
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_modification() {
        let text = "Simple text without tags";
        assert_eq!(convert_unity_to_godot(text), ConversionResult::NonModified);
    }

    #[test]
    fn test_bold_tag() {
        let text = "We are <b>not</b> amused.";
        let expected = "We are [b]not[/b] amused.";
        assert_eq!(
            convert_unity_to_godot(text),
            ConversionResult::Modified(expected.to_string())
        );
    }

    #[test]
    fn test_italic_tag() {
        let text = "We are <i>usually</i> not amused.";
        let expected = "We are [i]usually[/i] not amused.";
        assert_eq!(
            convert_unity_to_godot(text),
            ConversionResult::Modified(expected.to_string())
        );
    }

    #[test]
    fn test_color_tag_name() {
        let text = "<color=cyan>some text</color>";
        let expected = "[color=cyan]some text[/color]";
        assert_eq!(
            convert_unity_to_godot(text),
            ConversionResult::Modified(expected.to_string())
        );
    }

    #[test]
    fn test_color_tag_hex() {
        let text = "We are <color=#ff0000>colorfully</color> amused";
        let expected = "We are [color=#ff0000]colorfully[/color] amused";
        assert_eq!(
            convert_unity_to_godot(text),
            ConversionResult::Modified(expected.to_string())
        );
    }

    #[test]
    fn test_color_tag_hex_with_alpha() {
        let text = "We are <color=#ff0000ff>colorfully</color> amused";
        let expected = "We are [color=#ff0000]colorfully[/color] amused";
        assert_eq!(
            convert_unity_to_godot(text),
            ConversionResult::Modified(expected.to_string())
        );
    }

    #[test]
    fn test_color_with_quotes() {
        let text = r#"We are <color="green">green</color> with envy"#;
        let expected = "We are [color=green]green[/color] with envy";
        assert_eq!(
            convert_unity_to_godot(text),
            ConversionResult::Modified(expected.to_string())
        );
    }

    #[test]
    fn test_color_with_spaces() {
        let text = "We are <color = red>red</color> with anger";
        let expected = "We are [color=red]red[/color] with anger";
        assert_eq!(
            convert_unity_to_godot(text),
            ConversionResult::Modified(expected.to_string())
        );
    }

    #[test]
    fn test_nested_tags() {
        let text = "We are <b><i>definitely not</i></b> amused";
        let expected = "We are [b][i]definitely not[/i][/b] amused";
        assert_eq!(
            convert_unity_to_godot(text),
            ConversionResult::Modified(expected.to_string())
        );
    }

    #[test]
    fn test_complex_example() {
        let text =
            "We are <b>absolutely <i>definitely</i> not</b> amused and <color=green>green</color>";
        let expected =
            "We are [b]absolutely [i]definitely[/i] not[/b] amused and [color=green]green[/color]";
        assert_eq!(
            convert_unity_to_godot(text),
            ConversionResult::Modified(expected.to_string())
        );
    }

    #[test]
    fn test_multiple_colors() {
        let text = "<color=red>Red</color> and <color=#00ff00>Green</color>";
        let expected = "[color=red]Red[/color] and [color=#00ff00]Green[/color]";
        assert_eq!(
            convert_unity_to_godot(text),
            ConversionResult::Modified(expected.to_string())
        );
    }

    #[test]
    fn test_incomplete_tags_ignored() {
        let text = "This <b is not a tag and neither is <";
        assert_eq!(convert_unity_to_godot(text), ConversionResult::NonModified);
    }

    // Tests for strip_tags_extract_first_color

    #[test]
    fn test_strip_no_tags() {
        let text = "Simple text without tags";
        assert_eq!(strip_tags_extract_first_color(text), None);
    }

    #[test]
    fn test_strip_bold_tags() {
        let text = "We are <b>not</b> amused.";
        let result = strip_tags_extract_first_color(text).unwrap();
        assert_eq!(result.text, "We are not amused.");
        assert_eq!(result.first_color, None);
    }

    #[test]
    fn test_strip_color_extracts_first() {
        let text = "<color=red>Hello</color><color=yellow> World!</color>";
        let result = strip_tags_extract_first_color(text).unwrap();
        assert_eq!(result.text, "Hello World!");
        assert_eq!(result.first_color, Some("red".to_string()));
    }

    #[test]
    fn test_strip_color_hex() {
        let text = "<color=#ff0000>Red text</color>";
        let result = strip_tags_extract_first_color(text).unwrap();
        assert_eq!(result.text, "Red text");
        assert_eq!(result.first_color, Some("#ff0000".to_string()));
    }

    #[test]
    fn test_strip_color_hex_with_alpha() {
        let text = "<color=#ff0000ff>Red text</color>";
        let result = strip_tags_extract_first_color(text).unwrap();
        assert_eq!(result.text, "Red text");
        // Alpha is stripped by convert_color_value
        assert_eq!(result.first_color, Some("#ff0000".to_string()));
    }

    #[test]
    fn test_strip_mixed_tags() {
        let text = "<b><color=cyan>Bold cyan</color></b> and <i>italic</i>";
        let result = strip_tags_extract_first_color(text).unwrap();
        assert_eq!(result.text, "Bold cyan and italic");
        assert_eq!(result.first_color, Some("cyan".to_string()));
    }

    #[test]
    fn test_strip_only_first_color_extracted() {
        let text = "<color=red>R</color><color=green>G</color><color=blue>B</color>";
        let result = strip_tags_extract_first_color(text).unwrap();
        assert_eq!(result.text, "RGB");
        assert_eq!(result.first_color, Some("red".to_string()));
    }

    #[test]
    fn test_strip_unknown_tags() {
        let text = "<cspace=-0.1em>Creature Cards Treasure Chest</cspace>";
        let result = strip_tags_extract_first_color(text).unwrap();
        assert_eq!(result.text, "Creature Cards Treasure Chest");
        assert_eq!(result.first_color, None);
    }

    #[test]
    fn test_strip_unknown_tags_with_color() {
        let text = "<cspace=-0.1em><color=red>Red Text</color></cspace>";
        let result = strip_tags_extract_first_color(text).unwrap();
        assert_eq!(result.text, "Red Text");
        assert_eq!(result.first_color, Some("red".to_string()));
    }

    #[test]
    fn test_strip_size_tag() {
        let text = "<size=20>Big text</size> normal";
        let result = strip_tags_extract_first_color(text).unwrap();
        assert_eq!(result.text, "Big text normal");
        assert_eq!(result.first_color, None);
    }
}
