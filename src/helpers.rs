use crate::consts::*;
use anyhow::Result;
use std::fs;
use std::path::PathBuf;

// TODO: Use these helper functions to reduce code duplication throughout the codebase

/// Helper function to canonicalize a path with better error context
#[allow(dead_code)]
pub fn canonicalize_with_context(path: &str, context: &str) -> Result<PathBuf> {
    fs::canonicalize(path)
        .map_err(|e| anyhow::anyhow!("Failed to canonicalize {} ({}): {}", context, path, e))
}

/// Get the library file extension for the current or specified platform
pub fn get_lib_extension(target: &str) -> &'static str {
    match target {
        "windows" | "win64" => ".dll",
        "linux" => ".so",
        "macos" => ".dylib",
        "android" => ".so",
        "ios" => ".a",
        _ => ".so", // default to Linux
    }
}

/// Get the executable extension for the current or specified platform
pub fn get_exe_extension(target: &str) -> &'static str {
    match target {
        "windows" | "win64" => ".exe",
        _ => "",
    }
}

/// Helper to construct Android NDK path
pub fn get_android_ndk_path(sdk_root: &str) -> PathBuf {
    PathBuf::from(sdk_root)
        .join("ndk")
        .join(crate::consts::ANDROID_NDK_VERSION)
}

/// Check if a command exists and is executable
// TODO: Replace repeated command checking patterns with this function
#[allow(dead_code)]
pub fn command_exists(cmd: &str) -> bool {
    std::process::Command::new(cmd)
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Android build environment configuration
pub struct AndroidBuildEnv {
    pub ndk_path: String,
    pub target_cc: String,
    pub target_cxx: String,
    pub target_ar: String,
    pub cargo_target_linker: String,
}

impl AndroidBuildEnv {
    pub fn new(ndk_path: String) -> Self {
        // Determine the host OS directory name for NDK prebuilt toolchains
        let host_tag = if cfg!(windows) {
            "windows-x86_64"
        } else if cfg!(target_os = "macos") {
            "darwin-x86_64"
        } else {
            "linux-x86_64"
        };

        let toolchain_base = format!("{}/toolchains/llvm/prebuilt/{}/bin", ndk_path, host_tag);

        // On Windows, the executables have .cmd extension
        let clang_suffix = if cfg!(windows) { ".cmd" } else { "" };
        let ar_suffix = if cfg!(windows) { ".exe" } else { "" };

        Self {
            target_cc: format!(
                "{}/aarch64-linux-android{}-clang{}",
                toolchain_base,
                crate::consts::ANDROID_PLATFORM_VERSION.replace("android-", ""),
                clang_suffix
            ),
            target_cxx: format!(
                "{}/aarch64-linux-android{}-clang++{}",
                toolchain_base,
                crate::consts::ANDROID_PLATFORM_VERSION.replace("android-", ""),
                clang_suffix
            ),
            target_ar: format!("{}/llvm-ar{}", toolchain_base, ar_suffix),
            cargo_target_linker: format!(
                "{}/aarch64-linux-android{}-clang{}",
                toolchain_base,
                crate::consts::ANDROID_PLATFORM_VERSION.replace("android-", ""),
                clang_suffix
            ),
            ndk_path,
        }
    }

    pub fn apply_to_env(&self, env: &mut std::collections::HashMap<String, String>) {
        env.insert("TARGET_CC".to_string(), self.target_cc.clone());
        env.insert("TARGET_CXX".to_string(), self.target_cxx.clone());
        env.insert("TARGET_AR".to_string(), self.target_ar.clone());
        env.insert(
            "CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER".to_string(),
            self.cargo_target_linker.clone(),
        );
        env.insert(
            "CARGO_FFMPEG_SYS_DISABLE_SIZE_T_IS_USIZE".to_string(),
            "1".to_string(),
        );
        env.insert(
            "CARGO_PROFILE_RELEASE_BUILD_OVERRIDE_DEBUG".to_string(),
            "true".to_string(),
        );

        let cxxflags = "-v --target=aarch64-linux-android";

        // Use the same host tag for lib path
        let host_tag = if cfg!(windows) {
            "windows-x86_64"
        } else if cfg!(target_os = "macos") {
            "darwin-x86_64"
        } else {
            "linux-x86_64"
        };

        let rustflags = format!(
            "-L{}/toolchains/llvm/prebuilt/{}/lib/aarch64-unknown-linux-musl",
            self.ndk_path, host_tag
        );

        env.insert("CXXFLAGS".to_string(), cxxflags.to_string());
        env.insert("RUSTFLAGS".to_string(), rustflags);
    }
}

/// Construct FFmpeg download URL for a given platform
// TODO: Refactor install_dependency.rs to use this function instead of hardcoded URLs
#[allow(dead_code)]
pub fn get_ffmpeg_url(platform: &str) -> String {
    let arch = match platform {
        "linux" => "linux64",
        "windows" | "win64" => "win64",
        "macos" => "macos64",
        _ => "linux64",
    };

    let extension = match platform {
        "windows" | "win64" => "zip",
        _ => "tar.xz",
    };

    format!(
        "{}/ffmpeg-{}-{}-{}.{}",
        FFMPEG_BASE_URL, FFMPEG_VERSION_TAG, arch, FFMPEG_BUILD_TYPE, extension
    )
}

/// Extract filename from FFmpeg URL
#[allow(dead_code)]
pub fn get_ffmpeg_filename_from_url(url: &str) -> Option<String> {
    url.split('/').last().map(|s| s.to_string())
}

/// Get the extracted folder name from FFmpeg archive
// TODO: Use in install_dependency.rs to avoid hardcoding folder names
#[allow(dead_code)]
pub fn get_ffmpeg_extracted_folder(platform: &str) -> String {
    let arch = match platform {
        "linux" => "linux64",
        "windows" | "win64" => "win64",
        "macos" => "macos64",
        _ => "linux64",
    };

    format!(
        "ffmpeg-{}-{}-{}",
        FFMPEG_VERSION_TAG, arch, FFMPEG_BUILD_TYPE
    )
}

/// Check if a tool is installed (only in .bin folder)
pub fn is_tool_installed(tool: &str) -> bool {
    match tool {
        "protoc" => BinPaths::protoc_bin().exists(),
        "ffmpeg" => BinPaths::ffmpeg_bin().exists(),
        "godot" | "godot4_bin" => BinPaths::godot_bin().exists(),
        _ => which::which(tool).is_ok(),
    }
}

/// Get the path to a tool (only from local .bin folder)
pub fn get_tool_path(tool: &str) -> Option<PathBuf> {
    match tool {
        "protoc" => {
            let local = BinPaths::protoc_bin();
            if local.exists() {
                Some(local)
            } else {
                None
            }
        }
        "ffmpeg" => {
            let local = BinPaths::ffmpeg_bin();
            if local.exists() {
                Some(local)
            } else {
                None
            }
        }
        "godot" | "godot4_bin" => {
            let local = BinPaths::godot_bin();
            if local.exists() {
                Some(local)
            } else {
                None
            }
        }
        _ => which::which(tool).ok(),
    }
}

/// Common path constructors for bin folder
pub struct BinPaths;

impl BinPaths {
    pub fn godot() -> PathBuf {
        PathBuf::from(BIN_FOLDER).join("godot")
    }

    pub fn godot_bin() -> PathBuf {
        Self::godot().join("godot4_bin")
    }

    pub fn protoc() -> PathBuf {
        PathBuf::from(BIN_FOLDER).join("protoc")
    }

    pub fn protoc_bin() -> PathBuf {
        let protoc_name = if cfg!(windows) {
            "protoc.exe"
        } else {
            "protoc"
        };
        Self::protoc().join("bin").join(protoc_name)
    }

    pub fn ffmpeg() -> PathBuf {
        PathBuf::from(BIN_FOLDER).join("ffmpeg")
    }

    pub fn ffmpeg_bin() -> PathBuf {
        let ffmpeg_name = if cfg!(windows) {
            "ffmpeg.exe"
        } else {
            "ffmpeg"
        };
        Self::ffmpeg().join("bin").join(ffmpeg_name)
    }

    pub fn android_deps() -> PathBuf {
        PathBuf::from(BIN_FOLDER).join("android_deps")
    }

    pub fn android_deps_zip() -> PathBuf {
        PathBuf::from(BIN_FOLDER).join("android_dependencies.zip")
    }

    pub fn keystore(filename: &str) -> PathBuf {
        PathBuf::from(BIN_FOLDER).join(filename)
    }

    pub fn temp_dir(name: &str) -> PathBuf {
        PathBuf::from(BIN_FOLDER).join(name)
    }
}

/// Common Android SDK path patterns
// TODO: Use in platform.rs and run.rs for Android SDK detection
#[allow(dead_code)]
pub fn get_default_android_sdk_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "~".to_string());
    PathBuf::from(home).join("Android/Sdk")
}
