pub fn get_base_dir(file_path: &str) -> String {
    let last_slash = file_path.rfind('/');
    if let Some(last_slash) = last_slash {
        return file_path[0..last_slash].to_string();
    }
    "".to_string()
}

pub fn get_extension(file_path: &str) -> String {
    let last_dot = file_path.rfind('.');
    if let Some(last_dot) = last_dot {
        return file_path[last_dot..].to_string();
    }
    "".to_string()
}

/// Decode RFC 3986 percent escapes; `+` stays literal (glTF percent-encodes, it
/// does not form-encode). `None` on a malformed escape or invalid UTF-8, so the
/// caller keeps the URI as written instead of failing the whole model.
fn percent_decode(uri: &str) -> Option<String> {
    if !uri.contains('%') {
        return Some(uri.to_string());
    }
    let bytes = uri.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let high = hex_digit(*bytes.get(i + 1)?)?;
            let low = hex_digit(*bytes.get(i + 2)?)?;
            out.push((high << 4) | low);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// Join a glTF dependency URI onto its base directory, collapsing `.`, `..` and
/// repeated separators, so `assets/models` + `../tex/t.jpg` becomes
/// `assets/tex/t.jpg` (the key the content mapping actually holds).
/// Returns `None` for non-relative URIs (`scheme://`, `/rooted`) and for paths
/// that would climb above `base_dir`; both are rejected rather than resolved.
pub fn resolve_dependency_path(base_dir: &str, dep: &str) -> Option<String> {
    // The mapping is keyed by the file name as it exists in the bundle
    // (`base color.png`), not by the percent-encoded URI (`base%20color.png`). Only
    // `dep` is decoded; `%2F` is a literal `/` per RFC 3986 and the pass below runs
    // after it, so `..%2F` normalizes like `../`.
    let decoded = percent_decode(dep);
    let dep = decoded.as_deref().unwrap_or(dep);
    if dep.starts_with('/') || dep.starts_with('\\') || dep.contains("://") {
        return None;
    }
    let mut segments: Vec<&str> = Vec::new();
    for segment in base_dir.split(['/', '\\']).chain(dep.split(['/', '\\'])) {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop()?;
            }
            other => segments.push(other),
        }
    }
    Some(segments.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::content_mapping::ContentMappingAndUrl;
    use crate::dcl::common::content_entity::TypedIpfsRef;

    #[test]
    fn resolves_parent_relative_dependency() {
        let base_dir = get_base_dir("assets/models/a.glb");
        assert_eq!(
            resolve_dependency_path(&base_dir, "../optimized-textures/t.jpg").as_deref(),
            Some("assets/optimized-textures/t.jpg")
        );
    }

    #[test]
    fn keeps_already_normalized_dependencies() {
        let base_dir = get_base_dir("assets/models/a.glb");
        for dep in ["t.jpg", "./t.jpg", "sub/../t.jpg"] {
            assert_eq!(
                resolve_dependency_path(&base_dir, dep).as_deref(),
                Some("assets/models/t.jpg"),
                "dep={dep}"
            );
        }
        // Two levels up lands on a valid scene-root key, not outside the scene.
        assert_eq!(
            resolve_dependency_path(&base_dir, "../../t.jpg").as_deref(),
            Some("t.jpg")
        );
        assert_eq!(
            resolve_dependency_path(&base_dir, "sub//t.jpg").as_deref(),
            Some("assets/models/sub/t.jpg")
        );
        assert_eq!(
            resolve_dependency_path(&base_dir, "sub\\t.jpg").as_deref(),
            Some("assets/models/sub/t.jpg")
        );
        // A GLB at the scene root has an empty base directory.
        assert_eq!(
            resolve_dependency_path("", "t.jpg").as_deref(),
            Some("t.jpg")
        );
    }

    #[test]
    fn rejects_non_relative_and_escaping_dependencies() {
        let base_dir = get_base_dir("assets/models/a.glb");
        for dep in [
            "https://cdn.example/t.jpg",
            "ipfs://bafk/t.jpg",
            "ar://bafk/t.jpg",
            "/assets/t.jpg",
            "../../../t.jpg",
            "sub/../../../../t.jpg",
        ] {
            assert_eq!(resolve_dependency_path(&base_dir, dep), None, "dep={dep}");
        }
        // One segment too many for a single-segment base.
        assert_eq!(resolve_dependency_path("models", "../../t.jpg"), None);
    }

    #[test]
    fn decodes_percent_encoded_dependencies() {
        let base_dir = get_base_dir("assets/models/a.glb");
        for (dep, expected) in [
            ("base%20color.png", "assets/models/base color.png"),
            ("base%23color.png", "assets/models/base#color.png"),
            ("a%2Bb.png", "assets/models/a+b.png"),
            ("100%25.png", "assets/models/100%.png"),
            ("caf%C3%A9.png", "assets/models/café.png"),
            ("sub%2Fb.png", "assets/models/sub/b.png"),
        ] {
            assert_eq!(
                resolve_dependency_path(&base_dir, dep).as_deref(),
                Some(expected),
                "dep={dep}"
            );
        }
        // `+` is literal in glTF URIs, it never stands for a space.
        assert_eq!(
            resolve_dependency_path(&base_dir, "base+color.png").as_deref(),
            Some("assets/models/base+color.png")
        );
    }

    #[test]
    fn keeps_malformed_percent_escapes_intact() {
        let base_dir = get_base_dir("assets/models/a.glb");
        for dep in ["100%.png", "%ZZ.png", "%2.png", "caf%C3.png", "%FF%FE.png"] {
            let expected = format!("assets/models/{dep}");
            assert_eq!(
                resolve_dependency_path(&base_dir, dep).as_deref(),
                Some(expected.as_str()),
                "dep={dep}"
            );
        }
    }

    #[test]
    fn normalizes_after_decoding() {
        let base_dir = get_base_dir("assets/models/a.glb");
        assert_eq!(
            resolve_dependency_path(&base_dir, "%2E%2E/optimized-textures/t.jpg").as_deref(),
            Some("assets/optimized-textures/t.jpg")
        );
        assert_eq!(
            resolve_dependency_path(&base_dir, "..%2Foptimized-textures%2Ft.jpg").as_deref(),
            Some("assets/optimized-textures/t.jpg")
        );
        // Escapes that only become a rooted or absolute URI after decoding are
        // still rejected, because the checks run on the decoded string.
        for dep in ["%2Fassets%2Ft.jpg", "https%3A%2F%2Fcdn.example%2Ft.jpg"] {
            assert_eq!(resolve_dependency_path(&base_dir, dep), None, "dep={dep}");
        }
        assert_eq!(
            resolve_dependency_path(&base_dir, "%2E%2E%2F..%2F..%2Ft.jpg"),
            None
        );
    }

    #[test]
    fn parent_relative_dependency_matches_the_content_mapping() {
        // Creator Hub Optimize writes `../optimized-textures/t.jpg` into the GLB
        // while the content mapping only holds the scene-relative key.
        let mapping = ContentMappingAndUrl::from_base_url_and_content(
            "http://localhost:8001/content/contents/".into(),
            vec![TypedIpfsRef {
                file: "assets/Optimized-Textures/T.JPG".into(),
                hash: "bafkreitexture".into(),
            }],
        );
        let base_dir = get_base_dir("assets/models/a.glb");
        let joined = format!("{}/{}", base_dir, "../optimized-textures/t.jpg");
        assert_eq!(mapping.get_hash(&joined), None);

        let resolved = resolve_dependency_path(&base_dir, "../optimized-textures/t.jpg")
            .expect("parent-relative dependency resolves");
        // `get_hash` lowercases both sides, so the resolved path matches.
        assert_eq!(
            mapping.get_hash(&resolved).map(String::as_str),
            Some("bafkreitexture")
        );
    }
}
