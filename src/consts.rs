pub const GODOT_PROJECT_FOLDER: &str = "./godot/";
pub const GODOT_SENTRY_ADDON_FOLDER: &str = "./godot/addons/sentry";
pub const BIN_FOLDER: &str = "./.bin/";
pub const RUST_LIB_PROJECT_FOLDER: &str = "./lib/";
pub const EXPORTS_FOLDER: &str = "./exports/";

pub const SENTRY_ADDON_URL: &str = "https://github.com/getsentry/sentry-godot/releases/download/1.6.0/sentry-godot-1.6.0+4e3e3e5.zip";

pub const PROTOC_BASE_URL: &str =
    "https://github.com/protocolbuffers/protobuf/releases/download/v23.2/protoc-23.2-";

pub const GODOT_ENGINE_RELEASES_BASE_URL: &str = "https://godot-engine-releases.dclexplorer.com/";

pub const GODOT_CURRENT_VERSION: &str = "4.6.2";

/// Short commit SHA (first 9 chars) of the godotengine fork this stable build was compiled from.
/// Must match the `gh.<sha>` segment printed by `godot4_bin --version`
/// (e.g. `4.6.2.stable.gh.2c8983653 - Protocol Squad`) and the SHA-tagged release path published
/// by the godot-engine-releases pipeline. Bump it in lockstep with a new fork publish: it busts the
/// local download cache (keys embed it) and pins the immutable per-SHA release URLs below.
pub const GODOT_BUILD_SHA: &str = "2c8983653";

/// Release tag identifying a specific fork build — `<version>.stable.gh.<sha>`, mirroring the
/// `--version` string. Single source for the release URL path segment, the on-disk template SHA
/// marker, and the installed-binary version validation.
pub fn godot_release_tag() -> String {
    format!("{GODOT_CURRENT_VERSION}.stable.gh.{GODOT_BUILD_SHA}")
}

/// Editor (binary) base URL for the pinned stable fork build, e.g.
/// `https://godot-engine-releases.dclexplorer.com/4.6.2.stable.gh.2c8983653/editors/`.
pub fn godot_editor_base_url() -> String {
    format!(
        "{GODOT_ENGINE_RELEASES_BASE_URL}{}/editors/",
        godot_release_tag()
    )
}

/// Compressed-templates base URL for the pinned stable fork build, e.g.
/// `https://godot-engine-releases.dclexplorer.com/4.6.2.stable.gh.2c8983653/compressed-templates/`.
pub fn godot_templates_base_url() -> String {
    format!(
        "{GODOT_ENGINE_RELEASES_BASE_URL}{}/compressed-templates/",
        godot_release_tag()
    )
}

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
