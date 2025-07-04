use std::path::Path;

use crate::{
    consts::{BIN_FOLDER, RUST_LIB_PROJECT_FOLDER},
    platform::get_platform_info,
    ui::{print_message, print_section, MessageType},
};

#[derive(Debug, Clone)]
pub struct BuildStatus {
    pub host_built: bool,
    pub android_built: bool,
    pub ios_built: bool,
    pub windows_built: bool,
    pub macos_built: bool,
}

impl BuildStatus {
    pub fn check() -> Self {
        let lib_target_path = format!("{}/target", RUST_LIB_PROJECT_FOLDER);
        
        Self {
            host_built: check_host_build(&lib_target_path),
            android_built: check_android_build(&lib_target_path),
            ios_built: check_ios_build(&lib_target_path),
            windows_built: check_windows_build(&lib_target_path),
            macos_built: check_macos_build(&lib_target_path),
        }
    }
    
    pub fn is_built_for_target(&self, target: &str) -> bool {
        match target {
            "android" | "quest" => self.android_built,
            "ios" => self.ios_built,
            "windows" | "win64" => self.windows_built,
            "macos" => self.macos_built,
            "linux" => self.host_built && get_platform_info().os == "linux",
            _ => false,
        }
    }
}

fn check_host_build(lib_target_path: &str) -> bool {
    let platform = get_platform_info();
    match platform.os.as_str() {
        "linux" => {
            let lib_path = format!("{}/libdclgodot_linux/libdclgodot.so", lib_target_path);
            Path::new(&lib_path).exists()
        }
        "windows" => {
            let lib_path = format!("{}/libdclgodot_windows/dclgodot.dll", lib_target_path);
            Path::new(&lib_path).exists()
        }
        "macos" => {
            let lib_path = format!("{}/libdclgodot_macos/libdclgodot.dylib", lib_target_path);
            Path::new(&lib_path).exists()
        }
        _ => false,
    }
}

fn check_android_build(lib_target_path: &str) -> bool {
    let lib_path = format!("{}/libdclgodot_android/libdclgodot.so", lib_target_path);
    Path::new(&lib_path).exists()
}

fn check_ios_build(lib_target_path: &str) -> bool {
    let lib_path = format!("{}/libdclgodot_ios/libdclgodot.a", lib_target_path);
    Path::new(&lib_path).exists()
}

fn check_windows_build(lib_target_path: &str) -> bool {
    let lib_path = format!("{}/libdclgodot_windows/dclgodot.dll", lib_target_path);
    Path::new(&lib_path).exists()
}

fn check_macos_build(lib_target_path: &str) -> bool {
    let lib_path = format!("{}/libdclgodot_macos/libdclgodot.dylib", lib_target_path);
    Path::new(&lib_path).exists()
}

pub fn check_godot_installed() -> bool {
    let godot_path = format!("{}/godot/godot4_bin", BIN_FOLDER);
    Path::new(&godot_path).exists()
}

pub fn check_protoc_installed() -> bool {
    let protoc_path = format!("{}/protoc/bin/protoc", BIN_FOLDER);
    Path::new(&protoc_path).exists()
}

pub fn check_export_templates_for_platform(platform: &str) -> bool {
    if let Some(templates_path) = crate::install_dependency::godot_export_templates_path() {
        match platform {
            "android" | "quest" => {
                let files = ["android_debug.apk", "android_release.apk", "android_source.zip"];
                files.iter().all(|f| Path::new(&format!("{}/{}", templates_path, f)).exists())
            }
            "ios" => Path::new(&format!("{}/ios.zip", templates_path)).exists(),
            "linux" => {
                let files = ["linux_debug.x86_64", "linux_release.x86_64"];
                files.iter().all(|f| Path::new(&format!("{}/{}", templates_path, f)).exists())
            }
            "windows" | "win64" => {
                let files = ["windows_debug_x86_64.exe", "windows_release_x86_64.exe"];
                files.iter().all(|f| Path::new(&format!("{}/{}", templates_path, f)).exists())
            }
            "macos" => Path::new(&format!("{}/macos.zip", templates_path)).exists(),
            _ => false,
        }
    } else {
        false
    }
}

pub fn check_command_dependencies(command: &str, target: Option<&str>) -> Result<(), anyhow::Error> {
    let build_status = BuildStatus::check();
    
    match command {
        "run" => {
            // run command does build+run, so only needs Godot installed
            if !check_godot_installed() {
                print_message(MessageType::Error, "Godot is not installed");
                print_message(MessageType::Info, "Run: cargo run -- install");
                return Err(anyhow::anyhow!("Missing Godot installation"));
            }
            
            // No need to check build status as run does build+run
        }
        
        "import-assets" => {
            if !check_godot_installed() {
                print_message(MessageType::Error, "Godot is not installed");
                print_message(MessageType::Info, "Run: cargo run -- install");
                return Err(anyhow::anyhow!("Missing Godot installation"));
            }
            
            // import-assets calls Godot which needs the library, but we'll build it first in main.rs
        }
        
        "export" => {
            if !check_godot_installed() {
                print_message(MessageType::Error, "Godot is not installed");
                print_message(MessageType::Info, "Run: cargo run -- install");
                return Err(anyhow::anyhow!("Missing Godot installation"));
            }
            
            if !build_status.host_built {
                print_message(MessageType::Error, "Host platform not built");
                print_message(MessageType::Info, "Run: cargo run -- build");
                return Err(anyhow::anyhow!("Missing host build"));
            }
            
            if let Some(target_platform) = target {
                if !build_status.is_built_for_target(target_platform) {
                    print_message(MessageType::Error, &format!("Target platform '{}' not built", target_platform));
                    print_message(MessageType::Info, &format!("Run: cargo run -- build --target {}", target_platform));
                    return Err(anyhow::anyhow!("Missing target platform build"));
                }
                
                if !check_export_templates_for_platform(target_platform) {
                    print_message(MessageType::Error, &format!("Export templates for '{}' not installed", target_platform));
                    print_message(MessageType::Info, &format!("Run: cargo run -- install --platforms {}", target_platform));
                    return Err(anyhow::anyhow!("Missing export templates"));
                }
            }
        }
        
        "build" => {
            if !check_protoc_installed() {
                print_message(MessageType::Error, "protoc is not installed");
                print_message(MessageType::Info, "Run: cargo run -- install");
                return Err(anyhow::anyhow!("Missing protoc installation"));
            }
        }
        
        _ => {}
    }
    
    Ok(())
}

pub fn suggest_next_steps(command: &str, target: Option<&str>) {
    let build_status = BuildStatus::check();
    
    print_message(MessageType::Success, "Command completed successfully!");
    
    match command {
        "install" => {
            print_section("Next Steps");
            
            if !build_status.host_built {
                print_message(MessageType::Step, "Build the project for your host platform:");
                print_message(MessageType::Info, "  cargo run -- build");
            }
            
            print_message(MessageType::Step, "To build for specific platforms:");
            print_message(MessageType::Info, "  cargo run -- build --target android");
            print_message(MessageType::Info, "  cargo run -- build --target ios");
            
            print_message(MessageType::Step, "Check your setup:");
            print_message(MessageType::Info, "  cargo run -- doctor");
        }
        
        "build" => {
            print_section("Next Steps");
            
            if let Some(target_platform) = target {
                if target_platform == "android" || target_platform == "quest" {
                    print_message(MessageType::Step, "Generate keystore for Android signing:");
                    print_message(MessageType::Info, "  cargo run -- generate-keystore");
                    
                    print_message(MessageType::Step, "Export Android APK:");
                    print_message(MessageType::Info, "  cargo run -- export --target android --format apk");
                    
                    print_message(MessageType::Step, "Export Android AAB for Play Store:");
                    print_message(MessageType::Info, "  cargo run -- export --target android --format aab");
                }
            } else {
                print_message(MessageType::Step, "Run the Godot editor:");
                print_message(MessageType::Info, "  cargo run -- run -e");
                
                print_message(MessageType::Step, "Run the client:");
                print_message(MessageType::Info, "  cargo run -- run");
                
                print_message(MessageType::Step, "Import assets:");
                print_message(MessageType::Info, "  cargo run -- import-assets");
                
                if build_status.android_built {
                    print_message(MessageType::Step, "Export for Android:");
                    print_message(MessageType::Info, "  cargo run -- export --target android");
                }
            }
        }
        
        "export" => {
            if let Some(target_platform) = target {
                print_section("Export Completed");
                print_message(MessageType::Info, &format!("Check the exports/ directory for your {} build", target_platform));
            }
        }
        
        _ => {}
    }
}