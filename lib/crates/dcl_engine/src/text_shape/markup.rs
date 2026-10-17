//! Unity TextMeshPro markup for TextShape, shared by every renderer so they agree on tag
//! semantics. Supports `<b>`, `<i>`, `<color=...>` (named, `#RGB`, `#RRGGBB`, `#RRGGBBAA`, quoted
//! or spaced) and `<size=...>` with TMP units (bare = font units times the TMP-to-Godot factor,
//! `N px`, `N em`, `N %`, and a leading +/- for relative), all resolved to pixels since BBCode
//! only takes px. Unknown tags (`<cspace>`, ...) are stripped.

use godot::builtin::Color;

use super::layout::unity_to_godot_font_size;

#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    Text(String),
    BOpen,
    BClose,
    IOpen,
    IClose,
    ColorOpen(String),
    ColorClose,
    SizeOpen(String),
    SizeClose,
    /// An unrecognized `<...>` tag, dropped by the tokenizer.
    Unknown,
}

/// A parsed tag and how many chars it spans.
struct Tag {
    token: Token,
    length: usize,
}

/// Splits the markup into text and tags. Unknown `<...>` tags are dropped.
pub fn tokenize(s: &str) -> Vec<Token> {
    let chars: Vec<char> = s.chars().collect();
    let n = chars.len();
    let mut tokens = Vec::new();
    let mut i = 0;
    let mut text_start = 0;
    while i < n {
        if chars[i] != '<' {
            i += 1;
            continue;
        }
        let Some(tag) = parse_tag(&chars, i) else {
            i += 1;
            continue;
        };
        // Pending text, then the tag.
        if i > text_start {
            tokens.push(Token::Text(chars[text_start..i].iter().collect()));
        }
        if tag.token != Token::Unknown {
            tokens.push(tag.token);
        }
        i += tag.length;
        text_start = i;
    }
    if text_start < n {
        tokens.push(Token::Text(chars[text_start..n].iter().collect()));
    }
    tokens
}

fn starts_with(chars: &[char], start: usize, needle: &str) -> bool {
    needle
        .chars()
        .enumerate()
        .all(|(offset, c)| chars.get(start + offset) == Some(&c))
}

/// One tag starting at `start` (where `chars[start]` is `<`); None when the text is no tag at all.
fn parse_tag(chars: &[char], start: usize) -> Option<Tag> {
    let n = chars.len();
    if start + 2 >= n || chars[start] != '<' {
        return None;
    }
    if chars[start + 1] == '/' {
        for (needle, token) in [
            ("</b>", Token::BClose),
            ("</i>", Token::IClose),
            ("</color>", Token::ColorClose),
            ("</size>", Token::SizeClose),
        ] {
            if starts_with(chars, start, needle) {
                return Some(Tag {
                    length: needle.len(),
                    token,
                });
            }
        }
        return skip_unknown(chars, start);
    }
    if starts_with(chars, start, "<b>") {
        return Some(Tag {
            token: Token::BOpen,
            length: 3,
        });
    }
    if starts_with(chars, start, "<i>") {
        return Some(Tag {
            token: Token::IOpen,
            length: 3,
        });
    }
    if starts_with(chars, start, "<color") {
        return parse_kv_tag(chars, start, "color");
    }
    if starts_with(chars, start, "<size") {
        return parse_kv_tag(chars, start, "size");
    }
    skip_unknown(chars, start)
}

/// `<name=value>`, `<name = value>` or `<name="value">`.
fn parse_kv_tag(chars: &[char], start: usize, name: &str) -> Option<Tag> {
    let n = chars.len();
    let mut i = start + 1 + name.len();
    while i < n && chars[i] == ' ' {
        i += 1;
    }
    if i >= n || chars[i] != '=' {
        return skip_unknown(chars, start);
    }
    i += 1;
    while i < n && chars[i] == ' ' {
        i += 1;
    }
    let has_quotes = i < n && chars[i] == '"';
    if has_quotes {
        i += 1;
    }
    let value_start = i;
    let end_char = if has_quotes { '"' } else { '>' };
    while i < n && chars[i] != end_char {
        i += 1;
    }
    if i >= n {
        return skip_unknown(chars, start);
    }
    let value: String = chars[value_start..i].iter().collect();
    i += 1;
    if has_quotes {
        while i < n && chars[i] == ' ' {
            i += 1;
        }
        if i >= n || chars[i] != '>' {
            return skip_unknown(chars, start);
        }
        i += 1;
    }
    let token = if name == "color" {
        Token::ColorOpen(convert_color_value(&value))
    } else {
        Token::SizeOpen(value.trim().to_string())
    };
    Some(Tag {
        token,
        length: i - start,
    })
}

/// An unrecognized `<...>` tag, or None when the `<` does not begin a tag-like sequence.
fn skip_unknown(chars: &[char], start: usize) -> Option<Tag> {
    let n = chars.len();
    let mut i = start + 1;
    if i < n && chars[i] == '/' {
        i += 1;
    }
    if i >= n || !chars[i].is_ascii_alphabetic() {
        return None;
    }
    while i < n {
        if chars[i] == '>' {
            return Some(Tag {
                token: Token::Unknown,
                length: i + 1 - start,
            });
        }
        if chars[i] == '\n' || chars[i] == '\r' {
            return None;
        }
        i += 1;
    }
    None
}

/// Unity uses `#RRGGBBAA`; Godot's BBCode takes `#RRGGBB`: the alpha of a 9-char hex goes.
fn convert_color_value(color: &str) -> String {
    let color = color.trim();
    if color.starts_with('#') && color.chars().count() == 9 {
        color.chars().take(7).collect()
    } else {
        color.to_string()
    }
}

/// BBCode for a Label3D or RichTextLabel. `base_size` (the font size in px) turns percentage
/// `<size=N%>` tags into the absolute px BBCode understands.
pub fn to_bbcode(s: &str, base_size: i32) -> String {
    let mut out = String::new();
    for token in tokenize(s) {
        match token {
            Token::Text(text) => out.push_str(&text),
            Token::BOpen => out.push_str("[b]"),
            Token::BClose => out.push_str("[/b]"),
            Token::IOpen => out.push_str("[i]"),
            Token::IClose => out.push_str("[/i]"),
            Token::ColorOpen(value) => out.push_str(&format!("[color={value}]")),
            Token::ColorClose => out.push_str("[/color]"),
            Token::SizeOpen(value) => out.push_str(&format!(
                "[font_size={}]",
                resolve_size_px(&value, base_size)
            )),
            Token::SizeClose => out.push_str("[/font_size]"),
            Token::Unknown => {}
        }
    }
    out
}

/// A `<size=...>` value in absolute pixels, as TMP reads it: an optional sign, a number and an
/// optional unit.
///
///  - bare number or `N px`: font-size units, scaled by the TMP-to-Godot factor;
///  - `N em`: multiples of `base_size`;
///  - `N %`: a fraction of `base_size`, the sign ignored (`<size=+50%>` is 50%, smaller).
///
/// For bare, px and em a leading sign is relative (`base_size` plus or minus); none is absolute.
/// Unparseable values fall back to `base_size`; the result is at least 1 px.
pub fn resolve_size_px(value: &str, base_size: i32) -> i32 {
    let mut v = value.trim().to_lowercase();
    if v.is_empty() {
        return base_size;
    }
    let mut sign = 0;
    if let Some(rest) = v.strip_prefix('+') {
        sign = 1;
        v = rest.to_string();
    } else if let Some(rest) = v.strip_prefix('-') {
        sign = -1;
        v = rest.to_string();
    }
    enum Unit {
        FontUnits,
        Px,
        Em,
        Percent,
    }
    let mut unit = Unit::FontUnits;
    if let Some(rest) = v.strip_suffix("px") {
        unit = Unit::Px;
        v = rest.to_string();
    } else if let Some(rest) = v.strip_suffix("em") {
        unit = Unit::Em;
        v = rest.to_string();
    } else if let Some(rest) = v.strip_suffix('%') {
        unit = Unit::Percent;
        v = rest.to_string();
    }
    let v = v.trim();
    let Some(num) = v.parse::<f32>().ok().filter(|n| n.is_finite()) else {
        return base_size;
    };
    let base = base_size as f32;
    if matches!(unit, Unit::Percent) {
        return ((num / 100.0 * base).round() as i32).max(1);
    }
    // px and bare numbers are both font-size units in TMP, so both take the font factor.
    let contribution = if matches!(unit, Unit::Em) {
        num * base
    } else {
        unity_to_godot_font_size(num, None)
    };
    let result = match sign {
        1 => base + contribution,
        -1 => base - contribution,
        _ => contribution,
    };
    (result.round() as i32).max(1)
}

/// The text with every tag stripped, and the first `<color=...>` found.
pub fn strip_to_plain(s: &str) -> (String, Option<Color>) {
    let mut text = String::new();
    let mut color = None;
    for token in tokenize(s) {
        match token {
            Token::Text(t) => text.push_str(&t),
            Token::ColorOpen(value) if color.is_none() => {
                color = Some(parse_color(&value, Color::WHITE));
            }
            _ => {}
        }
    }
    (text, color)
}

/// A named or hex color; `fallback` when unrecognized.
pub fn parse_color(value: &str, fallback: Color) -> Color {
    let c = value.trim().to_lowercase();
    let named = match c.as_str() {
        "red" => Some((1.0, 0.0, 0.0)),
        "green" => Some((0.0, 0.5, 0.0)),
        "blue" => Some((0.0, 0.0, 1.0)),
        "white" => Some((1.0, 1.0, 1.0)),
        "black" => Some((0.0, 0.0, 0.0)),
        "yellow" => Some((1.0, 1.0, 0.0)),
        "cyan" | "aqua" => Some((0.0, 1.0, 1.0)),
        "magenta" | "fuchsia" => Some((1.0, 0.0, 1.0)),
        "gray" | "grey" => Some((0.5, 0.5, 0.5)),
        "orange" => Some((1.0, 0.65, 0.0)),
        "purple" => Some((0.5, 0.0, 0.5)),
        "pink" => Some((1.0, 0.75, 0.8)),
        "brown" => Some((0.65, 0.16, 0.16)),
        "lime" => Some((0.0, 1.0, 0.0)),
        "navy" => Some((0.0, 0.0, 0.5)),
        "teal" => Some((0.0, 0.5, 0.5)),
        "olive" => Some((0.5, 0.5, 0.0)),
        "maroon" => Some((0.5, 0.0, 0.0)),
        "silver" => Some((0.75, 0.75, 0.75)),
        _ => None,
    };
    if let Some((r, g, b)) = named {
        return Color::from_rgb(r, g, b);
    }
    if c.starts_with('#') {
        if let Some(color) = Color::from_html(c.as_str()) {
            return color;
        }
    }
    fallback
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(s: &str) -> Vec<Token> {
        tokenize(s)
    }

    #[test]
    fn plain_text_is_one_token() {
        assert_eq!(kinds("hello"), vec![Token::Text("hello".into())]);
        assert!(kinds("").is_empty());
    }

    #[test]
    fn bold_italic_and_color_tokenize() {
        assert_eq!(
            kinds("a<b>b</b><i>i</i><color=red>r</color>"),
            vec![
                Token::Text("a".into()),
                Token::BOpen,
                Token::Text("b".into()),
                Token::BClose,
                Token::IOpen,
                Token::Text("i".into()),
                Token::IClose,
                Token::ColorOpen("red".into()),
                Token::Text("r".into()),
                Token::ColorClose,
            ]
        );
    }

    #[test]
    fn key_value_tags_accept_spaces_and_quotes() {
        assert_eq!(
            kinds("<color = red>x"),
            vec![Token::ColorOpen("red".into()), Token::Text("x".into())]
        );
        assert_eq!(
            kinds("<color=\"#ff0000\" >x"),
            vec![Token::ColorOpen("#ff0000".into()), Token::Text("x".into())]
        );
        assert_eq!(
            kinds("<size=20>x"),
            vec![Token::SizeOpen("20".into()), Token::Text("x".into())]
        );
    }

    #[test]
    fn unknown_tags_are_dropped_and_lone_angle_brackets_kept() {
        assert_eq!(
            kinds("a<cspace=2>b"),
            vec![Token::Text("a".into()), Token::Text("b".into())]
        );
        assert_eq!(strip_to_plain("1 < 2").0, "1 < 2");
        assert_eq!(strip_to_plain("a<br>b").0, "ab");
        assert_eq!(strip_to_plain("<unterminated").0, "<unterminated");
    }

    #[test]
    fn alpha_is_stripped_from_nine_char_hex() {
        assert_eq!(
            kinds("<color=#11223344>x")[0],
            Token::ColorOpen("#112233".into())
        );
        assert_eq!(
            kinds("<color=#112233>x")[0],
            Token::ColorOpen("#112233".into())
        );
    }

    #[test]
    fn bbcode_mirrors_the_tags() {
        assert_eq!(
            to_bbcode("<b>x</b> <color=red>y</color>", 20),
            "[b]x[/b] [color=red]y[/color]"
        );
    }

    #[test]
    fn size_units() {
        // Percent of the base, sign ignored.
        assert_eq!(resolve_size_px("50%", 40), 20);
        assert_eq!(resolve_size_px("+50%", 40), 20);
        // Em multiplies the base; a sign makes it relative.
        assert_eq!(resolve_size_px("2em", 10), 20);
        assert_eq!(resolve_size_px("+1em", 10), 20);
        assert_eq!(resolve_size_px("-1em", 10), 1);
        // Bare numbers go through the font factor (19.514 per TMP unit by default).
        assert_eq!(resolve_size_px("10", 40), 195);
        assert_eq!(resolve_size_px("10px", 40), 195);
        assert_eq!(resolve_size_px("+10", 40), 235);
        // Garbage falls back to the base; the result is at least a pixel.
        assert_eq!(resolve_size_px("big", 33), 33);
        assert_eq!(resolve_size_px("", 33), 33);
        assert_eq!(resolve_size_px("0%", 33), 1);
    }

    #[test]
    fn size_tag_becomes_font_size_bbcode() {
        assert_eq!(
            to_bbcode("<size=50%>x</size>", 40),
            "[font_size=20]x[/font_size]"
        );
    }

    #[test]
    fn first_color_is_extracted() {
        let (text, color) = strip_to_plain("<color=blue>a</color><color=red>b</color>");
        assert_eq!(text, "ab");
        assert_eq!(color, Some(Color::from_rgb(0.0, 0.0, 1.0)));
        assert_eq!(strip_to_plain("plain").1, None);
    }

    #[test]
    fn named_colors_and_fallback() {
        assert_eq!(
            parse_color("Red", Color::WHITE),
            Color::from_rgb(1.0, 0.0, 0.0)
        );
        assert_eq!(
            parse_color("grey", Color::WHITE),
            Color::from_rgb(0.5, 0.5, 0.5)
        );
        assert_eq!(parse_color("nonsense", Color::BLACK), Color::BLACK);
    }
}
