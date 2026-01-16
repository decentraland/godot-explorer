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
            "CARGO_PROFILE_RELEASE_BUILD_OVERRIDE_DEBUG".to_string(),
            "true".to_string(),
        );

        // TODO: maybe the 35 should be api_version? Hardcoded for now, without 35 the cxx doesn't compile (pthread dep issue)
        let cxxflags = "-v --target=aarch64-linux-android35";

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

/// Check if a tool is installed (only in .bin folder)
pub fn is_tool_installed(tool: &str) -> bool {
    match tool {
        "protoc" => BinPaths::protoc_bin().exists(),
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

    pub fn android_deps() -> PathBuf {
        PathBuf::from(BIN_FOLDER).join("android_deps")
    }

    pub fn android_deps_zip() -> PathBuf {
        PathBuf::from(BIN_FOLDER).join("android_dependencies.zip")
    }

    pub fn keystore(filename: &str) -> PathBuf {
        PathBuf::from(BIN_FOLDER).join(filename)
    }
}

/// Common Android SDK path patterns
// TODO: Use in platform.rs and run.rs for Android SDK detection
#[allow(dead_code)]
pub fn get_default_android_sdk_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "~".to_string());
    PathBuf::from(home).join("Android/Sdk")
}

/// Get the path to the host platform's built library
pub fn get_host_library_path() -> PathBuf {
    let target = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    };

    let lib_prefix = if cfg!(target_os = "windows") {
        ""
    } else {
        "lib"
    };
    let lib_ext = get_lib_extension(target);
    let file_name = format!("{}dclgodot{}", lib_prefix, lib_ext);

    let output_folder = match target {
        "windows" => "libdclgodot_windows",
        "linux" => "libdclgodot_linux",
        "macos" => "libdclgodot_macos",
        _ => "libdclgodot_linux",
    };

    PathBuf::from(RUST_LIB_PROJECT_FOLDER)
        .join("target")
        .join(output_folder)
        .join(file_name)
}
