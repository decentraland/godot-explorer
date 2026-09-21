use deno_core::{anyhow::anyhow, error::AnyError, op2, OpDecl};

pub fn ops() -> Vec<OpDecl> {
    vec![op_utf8_encode(), op_utf8_decode()]
}

#[op2]
#[buffer]
fn op_utf8_encode(#[string] text: String) -> Vec<u8> {
    text.into_bytes()
}

#[op2]
#[string]
fn op_utf8_decode(
    #[buffer] bytes: &[u8],
    fatal: bool,
    ignore_bom: bool,
) -> Result<String, AnyError> {
    decode_utf8(bytes, fatal, ignore_bom)
}

fn decode_utf8(bytes: &[u8], fatal: bool, ignore_bom: bool) -> Result<String, AnyError> {
    let bytes = if ignore_bom {
        bytes
    } else {
        bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes)
    };
    if fatal {
        std::str::from_utf8(bytes)
            .map(str::to_owned)
            .map_err(|err| anyhow!("invalid utf-8 sequence: {err}"))
    } else {
        Ok(String::from_utf8_lossy(bytes).into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::decode_utf8;

    #[test]
    fn strips_bom_unless_ignored() {
        assert_eq!(decode_utf8(b"\xEF\xBB\xBFhi", false, false).unwrap(), "hi");
        assert_eq!(
            decode_utf8(b"\xEF\xBB\xBFhi", false, true).unwrap(),
            "\u{FEFF}hi"
        );
        assert_eq!(decode_utf8(b"\xEF\xBB", false, false).unwrap(), "\u{FFFD}");
    }

    #[test]
    fn lossy_replaces_invalid_sequences_with_maximal_subparts() {
        assert_eq!(
            decode_utf8(b"a\xF0\x90\x28\xBCb", false, false).unwrap(),
            "a\u{FFFD}(\u{FFFD}b"
        );
        assert_eq!(decode_utf8(b"\xC3\xA9", false, false).unwrap(), "é");
    }

    #[test]
    fn fatal_rejects_invalid_utf8() {
        assert!(decode_utf8(b"\xFF", true, false).is_err());
        assert!(decode_utf8(b"\xEF\xBB\xBF\xFF", true, false).is_err());
        assert_eq!(decode_utf8(b"ok", true, false).unwrap(), "ok");
    }
}
