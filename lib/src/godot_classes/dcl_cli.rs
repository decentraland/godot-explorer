use godot::classes::{Engine, Node, Os, SceneTree};
use godot::obj::Singleton;
use godot::prelude::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
struct ArgDefinition {
    name: String,
    description: String,
    arg_type: ArgType,
    category: String,
}

#[derive(Debug, Clone)]
enum ArgType {
    Flag,
    Value(String), // placeholder for expected value
}

#[derive(GodotClass)]
#[class(base=Node)]
pub struct DclCli {
    _base: Base<Node>,

    // Cached parsed arguments
    args: PackedStringArray,
    args_map: HashMap<String, Option<String>>,

    // Argument definitions for help system
    arg_definitions: Vec<ArgDefinition>,

    // Common flags
    #[var(get)]
    pub force_mobile: bool,
    #[var(get)]
    pub skip_lobby: bool,
    #[var(get)]
    pub guest_profile: bool,
    #[var(get)]
    pub preview_mode: bool,
    #[var(get)]
    pub scene_test_mode: bool,
    #[var(get)]
    pub scene_renderer_mode: bool,
    #[var(get)]
    pub avatar_renderer_mode: bool,
    #[var(get)]
    pub client_test_mode: bool,
    #[var(get)]
    pub test_runner: bool,
    #[var(get)]
    pub clear_cache_startup: bool,
    #[var(get)]
    pub raycast_debugger: bool,
    #[var(get)]
    pub network_debugger: bool,
    #[var(get)]
    pub spawn_avatars: bool,
    #[var(get)]
    pub debug_minimap: bool,
    #[var(get)]
    pub debug_panel: bool,
    #[var(get)]
    pub use_test_input: bool,
    #[var(get)]
    pub test_camera_tune: bool,
    #[var(get)]
    pub measure_perf: bool,
    #[var(get)]
    pub dcl_benchmark: bool,
    #[var(get)]
    pub benchmark_report: bool,
    #[var(get)]
    pub fixed_skybox_time: bool,
    #[var(get)]
    pub developer_mode: bool,
    #[var(get)]
    pub help_requested: bool,
    #[var(get)]
    pub only_optimized: bool,
    #[var(get)]
    pub only_no_optimized: bool,
    #[var(get)]
    pub emote_test_mode: bool,

    // Arguments with values
    #[var(get)]
    pub realm: GString,
    #[var(get)]
    pub location: GString,
    #[var(get)]
    pub scene_input_file: GString,
    #[var(get)]
    pub avatars_file: GString,
    #[var(get)]
    pub snapshot_folder: GString,
}

impl DclCli {
    fn register_arguments() -> Vec<ArgDefinition> {
        vec![
            // General
            ArgDefinition {
                name: "--dcl-help".to_string(),
                description: "Show this help message and exit".to_string(),
                arg_type: ArgType::Flag,
                category: "General".to_string(),
            },
            ArgDefinition {
                name: "--dev".to_string(),
                description: "Enable developer mode with additional debugging features".to_string(),
                arg_type: ArgType::Flag,
                category: "General".to_string(),
            },
            // UI/Display
            ArgDefinition {
                name: "--force-mobile".to_string(),
                description: "Force mobile UI mode on desktop platforms".to_string(),
                arg_type: ArgType::Flag,
                category: "UI/Display".to_string(),
            },
            ArgDefinition {
                name: "--skip-lobby".to_string(),
                description: "Skip the lobby screen and go directly to the explorer".to_string(),
                arg_type: ArgType::Flag,
                category: "UI/Display".to_string(),
            },
            // Authentication
            ArgDefinition {
                name: "--guest-profile".to_string(),
                description: "Use a guest profile without authentication".to_string(),
                arg_type: ArgType::Flag,
                category: "Authentication".to_string(),
            },
            // World/Scene
            ArgDefinition {
                name: "--realm".to_string(),
                description: "Specify the realm URL to connect to".to_string(),
                arg_type: ArgType::Value("<URL>".to_string()),
                category: "World/Scene".to_string(),
            },
            ArgDefinition {
                name: "--location".to_string(),
                description: "Starting location in the world (format: x,y)".to_string(),
                arg_type: ArgType::Value("<x,y>".to_string()),
                category: "World/Scene".to_string(),
            },
            ArgDefinition {
                name: "--preview".to_string(),
                description: "Enable preview mode for scene development".to_string(),
                arg_type: ArgType::Flag,
                category: "World/Scene".to_string(),
            },
            ArgDefinition {
                name: "--spawn-avatars".to_string(),
                description: "Spawn test avatars in the scene".to_string(),
                arg_type: ArgType::Flag,
                category: "World/Scene".to_string(),
            },
            // Testing/Development
            ArgDefinition {
                name: "--scene-test".to_string(),
                description: "Run in scene test mode with specified scenes".to_string(),
                arg_type: ArgType::Value("[[x,y],...]".to_string()),
                category: "Testing".to_string(),
            },
            ArgDefinition {
                name: "--client-test".to_string(),
                description: "Run client visual tests (avatar outline tests)".to_string(),
                arg_type: ArgType::Flag,
                category: "Testing".to_string(),
            },
            ArgDefinition {
                name: "--scene-renderer".to_string(),
                description: "Run in scene renderer mode".to_string(),
                arg_type: ArgType::Flag,
                category: "Testing".to_string(),
            },
            ArgDefinition {
                name: "--scene-input-file".to_string(),
                description: "Path to scene input file for renderer".to_string(),
                arg_type: ArgType::Value("<file>".to_string()),
                category: "Testing".to_string(),
            },
            ArgDefinition {
                name: "--avatar-renderer".to_string(),
                description: "Run in avatar renderer mode".to_string(),
                arg_type: ArgType::Flag,
                category: "Testing".to_string(),
            },
            ArgDefinition {
                name: "--emote-test".to_string(),
                description: "Run emote batch test (cycles through all emotes then exits)".to_string(),
                arg_type: ArgType::Flag,
                category: "Testing".to_string(),
            },
            ArgDefinition {
                name: "--avatars".to_string(),
                description: "Path to avatars input file for renderer".to_string(),
                arg_type: ArgType::Value("<file>".to_string()),
                category: "Testing".to_string(),
            },
            ArgDefinition {
                name: "--test-runner".to_string(),
                description: "Run Godot integration tests".to_string(),
                arg_type: ArgType::Flag,
                category: "Testing".to_string(),
            },
            ArgDefinition {
                name: "--use-test-input".to_string(),
                description: "Use test input files for scene/avatar renderers".to_string(),
                arg_type: ArgType::Flag,
                category: "Testing".to_string(),
            },
            ArgDefinition {
                name: "--test-camera-tune".to_string(),
                description: "Enable camera tuning mode for scene renderer".to_string(),
                arg_type: ArgType::Flag,
                category: "Testing".to_string(),
            },
            ArgDefinition {
                name: "--snapshot-folder".to_string(),
                description: "Folder path for storing test snapshots".to_string(),
                arg_type: ArgType::Value("<folder>".to_string()),
                category: "Testing".to_string(),
            },
            // Debugging
            ArgDefinition {
                name: "--raycast-debugger".to_string(),
                description: "Enable visual raycast debugging".to_string(),
                arg_type: ArgType::Flag,
                category: "Debugging".to_string(),
            },
            ArgDefinition {
                name: "--network-debugger".to_string(),
                description: "Enable network inspector window".to_string(),
                arg_type: ArgType::Flag,
                category: "Debugging".to_string(),
            },
            ArgDefinition {
                name: "--debug-minimap".to_string(),
                description: "Enable debug minimap overlay".to_string(),
                arg_type: ArgType::Flag,
                category: "Debugging".to_string(),
            },
            ArgDefinition {
                name: "--debug-panel".to_string(),
                description: "Show the debug panel UI".to_string(),
                arg_type: ArgType::Flag,
                category: "Debugging".to_string(),
            },
            // Performance
            ArgDefinition {
                name: "--measure-perf".to_string(),
                description: "Enable performance measurement and logging".to_string(),
                arg_type: ArgType::Flag,
                category: "Performance".to_string(),
            },
            ArgDefinition {
                name: "--dcl-benchmark".to_string(),
                description: "Run automated benchmark tests".to_string(),
                arg_type: ArgType::Flag,
                category: "Performance".to_string(),
            },
            ArgDefinition {
                name: "--benchmark-report".to_string(),
                description: "Run benchmarks and generate markdown reports".to_string(),
                arg_type: ArgType::Flag,
                category: "Performance".to_string(),
            },
            // Maintenance
            ArgDefinition {
                name: "--clear-cache-startup".to_string(),
                description: "Clear the cache on startup".to_string(),
                arg_type: ArgType::Flag,
                category: "Maintenance".to_string(),
            },
            // Asset Loading
            ArgDefinition {
                name: "--only-optimized".to_string(),
                description: "Only load optimized assets (skip scenes without optimized assets)"
                    .to_string(),
                arg_type: ArgType::Flag,
                category: "Asset Loading".to_string(),
            },
            ArgDefinition {
                name: "--only-no-optimized".to_string(),
                description: "Only load non-optimized assets (ignore optimized assets)".to_string(),
                arg_type: ArgType::Flag,
                category: "Asset Loading".to_string(),
            },
        ]
    }

    fn print_help() {
        println!("Decentraland Godot Explorer");
        println!("============================");
        println!("Usage: godot-explorer [OPTIONS]");
        println!("Note: Use --dcl-help instead of --help (which is reserved by Godot)");
        println!();

        let definitions = Self::register_arguments();
        let mut categories: HashMap<String, Vec<&ArgDefinition>> = HashMap::new();

        // Group by category
        for def in &definitions {
            categories
                .entry(def.category.clone())
                .or_default()
                .push(def);
        }

        // Sort categories for consistent output
        let mut sorted_categories: Vec<_> = categories.keys().collect();
        sorted_categories.sort();

        for category in sorted_categories {
            println!("{}:", category);
            if let Some(args) = categories.get(category) {
                for arg in args {
                    let arg_display = match &arg.arg_type {
                        ArgType::Flag => format!("  {:<30}", arg.name),
                        ArgType::Value(placeholder) => {
                            format!("  {} {:<20}", arg.name, placeholder)
                        }
                    };
                    println!("{} {}", arg_display, arg.description);
                }
            }
            println!();
        }

        println!("Examples:");
        println!("  # Show this help message");
        println!("  godot-explorer --dcl-help");
        println!();
        println!("  # Run with guest profile at specific location");
        println!("  godot-explorer --guest-profile --location 52,-52");
        println!();
        println!("  # Run in preview mode with custom realm");
        println!("  godot-explorer --preview --realm https://my-realm.com");
        println!();
        println!("  # Run scene tests with debugging");
        println!("  godot-explorer --scene-test [[0,0],[1,1]] --raycast-debugger");
    }
}

#[godot_api]
impl INode for DclCli {
    fn init(base: Base<Node>) -> Self {
        let args = Os::singleton().get_cmdline_args();
        let mut args_map = HashMap::new();

        // Add default arguments
        //args_map.insert("--skip-lobby".to_string(), None); // debug

        // Parse command line arguments into a map
        let args_vec = args.to_vec();
        let mut i = 0;
        while i < args_vec.len() {
            let arg = args_vec[i].to_string();
            if arg.starts_with("--") {
                // Check if next arg is a value (doesn't start with --)
                if i + 1 < args_vec.len() && !args_vec[i + 1].to_string().starts_with("--") {
                    args_map.insert(arg.clone(), Some(args_vec[i + 1].to_string()));
                    i += 2;
                } else {
                    args_map.insert(arg.clone(), None);
                    i += 1;
                }
            } else {
                i += 1;
            }
        }

        // Check for help flag first
        let help_requested = args_map.contains_key("--dcl-help");
        if help_requested {
            Self::print_help();
            // In a real application, you might want to exit here
            // For Godot, we'll let the application handle it
        }

        // Extract common flags
        let force_mobile = args_map.contains_key("--force-mobile");
        let skip_lobby = args_map.contains_key("--skip-lobby");
        let guest_profile = args_map.contains_key("--guest-profile");
        let preview_mode = args_map.contains_key("--preview");
        let scene_test_mode = args_map.contains_key("--scene-test");
        let scene_renderer_mode = args_map.contains_key("--scene-renderer");
        let avatar_renderer_mode = args_map.contains_key("--avatar-renderer");
        let client_test_mode = args_map.contains_key("--client-test");
        let test_runner = args_map.contains_key("--test-runner");
        let clear_cache_startup = args_map.contains_key("--clear-cache-startup");
        let raycast_debugger = args_map.contains_key("--raycast-debugger");
        let network_debugger = args_map.contains_key("--network-debugger");
        let spawn_avatars = args_map.contains_key("--spawn-avatars");
        let debug_minimap = args_map.contains_key("--debug-minimap");
        let debug_panel = args_map.contains_key("--debug-panel") || preview_mode;
        let use_test_input = args_map.contains_key("--use-test-input");
        let test_camera_tune = args_map.contains_key("--test-camera-tune");
        let measure_perf = args_map.contains_key("--measure-perf");
        let dcl_benchmark = args_map.contains_key("--dcl-benchmark");
        let benchmark_report = args_map.contains_key("--benchmark-report");
        let developer_mode = args_map.contains_key("--dev");
        let fixed_skybox_time = scene_test_mode || scene_renderer_mode;
        let only_optimized = args_map.contains_key("--only-optimized");
        let only_no_optimized = args_map.contains_key("--only-no-optimized");
        let emote_test_mode = args_map.contains_key("--emote-test");

        // Extract arguments with values
        let realm = args_map
            .get("--realm")
            .and_then(|v| v.as_ref())
            .map(GString::from)
            .unwrap_or_default();
        let location = args_map
            .get("--location")
            .and_then(|v| v.as_ref())
            .map(GString::from)
            .unwrap_or_default();
        let scene_input_file = args_map
            .get("--scene-input-file")
            .and_then(|v| v.as_ref())
            .map(GString::from)
            .unwrap_or_default();
        let avatars_file = args_map
            .get("--avatars")
            .and_then(|v| v.as_ref())
            .map(GString::from)
            .unwrap_or_default();
        let snapshot_folder = args_map
            .get("--snapshot-folder")
            .and_then(|v| v.as_ref())
            .map(GString::from)
            .unwrap_or_default();

        Self {
            _base: base,
            args,
            args_map,
            arg_definitions: Self::register_arguments(),
            force_mobile,
            skip_lobby,
            guest_profile,
            preview_mode,
            scene_test_mode,
            scene_renderer_mode,
            avatar_renderer_mode,
            client_test_mode,
            test_runner,
            clear_cache_startup,
            raycast_debugger,
            network_debugger,
            spawn_avatars,
            debug_minimap,
            debug_panel,
            use_test_input,
            test_camera_tune,
            measure_perf,
            dcl_benchmark,
            benchmark_report,
            fixed_skybox_time,
            developer_mode,
            help_requested,
            only_optimized,
            only_no_optimized,
            emote_test_mode,
            realm,
            location,
            scene_input_file,
            avatars_file,
            snapshot_folder,
        }
    }
}

#[godot_api]
impl DclCli {
    #[func]
    pub fn has_arg(&self, arg: GString) -> bool {
        self.args_map.contains_key(&arg.to_string())
    }

    #[func]
    pub fn get_arg(&self, arg: GString) -> GString {
        self.args_map
            .get(&arg.to_string())
            .and_then(|v| v.as_ref())
            .map(GString::from)
            .unwrap_or_default()
    }

    #[func]
    pub fn get_arg_or_default(&self, arg: GString, default: GString) -> GString {
        let value = self.get_arg(arg);
        if value.is_empty() {
            default
        } else {
            value
        }
    }

    #[func]
    pub fn get_all_args(&self) -> PackedStringArray {
        self.args.clone()
    }

    // Helper to parse location string like "52,-52" into Vector2i
    #[func]
    pub fn get_location_vector(&self) -> Vector2i {
        if self.location.is_empty() {
            return Vector2i::MAX;
        }

        let loc_str = self.location.to_string();
        let parts: Vec<&str> = loc_str.split(',').collect();
        if parts.len() == 2 {
            if let (Ok(x), Ok(y)) = (parts[0].parse::<i32>(), parts[1].parse::<i32>()) {
                return Vector2i::new(x, y);
            }
        }
        Vector2i::MAX
    }

    #[func]
    pub fn get_help_text(&self) -> GString {
        let mut help = String::new();
        help.push_str("Decentraland Godot Explorer\n");
        help.push_str("============================\n");
        help.push_str("Usage: godot-explorer [OPTIONS]\n");
        help.push_str("Note: Use --dcl-help instead of --help (which is reserved by Godot)\n\n");

        let mut categories: HashMap<String, Vec<&ArgDefinition>> = HashMap::new();

        // Group by category
        for def in &self.arg_definitions {
            categories
                .entry(def.category.clone())
                .or_default()
                .push(def);
        }

        // Sort categories
        let mut sorted_categories: Vec<_> = categories.keys().collect();
        sorted_categories.sort();

        for category in sorted_categories {
            help.push_str(&format!("{}:\n", category));
            if let Some(args) = categories.get(category) {
                for arg in args {
                    let arg_display = match &arg.arg_type {
                        ArgType::Flag => format!("  {:<30}", arg.name),
                        ArgType::Value(placeholder) => {
                            format!("  {} {:<20}", arg.name, placeholder)
                        }
                    };
                    help.push_str(&format!("{} {}\n", arg_display, arg.description));
                }
            }
            help.push('\n');
        }

        GString::from(&help)
    }

    #[func]
    pub fn is_help_requested(&self) -> bool {
        self.help_requested
    }

    // Singleton access
    pub fn try_singleton() -> Option<Gd<Self>> {
        let res = Engine::singleton()
            .get_main_loop()?
            .cast::<SceneTree>()
            .get_root()?
            .get_node_or_null("DclCli")?
            .try_cast::<Self>();
        res.ok()
    }

    pub fn singleton() -> Gd<Self> {
        Self::try_singleton().expect("Failed to get DclCli singleton!")
    }
}
