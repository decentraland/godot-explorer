pub const GODOT_PROJECT_FOLDER: &str = "./godot/";
pub const BIN_FOLDER: &str = "./.bin/";
pub const RUST_LIB_PROJECT_FOLDER: &str = "./lib/";
pub const EXPORTS_FOLDER: &str = "./exports/";

pub const PROTOC_BASE_URL: &str =
    "https://github.com/protocolbuffers/protobuf/releases/download/v23.2/protoc-23.2-";

pub const GODOT4_BIN_BASE_URL: &str =
    "https://github.com/godotengine/godot/releases/download/4.3-stable/Godot_v4.3-stable_";

pub const GODOT_CURRENT_VERSION: &str = "4.3";

pub const GODOT4_EXPORT_TEMPLATES_BASE_URL: &str =
    "https://github.com/decentraland/godotengine/releases/download/4.3-stable/";

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
