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

/// Join a glTF dependency URI onto its base directory, collapsing `.`, `..` and
/// repeated separators, so `assets/models` + `../tex/t.jpg` becomes
/// `assets/tex/t.jpg` (the key the content mapping actually holds).
/// Returns `None` for non-relative URIs (`scheme://`, `/rooted`) and for paths
/// that would climb above `base_dir`; both are rejected rather than resolved.
pub fn resolve_dependency_path(base_dir: &str, dep: &str) -> Option<String> {
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
