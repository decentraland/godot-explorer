//! Reading commands off the inspector socket. Two robustness concerns:
//!
//! 1. A packet can hold several JSON commands concatenated (the relay coalesces frames from the
//!    consumer to the device), so a packet is split into balanced top-level `{...}` objects.
//! 2. Godot's `WebSocketPeer` can hand over a byte-corrupted frame: a brace duplicated at an
//!    object boundary (`...{"depth":1}}},"id":...`), which closes the object early and drops the
//!    trailing `id`, so the acknowledgement is never sent and the caller times out. A leftover
//!    tail that starts with `,` marks it; the tail is spliced back on after dropping the spurious
//!    closing brace and validated again. A repaired frame dispatches normally, silently: the real
//!    fix belongs in `WebSocketPeer`, this keeps the debug channel usable meanwhile.

use serde_json::{Map, Value};

/// A well-formed `SCENE_INSPECTOR_CMD`.
#[derive(Debug, PartialEq)]
pub struct Command {
    pub cmd: String,
    pub args: Map<String, Value>,
    pub request_id: String,
}

impl Command {
    /// `None` unless `frame` is a command with a type and a non-empty id.
    fn from_frame(frame: &Value) -> Option<Self> {
        let fields = frame.as_object()?;
        if fields.get("type")?.as_str()? != "SCENE_INSPECTOR_CMD" {
            return None;
        }
        let request_id = match fields.get("id")? {
            Value::String(id) => id.clone(),
            other => other.to_string(),
        };
        if request_id.is_empty() {
            return None;
        }
        Some(Self {
            cmd: fields
                .get("cmd")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            args: fields
                .get("args")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default(),
            request_id,
        })
    }
}

/// The commands a packet holds, repairing the known corruption.
pub fn parse_packet(text: &str) -> Vec<Command> {
    // A single, uncorrupted frame parses as it is.
    if let Some(command) = serde_json::from_str::<Value>(text)
        .ok()
        .and_then(|whole| Command::from_frame(&whole))
    {
        return vec![command];
    }
    extract_frames(text)
        .iter()
        .filter_map(Command::from_frame)
        .collect()
}

fn extract_frames(text: &str) -> Vec<Value> {
    let mut out = Vec::new();
    let mut cursor = 0;
    while cursor < text.len() {
        let Some((start, end)) = next_balanced_object(text, cursor) else {
            break;
        };
        let object = &text[start..=end];
        let tail_start = end + 1;
        // A tail that resumes the same object (a `,` before the next top-level `{`) is the
        // corruption signature.
        if text[tail_start..].trim_start().starts_with(',') {
            let repaired = format!("{}{}", &object[..object.len() - 1], &text[tail_start..]);
            if let Ok(value @ Value::Object(_)) = serde_json::from_str::<Value>(&repaired) {
                out.push(value);
                return out;
            }
        }
        if let Ok(value @ Value::Object(_)) = serde_json::from_str::<Value>(object) {
            out.push(value);
        }
        cursor = tail_start;
    }
    out
}

/// The next brace-balanced `{...}` at or after `from`, as byte offsets of its braces. String
/// literals and escapes are respected so braces in a payload do not miscount.
fn next_balanced_object(text: &str, from: usize) -> Option<(usize, usize)> {
    let mut depth = 0usize;
    let mut start = None;
    let (mut in_string, mut escaped) = (false, false);
    for (offset, c) in text[from..].char_indices() {
        let index = from + offset;
        if in_string {
            match (escaped, c) {
                (true, _) => escaped = false,
                (false, '\\') => escaped = true,
                (false, '"') => in_string = false,
                _ => {}
            }
            continue;
        }
        match c {
            '"' => in_string = true,
            '{' => {
                if depth == 0 {
                    start = Some(index);
                }
                depth += 1;
            }
            '}' if depth > 0 => {
                depth -= 1;
                if depth == 0 {
                    return start.map(|s| (s, index));
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const PING: &str = r#"{"type":"SCENE_INSPECTOR_CMD","id":"a","cmd":"ping","args":{}}"#;

    #[test]
    fn a_single_frame_is_a_command() {
        let commands = parse_packet(PING);
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].cmd, "ping");
        assert_eq!(commands[0].request_id, "a");
    }

    #[test]
    fn coalesced_frames_are_split() {
        let packet = format!("{PING}{}", PING.replace("\"a\"", "\"b\""));
        let ids: Vec<_> = parse_packet(&packet)
            .into_iter()
            .map(|c| c.request_id)
            .collect();
        assert_eq!(ids, ["a", "b"]);
    }

    #[test]
    fn a_duplicated_boundary_brace_is_repaired() {
        let corrupted = r#"{"type":"SCENE_INSPECTOR_CMD","cmd":"scene","args":{"filters":{"depth":1}}},"id":"c"}"#;
        let commands = parse_packet(corrupted);
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].request_id, "c");
        assert_eq!(commands[0].args["filters"]["depth"], 1);
    }

    #[test]
    fn braces_inside_strings_do_not_count() {
        let packet =
            r#"{"type":"SCENE_INSPECTOR_CMD","id":"d","cmd":"eval","args":{"code":"} { \" }"}}"#;
        let commands = parse_packet(packet);
        assert_eq!(commands[0].args["code"], "} { \" }");
    }

    #[test]
    fn frames_without_an_id_or_type_are_ignored() {
        assert!(parse_packet(r#"{"type":"SCENE_INSPECTOR_CMD","cmd":"ping"}"#).is_empty());
        assert!(parse_packet(r#"{"type":"OTHER","id":"x"}"#).is_empty());
        assert!(parse_packet("garbage").is_empty());
    }
}
