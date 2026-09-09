use chrono::prelude::*;
use std::process::Command;
use std::{
    env,
    fs::{self, File},
    io::{self, Write},
    path::Path,
};

mod build_quant;

struct Component {
    id: u32,
    pascal_name: String,
    snake_name: String,
}

const PROTO_FILES_BASE_DIR: &str = "src/dcl/components/proto/";
const COMPONENT_BASE_DIR: &str = "src/dcl/components/proto/decentraland/sdk/components/";
const GROW_ONLY_SET_COMPONENTS: [&str; 6] = [
    "PointerEventsResult",
    "VideoEvent",
    "AvatarEmoteCommand",
    // The SDK defines AudioEvent as a GrowOnlyValueSet (js-sdk-toolchain
    // `generateIndex.ts`); registering it as LWW would not reach the scene.
    "AudioEvent",
    "TriggerAreaResult",
    // The renderer appends one value per-asset transition; the scene-side
    // SDK reads AssetLoadLoadingState as a GrowOnlyValueSet (protocol PR #339).
    "AssetLoadLoadingState",
];

pub fn snake_to_pascal(input: &str) -> String {
    input
        .split('_')
        .map(|part| {
            upper_first(
                &part
                    .split('/')
                    .map(upper_first)
                    .collect::<Vec<String>>()
                    .join("/"),
            )
        })
        .collect::<String>()
}

fn upper_first(input: &str) -> String {
    let mut chars = input.chars();
    match chars.next() {
        Some(first_char) => first_char.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

fn get_component_id(proto_content: &str) -> Result<u32, String> {
    let component_id_line = proto_content
        .lines()
        .filter(|line| line.contains("ecs_component_id") && line.contains("option"))
        .collect::<Vec<&str>>();

    if component_id_line.len() > 1 {
        return Err("There are more than one match with `ecs_component_id` and `option`. Please reserve this keyword to only the definition of ComponentId".to_string());
    } else if component_id_line.is_empty() {
        return Err("`ecs_component_id` is missing.".to_string());
    }

    let component_id_value = component_id_line[0]
        .split('=')
        .nth(1)
        .unwrap_or("111111111")
        .trim();

    let parsed_component_id = component_id_value
        .split(|c: char| !c.is_ascii_digit())
        .find(|s| !s.is_empty())
        .ok_or_else(|| format!("Failed to parse `ecs_component_id` value: {component_id_value}"))?;

    let parsed_component_id = parsed_component_id.parse::<u32>().map_err(|err| {
        format!("Failed to parse `ecs_component_id` value: {component_id_value}, err: {err:?}")
    })?;

    Ok(parsed_component_id)
}

fn get_component_id_and_name(file_path: &str) -> Component {
    let contents = fs::read_to_string(file_path).expect("Should have been able to read the file");

    let id = get_component_id(&contents).unwrap();

    let snake_name = &file_path[COMPONENT_BASE_DIR.len()..file_path.len() - 6];
    let pascal_name = snake_to_pascal(snake_name);

    Component {
        id,
        pascal_name,
        snake_name: String::from(snake_name),
    }
}

fn generate_dcl_component_impl(proto_components: &Vec<Component>) {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("dclcomponent.proto.impl.gen.rs");

    let mut output_str = String::new();
    for component in proto_components {
        output_str += &format!(
            "impl DclProtoComponent for sdk::components::Pb{} {{}}\n",
            component.pascal_name
        );
    }
    generate_file(dest_path, output_str.as_bytes());
}

fn generate_enum(proto_components: &Vec<Component>) {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("components_enum.gen.rs");

    let mut output_str = String::new();
    for component in proto_components {
        output_str += &format!(
            "pub const {}: SceneComponentId = SceneComponentId({});\n",
            component.snake_name.to_uppercase(),
            component.id
        );
    }

    // Generate component_id_to_name function
    let mut name_mapping = String::from(
        "pub fn component_id_to_name(id: u32) -> &'static str {\n    match id {\n        1 => \"Transform\",\n        101 => \"InternalPlayerData\",\n"
    );
    for component in proto_components {
        name_mapping += &format!(
            "        {} => \"{}\",\n",
            component.id, component.pascal_name
        );
    }
    name_mapping += "        _ => \"Unknown\",\n    }\n}\n";

    let output_str = format!("impl SceneComponentId {{ {output_str} }}\n\n{name_mapping}");
    generate_file(dest_path, output_str.as_bytes());
}

fn generate_impl_crdt(proto_components: &Vec<Component>) {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("crdt_impl.gen.rs");

    let mut defining_proto = String::new();
    let mut lww_getter = String::new();
    let mut gos_getter = String::new();
    let mut lww_getter_mut = String::new();
    let mut gos_getter_mut = String::new();
    let mut custom_proto_methods = String::new();

    for component in proto_components {
        let is_grow_only_set = GROW_ONLY_SET_COMPONENTS
            .iter()
            .any(|&x| x.eq(component.pascal_name.as_str()));

        if is_grow_only_set {
            defining_proto += &format!(
                ".insert_gos_component::<proto_components::sdk::components::Pb{}>(
                SceneComponentId({})
            )\n",
                component.pascal_name, component.id
            );
            gos_getter_mut += &format!(
                "SceneComponentId({}) => self.get_unknown_gos_component_mut::<GrowOnlySet<proto_components::sdk::components::Pb{}>>(SceneComponentId({})),\n",
                component.id, component.pascal_name, component.id
            );
            gos_getter += &format!(
                "SceneComponentId({}) => self.get_unknown_gos_component::<GrowOnlySet<proto_components::sdk::components::Pb{}>>(SceneComponentId({})),\n",
                component.id, component.pascal_name, component.id
            );
            custom_proto_methods += &format!(
                "#[allow(dead_code)]
                pub fn get_{1}_mut(crdt_state: &mut SceneCrdtState) -> &mut GrowOnlySet<proto_components::sdk::components::Pb{0}> {{
                    crdt_state.components
                        .get_mut(&SceneComponentId({2}))
                        .unwrap()
                        .downcast_mut::<GrowOnlySet<proto_components::sdk::components::Pb{0}>>()
                        .unwrap()
                }}\n",
                component.pascal_name, component.snake_name, component.id
            );
            custom_proto_methods += &format!(
                "#[allow(dead_code)]
                pub fn get_{1}(crdt_state: &SceneCrdtState) -> &GrowOnlySet<proto_components::sdk::components::Pb{0}> {{
                    crdt_state.components
                        .get(&SceneComponentId({2}))
                        .unwrap()
                        .downcast_ref::<GrowOnlySet<proto_components::sdk::components::Pb{0}>>()
                        .unwrap()
                }}\n",
                component.pascal_name, component.snake_name, component.id
            );
        } else {
            defining_proto += &format!(
                ".insert_lww_component::<proto_components::sdk::components::Pb{}>(
                SceneComponentId({})
            )\n",
                component.pascal_name, component.id
            );
            lww_getter_mut += &format!(
                "SceneComponentId({0}) => self.get_unknown_lww_component_mut::<LastWriteWins<proto_components::sdk::components::Pb{1}>>(SceneComponentId({0})),\n",
                component.id, component.pascal_name
            );
            lww_getter += &format!(
                "SceneComponentId({0}) => self.get_unknown_lww_component::<LastWriteWins<proto_components::sdk::components::Pb{1}>>(SceneComponentId({0})),\n",
                component.id, component.pascal_name
            );
            custom_proto_methods += &format!(
                "#[allow(dead_code)]
                pub fn get_{1}_mut(crdt_state: &mut SceneCrdtState) -> &mut LastWriteWins<proto_components::sdk::components::Pb{0}> {{
                    crdt_state.components
                        .get_mut(&SceneComponentId({2}))
                        .unwrap()
                        .downcast_mut::<LastWriteWins<proto_components::sdk::components::Pb{0}>>()
                        .unwrap()
                }}\n",
                component.pascal_name, component.snake_name, component.id
            );
            custom_proto_methods += &format!(
                "#[allow(dead_code)]
                pub fn get_{1}(crdt_state: &SceneCrdtState) -> &LastWriteWins<proto_components::sdk::components::Pb{0}> {{
                    crdt_state.components
                        .get(&SceneComponentId({2}))
                        .unwrap()
                        .downcast_ref::<LastWriteWins<proto_components::sdk::components::Pb{0}>>()
                        .unwrap()
                }}\n",
                component.pascal_name, component.snake_name, component.id
            );
        }
    }

    let or_components = proto_components
        .iter()
        .map(|component| component.id.to_string())
        .collect::<Vec<String>>()
        .join(" | ");

    custom_proto_methods += &format!(
        "pub fn is_proto_component_id(id: SceneComponentId) -> bool {{
            matches!(id.0, {or_components})
        }}\n"
    );

    let output_str = format!(
        "
#[allow(unreachable_patterns)]
impl SceneCrdtState {{
    pub fn from_proto() -> Self {{
        let mut crdt_state = Self::default();
        crdt_state{defining_proto};
        crdt_state
    }}

    pub fn get_proto_lww_component_definition(
        &self,
        component_id: SceneComponentId,
    ) -> Option<&dyn GenericLastWriteWinsComponent> {{
        match component_id {{
            {lww_getter}
            _ => None
        }}
    }}

    pub fn get_proto_gos_component_definition(
        &self,
        component_id: SceneComponentId,
    ) -> Option<&dyn GenericGrowOnlySetComponent> {{
        match component_id {{
            {gos_getter}
            _ => None
        }}
    }}
    
    pub fn get_proto_lww_component_definition_mut(
        &mut self,
        component_id: SceneComponentId,
    ) -> Option<&mut dyn GenericLastWriteWinsComponent> {{
        match component_id {{
            {lww_getter_mut}
            _ => None
        }}
    }}

    pub fn get_proto_gos_component_definition_mut(
        &mut self,
        component_id: SceneComponentId,
    ) -> Option<&mut dyn GenericGrowOnlySetComponent> {{
        match component_id {{
            {gos_getter_mut}
            _ => None
        }}
    }}
}}

pub struct SceneCrdtStateProtoComponents();
#[allow(unreachable_patterns)]
impl SceneCrdtStateProtoComponents {{
{custom_proto_methods}
}}
"
    );
    generate_file(dest_path, output_str.as_bytes());
}

/// Generate `deserialize_proto_component_to_json` so the runtime Scene Inspector
/// can decode any proto component without us hand-maintaining a 60+ entry table.
/// Stays in sync with the .proto sources automatically.
fn generate_deserialize_component(proto_components: &Vec<Component>) {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("deserialize_component.gen.rs");

    let mut arms = String::new();
    for component in proto_components {
        arms += &format!(
            "        {} => decode_component!(Pb{}),\n",
            component.id, component.pascal_name
        );
    }

    let body = format!(
        "/// Decode any proto component by id. Generated by build.rs from\n\
        /// the .proto sources so the table never drifts out of sync.\n\
        fn deserialize_proto_component_to_json(\n    \
            component_id: u32,\n    \
            data: &[u8],\n\
        ) -> Option<serde_json::Value> {{\n    \
            use prost::Message;\n    \
            use sdk::components::*;\n    \
            macro_rules! decode_component {{\n        \
                ($type:ty) => {{{{\n            \
                    <$type>::decode(data)\n                \
                        .ok()\n                \
                        .and_then(|v| serde_json::to_value(v).ok())\n        \
                }}}};\n    \
            }}\n    \
            match component_id {{\n\
{arms}        \
                _ => None,\n    \
            }}\n\
        }}\n"
    );
    generate_file(dest_path, body.as_bytes());
}

fn main() -> io::Result<()> {
    // ---------- Linux, Android, the BSDs, Windows-gnu, and other ld/LLD users
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    let target_vendor = env::var("CARGO_CFG_TARGET_VENDOR").unwrap_or_default();

    if matches!(
        target_os.as_str(),
        "linux" | "android" | "freebsd" | "netbsd" | "openbsd" | "dragonfly"
    ) || (target_os == "windows" && target_env == "gnu")
    {
        println!("cargo:rustc-link-arg=-Wl,--allow-multiple-definition");
    }

    // ---------- macOS & iOS (Apple ld64)
    //
    //  -multiply_defined,suppress   = choose first definition, ignore the rest
    if target_vendor == "apple" || target_os == "ios" {
        println!("cargo:rustc-link-arg=-Wl,-multiply_defined,suppress");
    }

    // ---------- Windows MSVC (link.exe or lld-link)
    //
    //  /FORCE:MULTIPLE  = keep first symbol, drop duplicates
    // Only apply this when actually building FOR Windows, not just ON Windows
    if env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows"
        && env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default() == "msvc"
    {
        println!("cargo:rustc-link-arg=/FORCE:MULTIPLE");
    }

    // Must run before ANY patch is written, or it deletes the copies produced
    // below (the component loop patches avatar_emote_command.proto).
    clear_patched_proto_root();

    let mut proto_components = vec![];
    let mut proto_files = vec![];
    let dir_path = Path::new(COMPONENT_BASE_DIR);
    for entry in fs::read_dir(dir_path)
        .expect("Failed to read directory")
        .flatten()
    {
        if let Some(extension) = entry.path().extension() {
            if extension == "proto" {
                // Component id/name derivation always uses the pristine npm copy;
                // the compiled source may be swapped for a patched copy below.
                proto_components.push(get_component_id_and_name(entry.path().to_str().unwrap()));

                if entry.path().file_name().and_then(|n| n.to_str())
                    == Some("avatar_emote_command.proto")
                {
                    proto_files.push(patched_avatar_emote_command_proto());
                } else {
                    proto_files.push(entry.path());
                }
            }
        }
    }

    proto_files.push(
        format!("{PROTO_FILES_BASE_DIR}decentraland/kernel/comms/rfc5/ws_comms.proto").into(),
    );
    proto_files.push(patched_rfc4_comms_proto());
    proto_files.push(
        format!("{PROTO_FILES_BASE_DIR}decentraland/kernel/comms/v3/archipelago.proto").into(),
    );

    // Pulse comms protos, shipped by the pinned @dcl/protocol tarball (built from protocol
    // `main`, commit 0ff6038 — see PROTOCOL_FIXED_VERSION_URL in src/install_dependency.rs). Compiled
    // unconditionally — runtime code is gated by the `use_pulse` feature instead, keeping the
    // build script feature-free. options.proto must be in the compile set so the
    // FileDescriptorSet carries the quantization extension values for build_quant.
    proto_files.push(format!("{PROTO_FILES_BASE_DIR}decentraland/common/options.proto").into());
    proto_files.push(format!("{PROTO_FILES_BASE_DIR}decentraland/pulse/pulse_shared.proto").into());
    proto_files.push(format!("{PROTO_FILES_BASE_DIR}decentraland/pulse/pulse_client.proto").into());
    proto_files.push(format!("{PROTO_FILES_BASE_DIR}decentraland/pulse/pulse_server.proto").into());

    // Preview hot-reload protocol: the sdk-commands preview server broadcasts
    // WsSceneMessage frames over the preview WebSocket (see PreviewWebSocket).
    proto_files.push(
        format!("{PROTO_FILES_BASE_DIR}decentraland/sdk/development/local_development.proto")
            .into(),
    );

    // Social service protos (with RPC services)
    proto_files
        .push(format!("{PROTO_FILES_BASE_DIR}decentraland/social_service/errors.proto").into());
    proto_files.push(
        format!("{PROTO_FILES_BASE_DIR}decentraland/social_service/v2/social_service_v2.proto")
            .into(),
    );

    generate_enum(&proto_components);
    generate_impl_crdt(&proto_components);
    generate_dcl_component_impl(&proto_components);
    generate_deserialize_component(&proto_components);

    let mut protoc_path = std::env::current_dir()
        .unwrap()
        .join("../.bin/protoc/bin/protoc");
    if std::env::consts::OS == "windows" {
        protoc_path.set_extension("exe");
    }
    let protoc_path = protoc_path
        .canonicalize()
        .expect("Failed to canonicalize protoc path");

    std::env::set_var("PROTOC", protoc_path);

    // Always derive serde::Serialize on proto types so the runtime Scene Inspector
    // can serialize component payloads when --scene-inspector is enabled.
    //
    // This is intentionally applied to ALL proto types (`"."`) rather than a
    // hand-curated whitelist. Rationale:
    //   - Zero runtime overhead: when the derive isn't called, the compiler
    //     eliminates it as dead code; only a few KB of binary footprint remain.
    //   - Maintenance: a whitelist drifts out of sync with the .proto sources
    //     every time a new component is added; blanket-derive avoids that.
    let mut prost_config = prost_build::Config::new();
    prost_config.type_attribute(".", "#[derive(serde::Serialize)]");
    prost_config.service_generator(Box::new(dcl_rpc::codegen::RPCServiceGenerator::new()));
    // Emit the descriptor set so build_quant can read the Pulse quantization
    // field options (the .proto stays the single source of truth for the wire ABI).
    let descriptor_path = Path::new(&env::var("OUT_DIR").unwrap()).join("proto_descriptor_set.bin");
    prost_config.file_descriptor_set_path(&descriptor_path);
    // The patched root goes first so `decentraland/kernel/comms/rfc4/comms.proto`
    // resolves to the patched copy, not the npm one (same canonical path).
    let proto_patched_root = patched_proto_root();
    prost_config.compile_protos(
        &proto_files,
        &[
            proto_patched_root.as_path(),
            Path::new(PROTO_FILES_BASE_DIR),
        ],
    )?;

    let descriptor_bytes = fs::read(&descriptor_path).expect("read proto descriptor set");
    let quant_path = Path::new(&env::var("OUT_DIR").unwrap()).join("pulse_quant.rs");
    build_quant::generate(&descriptor_bytes, &quant_path);
    println!("cargo:rerun-if-changed=build_quant.rs");
    println!(
        "cargo:rerun-if-changed={PROTO_FILES_BASE_DIR}decentraland/kernel/comms/rfc4/comms.proto"
    );
    // Both patched protos compile from an OUT_DIR copy, so the loop below only
    // watches files this script rewrites itself — watch the npm sources too, or
    // `cargo run -- install` updating @dcl/protocol leaves codegen stale.
    println!(
        "cargo:rerun-if-changed={PROTO_FILES_BASE_DIR}decentraland/sdk/components/avatar_emote_command.proto"
    );

    #[cfg(feature = "use_livekit")]
    if env::var("CARGO_CFG_TARGET_OS").unwrap() == "android" {
        webrtc_sys_build::configure_jni_symbols().unwrap();
    }

    for source in proto_files {
        let value = source.to_str().unwrap();
        println!("cargo:rerun-if-changed={value}");
    }

    set_godot_explorer_version();

    Ok(())
}

/// The pinned @dcl/protocol build (protocol `main`, commit 0ff6038) has an
/// rfc4 `comms.proto` that lacks fields the Pulse transport and Unity interop rely on
/// (they live in protocol `experimental`): `PlayerEmote` 4..=11 (`is_stopping` &
/// co., bridged from Pulse `EmoteStopped` and already sent by Unity peers over
/// LiveKit) and `Chat.forwarded_from` (SFU forwarding). The npm tree is gitignored
/// and wiped by `cargo run -- install`, so it can't be edited in place; instead,
/// patch a copy under OUT_DIR/proto_patched/ and compile that one (that root is
/// listed first so the canonical path resolves to the patched copy). Each patch
/// no-ops once the pinned build ships its fields — delete this when both do.
/// Root of the build-time patched protos, listed first on protoc's include path
/// (see `patched_rfc4_comms_proto`).
fn patched_proto_root() -> std::path::PathBuf {
    Path::new(&env::var("OUT_DIR").unwrap()).join("proto_patched")
}

/// Wipes the patched-proto root before this build repopulates it.
///
/// `OUT_DIR` survives across builds and — on the self-hosted CI runners, which
/// reuse a single checkout and `target/` for every branch — is shared between
/// branches. A patched copy written by *another* branch would otherwise linger
/// here and, because this root comes first on the include path, shadow the npm
/// file that this branch passes to protoc as an input:
///
/// ```text
/// protoc failed: src/dcl/components/proto/decentraland/sdk/components/avatar_emote_command.proto:
/// Input is shadowed in the --proto_path by ".../out/proto_patched/decentraland/sdk/components/avatar_emote_command.proto".
/// ```
///
/// Every build re-derives the patches it needs from the npm tree, so starting
/// from an empty root costs nothing and keeps the set exact.
fn clear_patched_proto_root() {
    let root = patched_proto_root();
    if root.exists() {
        fs::remove_dir_all(&root).expect("clear proto_patched dir");
    }
}

fn patched_rfc4_comms_proto() -> std::path::PathBuf {
    const RFC4_REL: &str = "decentraland/kernel/comms/rfc4/comms.proto";
    let mut source = fs::read_to_string(format!("{PROTO_FILES_BASE_DIR}{RFC4_REL}"))
        .expect("read rfc4 comms.proto (run `cargo run -- install` to fetch protos)");

    if !source.contains("is_stopping") {
        source = insert_fields_before_message_close(
            &source,
            "message PlayerEmote {",
            concat!(
                "  optional bool is_stopping = 4; // true means the emote has been stopped in the sender's client\n",
                "  optional bool is_repeating = 5; // true when it is not the first time the looping animation plays\n",
                "  optional int32 interaction_id = 6; // identifies an interaction univocally\n",
                "  optional int32 social_emote_outcome = 7; // -1 means it does not use an outcome animation\n",
                "  optional bool is_reacting = 8; // to a social emote started by other user\n",
                "  optional string social_emote_initiator = 9; // wallet address of the social emote initiator\n",
                "  optional string target_avatar = 10; // wallet address of the directed emote target\n",
                "  optional uint32 mask = 11; // mask for which bones an animation applies to\n",
            ),
        );
    }
    if !source.contains("forwarded_from") {
        source = insert_fields_before_message_close(
            &source,
            "message Chat {",
            "  optional string forwarded_from = 3; // original sender when forwarded through an SFU\n",
        );
    }

    let dest = patched_proto_root().join(RFC4_REL);
    fs::create_dir_all(dest.parent().unwrap()).expect("create proto_patched dir");
    fs::write(&dest, source).expect("write patched rfc4 comms.proto");
    dest
}

/// The pinned @dcl/protocol build (commit 0ff6038) predates protocol#459, which added
/// the `EmoteState` lifecycle enum and `PBAvatarEmoteCommand.state` (field 5) so scenes
/// can observe emote completion/interruption. Same idiom as `patched_rfc4_comms_proto`:
/// patch a copy under OUT_DIR/proto_patched/ and compile that one instead of the npm
/// copy (which `cargo run -- install` rewrites). The enum is declared at FILE scope to
/// match upstream exactly: prost maps a nested enum to `pb_avatar_emote_command::EmoteState`
/// and a file-scope one to `sdk::components::EmoteState`, so declaring it anywhere else
/// would break every import site the day the pin bumps. This way that bump is a true
/// no-op. No-ops once the pinned build ships the field.
fn patched_avatar_emote_command_proto() -> std::path::PathBuf {
    const REL: &str = "decentraland/sdk/components/avatar_emote_command.proto";
    let mut source = fs::read_to_string(format!("{PROTO_FILES_BASE_DIR}{REL}"))
        .expect("read avatar_emote_command.proto (run `cargo run -- install` to fetch protos)");

    if !source.contains("EmoteState") {
        source = insert_fields_before_message_close(
            &source,
            "message PBAvatarEmoteCommand {",
            "  // When absent (older explorers), defaults to ES_STARTED.\n  optional EmoteState state = 5;\n",
        );
        // File-scope enum, inserted before the message that references it.
        source = source.replace(
            "message PBAvatarEmoteCommand {",
            concat!(
                "// EmoteState describes the lifecycle state of an emote playback.\n",
                "enum EmoteState {\n",
                "  ES_STARTED = 0; // zero value: entries from older explorers read as \"started\"\n",
                "  ES_FINISHED = 1; // non-looping emote completed naturally\n",
                "  ES_INTERRUPTED = 2; // cancelled: movement, stop, superseded, or scene change\n",
                "}\n\n",
                "message PBAvatarEmoteCommand {"
            ),
        );
    }

    let dest = Path::new(&env::var("OUT_DIR").unwrap())
        .join("proto_patched")
        .join(REL);
    fs::create_dir_all(dest.parent().unwrap()).expect("create proto_patched dir");
    fs::write(&dest, source).expect("write patched avatar_emote_command.proto");
    dest
}

fn insert_fields_before_message_close(source: &str, marker: &str, fields: &str) -> String {
    let start = source.find(marker).unwrap_or_else(|| {
        panic!("rfc4 comms.proto: `{marker}` not found — update the patch in build.rs")
    });
    let close = source[start..]
        .find('}')
        .map(|offset| start + offset)
        .unwrap_or_else(|| panic!("rfc4 comms.proto: closing brace for `{marker}` not found"));
    format!("{}{}{}", &source[..close], fields, &source[close..])
}

fn generate_file<P: AsRef<Path>>(path: P, text: &[u8]) {
    let mut f = File::create(path).unwrap();
    f.write_all(text).unwrap()
}

fn check_safe_repo() -> Result<(), String> {
    // Get the current working directory and navigate up two levels
    let mut repo_path = env::current_dir().map_err(|e| e.to_string())?;
    repo_path.pop(); // Go up one level
    repo_path.pop(); // Go up another level
    let repo_path_str = repo_path
        .to_str()
        .ok_or("Failed to convert repo path to string")?;

    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8(output.stderr).map_err(|e| e.to_string())?;
    if stderr.contains("detected dubious ownership") {
        Command::new("git")
            .args([
                "config",
                "--global",
                "--add",
                "safe.directory",
                repo_path_str,
            ])
            .output()
            .map_err(|e| e.to_string())?;

        let output_retry = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .output()
            .map_err(|e| e.to_string())?;
        if output_retry.status.success() {
            return Ok(());
        } else {
            let err_str = format!(
                "After retrying the git command, the error persisted: {}",
                String::from_utf8(output_retry.stderr)
                    .unwrap_or_else(|_| "Unknown error".to_string())
            );
            return Err(err_str);
        }
    }

    Err(stderr)
}

fn set_godot_explorer_version() {
    // Re-run when the git HEAD/branch ref/index changes so the embedded commit
    // hash stays in sync with the current checkout.
    println!("cargo:rerun-if-changed=../.git/HEAD");
    println!("cargo:rerun-if-changed=../.git/refs/heads");
    println!("cargo:rerun-if-changed=../.git/index");
    println!("cargo:rerun-if-env-changed=BRANCH_NAME");
    println!("cargo:rerun-if-env-changed=DECENTRALAND_PROD_BUILD");
    println!("cargo:rerun-if-env-changed=DECENTRALAND_STAGING_BUILD");
    println!("cargo:rerun-if-env-changed=DCL_BUILD_NUMBER");

    // Always use git to get the actual checked-out commit (what GitHub checkout uses)
    let commit_hash = match check_safe_repo() {
        Ok(_) => {
            if let Ok(output) = Command::new("git")
                .args(["log", "-1", "--format=%H"])
                .output()
            {
                let long_hash = String::from_utf8(output.stdout).unwrap().trim().to_string();
                println!(
                    "cargo:warning=Using commit hash: {} (from git log)",
                    long_hash.chars().take(7).collect::<String>()
                );
                Some(long_hash)
            } else {
                println!("cargo:warning=After checking if the repo is safe, couldn't get the hash");
                None
            }
        }
        Err(e) => {
            println!("cargo:warning=Check if the repo is safe: {}", e);
            None
        }
    };

    if commit_hash.is_none() {
        println!("cargo:warning=No commit hash available, using timestamp");
    }

    // Get short hash (first 7 characters)
    let short_hash = commit_hash
        .as_ref()
        .map(|hash| hash.chars().take(7).collect::<String>());

    // Get the CARGO_PKG_VERSION env var
    let version = env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".to_string());

    // Store build number (Android versionCode / iOS CFBundleVersion), allocated per commit by the
    // Cloudflare Worker and exported via DCL_BUILD_NUMBER. Woven in as a 4th version segment
    // (`{version}.{build}`) so the baked string carries the exact store build (Sentry release, UI,
    // logs). Unset on local/fork builds -> the segment is omitted.
    //
    // NOTE: in CI the `compute-build-number` step MUST run BEFORE the lib build so this env is set
    // when build.rs runs (see android_builds.yml / ios_r2_artifact.yml). The build-number axis is
    // separate from the marketing SemVer and version_gate parses only major.minor.patch, so the
    // extra `.{build}` segment is ignored by the force-update gate.
    let build_segment = env::var("DCL_BUILD_NUMBER")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty() && v.chars().all(|c| c.is_ascii_digit()))
        .map(|v| format!(".{v}"))
        .unwrap_or_default();

    // Check if this is a production or staging build
    let is_prod_build = env::var("DECENTRALAND_PROD_BUILD").is_ok();
    let is_staging_build = env::var("DECENTRALAND_STAGING_BUILD").is_ok();

    // Check if debug or release build
    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    let is_debug = profile == "debug";

    // Determine environment suffix (dev, staging, or prod)
    let env_suffix = if is_prod_build {
        "prod"
    } else if is_staging_build {
        "staging"
    } else {
        "dev"
    };

    // Determine build mode suffix (debug for debug builds, none for release)
    let mode_suffix = if is_debug { "-debug" } else { "" };

    let full_version = match short_hash {
        // With git hash: {version}{.build_number}-{short_hash}{-debug}-{dev|staging|prod}
        Some(hash) => format!(
            "{}{}-{}{}-{}",
            version, build_segment, hash, mode_suffix, env_suffix
        ),
        // Fallback if no git hash available
        _ => {
            let timestamp = Utc::now()
                .to_rfc3339()
                .replace(|c: char| !c.is_ascii_digit(), "");
            format!(
                "{}{}-t{}{}-{}",
                version, build_segment, timestamp, mode_suffix, env_suffix
            )
        }
    };

    println!("cargo:rustc-env=GODOT_EXPLORER_VERSION={}", full_version);

    // Sentry-friendly semver release: `{major.minor.patch}+{build}`.
    //
    // Sentry only exposes `release.version` / `release.build` (and adoption,
    // regression detection, "resolved in next release") when the release parses
    // as clean semver `pkg@major.minor.patch(+build)`. The full `GODOT_EXPLORER_VERSION`
    // above does NOT: it trails the git hash + `-{env}` into the semver prerelease
    // slot and puts the build in a 4th dotted segment, so `release.build` stays empty.
    //
    // Here the build number goes into the numeric `+build` metadata slot (build number
    // is globally monotonic per commit, so `release.build:>=N` == "this build or newer").
    // The commit hash and environment are intentionally dropped — they're already carried
    // by Sentry `dist` and `environment` respectively (see project_main_loop.gd init).
    // When no build number is allocated (local/fork builds) the bare `{version}` is still
    // valid semver.
    let sentry_release = match build_segment.strip_prefix('.') {
        Some(build) if !build.is_empty() => format!("{}+{}", version, build),
        _ => version.clone(),
    };
    println!(
        "cargo:rustc-env=GODOT_EXPLORER_SENTRY_RELEASE={}",
        sentry_release
    );

    // Get full commit hash for Sentry tags
    let full_commit_hash = commit_hash.clone().unwrap_or_default();

    // Get commit message (first 30 characters) for Sentry tags
    let commit_message = if commit_hash.is_some() {
        if let Ok(output) = Command::new("git")
            .args(["log", "-1", "--format=%s"])
            .output()
        {
            let msg = String::from_utf8(output.stdout)
                .unwrap_or_default()
                .trim()
                .to_string();
            // Take first 30 characters
            msg.chars().take(30).collect::<String>()
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    // Get branch name from environment variable (set by CI) or from git
    let branch_name = env::var("BRANCH_NAME").unwrap_or_else(|_| {
        // Fallback: try to get branch name from git
        if let Ok(output) = Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .output()
        {
            String::from_utf8(output.stdout)
                .unwrap_or_default()
                .trim()
                .to_string()
        } else {
            String::new()
        }
    });

    println!(
        "cargo:rustc-env=GODOT_EXPLORER_COMMIT_HASH={}",
        full_commit_hash
    );
    println!(
        "cargo:rustc-env=GODOT_EXPLORER_COMMIT_MESSAGE={}",
        commit_message
    );
    println!("cargo:rustc-env=GODOT_EXPLORER_BRANCH_NAME={}", branch_name);

    // Write checkpoint file for version verification
    let checkpoint_path = Path::new("../.build.version");
    if let Err(e) = fs::write(checkpoint_path, &full_version) {
        println!(
            "cargo:warning=Failed to write version checkpoint file: {}",
            e
        );
    } else {
        println!("cargo:warning=Version checkpoint written: {}", full_version);
    }
}
