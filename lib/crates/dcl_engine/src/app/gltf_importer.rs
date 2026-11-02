//! Rewrites a glTF's external references before Godot imports it: with a placeholder it swaps
//! every image and buffer for a stand-in and records them as dependencies; otherwise it maps the
//! URIs through the scene's content mapping.

use godot::classes::{GltfDocumentExtension, GltfState, IGltfDocumentExtension};
use godot::global::Error;
use godot::prelude::*;

const PLACEHOLDER_IMAGE: &str = "decentraland_logo.png";
const PLACEHOLDER_BUFFER: &str = "empty_buffer.bin";

#[derive(GodotClass)]
#[class(tool, base=GltfDocumentExtension, init)]
pub struct CustomGltfImporter {
    base: Base<GltfDocumentExtension>,
}

/// An external URI is any non-empty one that is not an inline `data:` blob.
pub fn is_external(uri: &str) -> bool {
    !uri.is_empty() && !uri.starts_with("data:")
}

/// The `uri` of each entry of `json[key]`, as the dictionaries Godot parsed.
fn entries(json: &VarDictionary, key: &str) -> Vec<VarDictionary> {
    json.get(key)
        .and_then(|v| v.try_to::<VarArray>().ok())
        .map(|a| a.iter_shared().filter_map(|v| v.try_to().ok()).collect())
        .unwrap_or_default()
}

fn uri_of(entry: &VarDictionary) -> String {
    entry
        .get("uri")
        .and_then(|v| v.try_to::<GString>().ok())
        .map(|s| s.to_string())
        .unwrap_or_default()
}

#[godot_api]
impl IGltfDocumentExtension for CustomGltfImporter {
    fn import_preflight(
        &mut self,
        state: Option<Gd<GltfState>>,
        _extensions: PackedStringArray,
    ) -> Error {
        let Some(mut state) = state else {
            return Error::ERR_INVALID_PARAMETER;
        };
        let json = state.get_json();
        let placeholder = state.get_additional_data("placeholder_image");
        if !placeholder.is_nil() && placeholder.booleanize() {
            let mut dependencies = PackedStringArray::new();
            for (key, stand_in) in [
                ("images", PLACEHOLDER_IMAGE),
                ("buffers", PLACEHOLDER_BUFFER),
            ] {
                for mut entry in entries(&json, key) {
                    let uri = uri_of(&entry);
                    if is_external(&uri) {
                        dependencies.push(uri.as_str());
                        entry.set("uri", stand_in);
                    }
                }
            }
            state.set_additional_data("dependencies", &dependencies.to_variant());
        } else if !state.get_additional_data("base_path").is_nil() {
            let mappings = state
                .get_additional_data("mappings")
                .try_to::<VarDictionary>()
                .unwrap_or_default();
            for key in ["images", "buffers"] {
                for mut entry in entries(&json, key) {
                    let uri = uri_of(&entry);
                    if is_external(&uri) {
                        let mapped = mappings
                            .get(uri.as_str())
                            .unwrap_or_else(|| uri.to_variant());
                        entry.set("uri", &mapped);
                    }
                }
            }
        }
        Error::OK
    }
}

#[cfg(test)]
mod tests {
    use super::is_external;

    #[test]
    fn inline_and_empty_uris_are_not_external() {
        assert!(is_external("texture.png"));
        assert!(!is_external(""));
        assert!(!is_external("data:image/png;base64,AAAA"));
    }
}
