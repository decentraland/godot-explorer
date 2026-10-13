//! Creation of the services the app runs on. `Global` (a GDScript autoload) calls these from its
//! `_ready`, so the services exist before the autoloads that follow it read them; the property
//! assignment goes through `Global`'s own setters, exactly as the GDScript `.new()` lines did.

use godot::classes::{GDScript, ResourceLoader};
use godot::prelude::*;

/// Where a created node goes.
#[derive(Clone, Copy)]
enum Attach {
    /// Held by the property only.
    None,
    /// A child of `Global`, added immediately.
    GlobalChild,
}

struct ServiceSpec {
    /// The `Global` property that holds the service.
    property: &'static str,
    /// Path of the GDScript class, under `res://src/`.
    script: &'static str,
    /// `Node.name` to set, when the old code set one.
    node_name: Option<&'static str>,
    attach: Attach,
}

const fn spec(
    property: &'static str,
    script: &'static str,
    node_name: Option<&'static str>,
    attach: Attach,
) -> ServiceSpec {
    ServiceSpec {
        property,
        script,
        node_name,
        attach,
    }
}

/// Services every boot needs, in creation order.
const CORE_SERVICES: [ServiceSpec; 10] = [
    spec(
        "nft_frame_loader",
        "decentraland_components/nft_frame_style_loader.gd",
        None,
        Attach::None,
    ),
    spec(
        "nft_fetcher",
        "logic/content/opensea_nft_fetcher.gd",
        None,
        Attach::None,
    ),
    spec(
        "music_player",
        "helpers_components/music_player.gd",
        None,
        Attach::None,
    ),
    spec("snapshot", "logic/snapshot.gd", None, Attach::GlobalChild),
    spec(
        "preload_assets",
        "logic/preload_assets.gd",
        None,
        Attach::None,
    ),
    spec("config", "config/config_data.gd", None, Attach::None),
    spec("realm", "logic/realm.gd", Some("realm"), Attach::None),
    spec(
        "player_identity",
        "logic/player/player_identity.gd",
        Some("player_identity"),
        Attach::None,
    ),
    spec(
        "testing_tools",
        "test/testing_api.gd",
        Some("testing_tool"),
        Attach::None,
    ),
    spec(
        "portable_experience_controller",
        "logic/portable_experience_controller.gd",
        Some("portable_experience_controller"),
        Attach::None,
    ),
];

/// A new instance of the GDScript class at `res://src/<script>`.
fn instantiate_script(script: &str) -> Result<Variant, String> {
    let path = GString::from(&format!("res://src/{script}"));
    let resource = ResourceLoader::singleton()
        .load_ex(&path)
        .type_hint("Script")
        .done()
        .ok_or_else(|| format!("{path} did not load"))?;
    let mut script = resource
        .try_cast::<GDScript>()
        .map_err(|_| format!("{path} is not a GDScript"))?;
    let instance = script.call("new", &[]);
    if instance.is_nil() {
        return Err(format!("{path}: new() returned nothing"));
    }
    Ok(instance)
}

fn create(global: &mut Gd<Node>, specs: &[ServiceSpec]) -> bool {
    let mut ok = true;
    for service in specs {
        let instance = match instantiate_script(service.script) {
            Ok(instance) => instance,
            Err(reason) => {
                godot_error!("bootstrap: cannot create `{}`: {reason}", service.property);
                ok = false;
                continue;
            }
        };
        if let (Some(name), Ok(mut node)) = (service.node_name, instance.try_to::<Gd<Node>>()) {
            node.set_name(name);
        }
        global.set(service.property, &instance);
        if let (Attach::GlobalChild, Ok(node)) = (service.attach, instance.try_to::<Gd<Node>>()) {
            global.add_child(&node);
        }
    }
    ok
}

/// Static entry points for `global.gd`.
#[derive(GodotClass)]
#[class(init, base=RefCounted)]
pub struct DclBootstrap {
    _base: Base<RefCounted>,
}

#[godot_api]
impl DclBootstrap {
    /// Creates the services every boot needs and assigns them to `global`. Returns false if any
    /// could not be created (each failure is logged).
    #[func]
    pub fn create_core_services(mut global: Gd<Node>) -> bool {
        create(&mut global, &CORE_SERVICES)
    }
}
