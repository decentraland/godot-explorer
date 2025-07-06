pub const GODOT_PROJECT_FOLDER: &str = "./godot/";
pub const BIN_FOLDER: &str = "./.bin/";
pub const RUST_LIB_PROJECT_FOLDER: &str = "./lib/";
pub const EXPORTS_FOLDER: &str = "./exports/";

pub const PROTOC_BASE_URL: &str =
    "https://github.com/protocolbuffers/protobuf/releases/download/v23.2/protoc-23.2-";

pub const GODOT4_BIN_BASE_URL: &str =
    "https://github.com/decentraland/godotengine/releases/download/4.4.1-stable/";

pub const GODOT_CURRENT_VERSION: &str = "4.4.1";

pub const GODOT4_EXPORT_TEMPLATES_BASE_URL: &str =
    "https://github.com/decentraland/godotengine/releases/download/4.4.1-stable/";

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

// FFmpeg constants
// TODO: Refactor FFmpeg URL construction to use these constants
#[allow(dead_code)]
pub const FFMPEG_BASE_URL: &str = "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest";
#[allow(dead_code)]
pub const FFMPEG_VERSION_TAG: &str = "n6.1-latest";
#[allow(dead_code)]
pub const FFMPEG_BUILD_TYPE: &str = "lgpl-shared-6.1";

