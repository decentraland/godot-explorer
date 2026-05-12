pub const GODOT_PROJECT_FOLDER: &str = "./godot/";
pub const GODOT_SENTRY_ADDON_FOLDER: &str = "./godot/addons/sentry";
pub const BIN_FOLDER: &str = "./.bin/";
pub const RUST_LIB_PROJECT_FOLDER: &str = "./lib/";
pub const EXPORTS_FOLDER: &str = "./exports/";

pub const SENTRY_ADDON_URL: &str = "https://github.com/getsentry/sentry-godot/releases/download/1.6.0/sentry-godot-1.6.0+4e3e3e5.zip";

pub const PROTOC_BASE_URL: &str =
    "https://github.com/protocolbuffers/protobuf/releases/download/v23.2/protoc-23.2-";

pub const GODOT_ENGINE_RELEASES_BASE_URL: &str = "https://godot-engine-releases.dclexplorer.com/";

pub const GODOT4_BIN_BASE_URL: &str =
    "https://godot-engine-releases.dclexplorer.com/4.6.2.stable/editors/";

pub const GODOT_CURRENT_VERSION: &str = "4.6.2";

pub const GODOT4_EXPORT_TEMPLATES_BASE_URL: &str =
    "https://godot-engine-releases.dclexplorer.com/4.6.2.stable/compressed-templates/";

/// Sanitizes a git branch name for use in the release artifact URL path.
/// Slashes (e.g. `fix/foo`) are replaced with dashes (`fix-foo`) to match how
/// branch builds are published under `/branches/<sanitized>/`.
pub fn sanitize_branch_for_url(branch: &str) -> String {
    branch.replace('/', "-")
}

/// Returns the editor base URL for a given branch build, e.g.
/// `https://godot-engine-releases.dclexplorer.com/branches/<branch>/editors/`
pub fn godot_editor_base_url_for_branch(branch: &str) -> String {
    let slug = sanitize_branch_for_url(branch);
    format!("{GODOT_ENGINE_RELEASES_BASE_URL}branches/{slug}/editors/")
}

/// Returns the compressed-templates base URL for a given branch build, e.g.
/// `https://godot-engine-releases.dclexplorer.com/branches/<branch>/compressed-templates/`
pub fn godot_templates_base_url_for_branch(branch: &str) -> String {
    let slug = sanitize_branch_for_url(branch);
    format!("{GODOT_ENGINE_RELEASES_BASE_URL}branches/{slug}/compressed-templates/")
}

pub const GODOT_PLATFORM_FILES: &[(&str, &[&str])] = &[
    ("ios", &["ios.zip"]),
    (
        "android",
        &[
            "android_debug.apk",
            "android_release.apk",
            "android_source.zip",
        ],
    ),
    ("linux", &["linux_debug.x86_64", "linux_release.x86_64"]),
    ("macos", &["macos.zip"]),
    (
        "windows",
        &["windows_debug_x86_64.exe", "windows_release_x86_64.exe"],
    ),
];

// iOS export name (matches export.rs)
pub const IOS_EXPORT_NAME: &str = "decentraland-godot-client";

// Android SDK/NDK constants - these are repeated 6+ times in the code
pub const ANDROID_NDK_VERSION: &str = "28.1.13356709";
// TODO: Use these constants to replace hardcoded values throughout the codebase
#[allow(dead_code)]
pub const ANDROID_SDK_BUILD_TOOLS_VERSION: &str = "35.0.0";
#[allow(dead_code)]
pub const ANDROID_PLATFORM_VERSION: &str = "android-35";
