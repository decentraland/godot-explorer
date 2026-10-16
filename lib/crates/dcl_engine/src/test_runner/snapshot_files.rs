//! Where the snapshot tests keep their reference images and the images of the current run.

use godot::classes::{DirAccess, Os, ProjectSettings};
use godot::prelude::*;

#[derive(Clone, Debug)]
pub struct SnapshotFolders {
    /// The committed references.
    pub reference: String,
    /// What this run produced, next to the references.
    pub comparison: String,
}

impl SnapshotFolders {
    /// `from_cli` is `--snapshot-folder`; without it the repository's folder in the editor and the
    /// user data folder in an export. Both folders are created.
    pub fn resolve(from_cli: &str, editor_default: &str, user_default: &str) -> Self {
        let base = if !from_cli.is_empty() {
            from_cli.to_string()
        } else if Os::singleton().has_feature("editor") {
            ProjectSettings::singleton()
                .globalize_path(editor_default)
                .to_string()
        } else {
            format!("{}{user_default}", Os::singleton().get_user_data_dir())
        };
        let folders = Self::under(&base);
        for folder in [&folders.reference, &folders.comparison] {
            if !DirAccess::dir_exists_absolute(folder.as_str()) {
                DirAccess::make_dir_recursive_absolute(folder.as_str());
            }
        }
        folders
    }

    fn under(base: &str) -> Self {
        let reference = if base.ends_with('/') {
            base.to_string()
        } else {
            format!("{base}/")
        };
        Self {
            comparison: format!("{reference}comparison/"),
            reference,
        }
    }
}

/// A scene's snapshot name as a file name: spaces and separators become underscores, lower case.
pub fn snapshot_file_name(name: &str) -> String {
    let mut file = name.replace([' ', '/', '\\'], "_").to_lowercase();
    if !file.ends_with(".png") {
        file.push_str(".png");
    }
    file
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folders_end_with_a_separator_and_nest_the_comparison() {
        let folders = SnapshotFolders::under("/tmp/shots");
        assert_eq!(folders.reference, "/tmp/shots/");
        assert_eq!(folders.comparison, "/tmp/shots/comparison/");
        assert_eq!(
            SnapshotFolders::under("/tmp/shots/").reference,
            "/tmp/shots/"
        );
    }

    #[test]
    fn snapshot_names_become_safe_png_files() {
        assert_eq!(snapshot_file_name("Avatar Attach/1"), "avatar_attach_1.png");
        assert_eq!(snapshot_file_name("a\\B.png"), "a_b.png");
    }
}
