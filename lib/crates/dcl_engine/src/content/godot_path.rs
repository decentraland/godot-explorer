use std::sync::OnceLock;

use godot::classes::ProjectSettings;
use godot::prelude::*;

/// The core's data dir as written in our paths, when it is Godot's `user://` folder.
fn user_dir_prefix() -> Option<&'static str> {
    static PREFIX: OnceLock<Option<String>> = OnceLock::new();
    PREFIX
        .get_or_init(|| {
            let user = ProjectSettings::singleton()
                .globalize_path("user://")
                .to_string();
            let data = dcl_core::Core::get().data_dir().to_path_buf();
            let same = std::fs::canonicalize(&user).ok()? == std::fs::canonicalize(&data).ok()?;
            same.then(|| data.display().to_string().trim_end_matches('/').to_string())
        })
        .as_deref()
}

/// The path to hand Godot for a file under the data dir. On Android Godot opens absolute paths
/// through JNI, which crashes on the content threads (GODOT-EXPLORER-3G); `user://` does not.
pub fn godot_path(path: &str) -> GString {
    if let Some(rest) = user_dir_prefix().and_then(|prefix| path.strip_prefix(prefix)) {
        if rest.is_empty() || rest.starts_with('/') {
            return GString::from(format!("user://{}", rest.trim_start_matches('/')).as_str());
        }
    }
    GString::from(path)
}
