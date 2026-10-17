//! `{name}` placeholders in translated text, filled from a dictionary (the engine's own
//! `String.format`, without going through a by-name call).

use godot::prelude::*;

/// `text` with every `{key}` replaced by the dictionary's value for `key` (as text); a
/// placeholder without a value stays as written.
pub fn fill_placeholders(text: &str, replacements: &VarDictionary) -> String {
    fill(
        text,
        replacements
            .iter_shared()
            .map(|(key, value)| (key.stringify().to_string(), value.stringify().to_string())),
    )
}

fn fill(text: &str, pairs: impl IntoIterator<Item = (String, String)>) -> String {
    pairs
        .into_iter()
        .fold(text.to_string(), |out, (key, value)| {
            out.replace(&format!("{{{key}}}"), &value)
        })
}

#[cfg(test)]
mod tests {
    use super::fill;

    #[test]
    fn placeholders_take_their_values() {
        let pairs = [("name".into(), "Ana".into()), ("count".into(), "3".into())];
        assert_eq!(
            fill("{name} has {count} {unknown}", pairs),
            "Ana has 3 {unknown}"
        );
    }
}
