pub const GODOT_PROJECT_FOLDER: &str = "./godot/";
pub const GODOT_SENTRY_ADDON_FOLDER: &str = "./godot/addons/sentry";
pub const BIN_FOLDER: &str = "./.bin/";
pub const RUST_LIB_PROJECT_FOLDER: &str = "./lib/";
pub const EXPORTS_FOLDER: &str = "./exports/";

pub const SENTRY_ADDON_URL: &str = "https://github.com/getsentry/sentry-godot/releases/download/1.0.0/sentry-godot-gdextension-1.0.0+f672aa4.zip";

pub const PROTOC_BASE_URL: &str =
    "https://github.com/protocolbuffers/protobuf/releases/download/v23.2/protoc-23.2-";

pub const GODOT4_BIN_BASE_URL: &str =
    "https://github.com/decentraland/godotengine/releases/download/4.5.1-stable/";

pub const GODOT_CURRENT_VERSION: &str = "4.5.1";

pub const GODOT4_EXPORT_TEMPLATES_BASE_URL: &str =
    "https://github.com/decentraland/godotengine/releases/download/4.5.1-stable/";

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

// Android SDK/NDK constants - these are repeated 6+ times in the code
pub const ANDROID_NDK_VERSION: &str = "27.1.12297006";
// TODO: Use these constants to replace hardcoded values throughout the codebase
#[allow(dead_code)]
pub const ANDROID_SDK_BUILD_TOOLS_VERSION: &str = "35.0.0";
#[allow(dead_code)]
pub const ANDROID_PLATFORM_VERSION: &str = "android-35";
