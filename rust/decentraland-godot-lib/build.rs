use std::{
    env,
    fs::{self, File},
    io::{self, Write},
    path::Path,
};

struct Component {
    id: u32,
    pascal_name: String,
    snake_name: String,
}

const PROTO_FILES_BASE_DIR: &str = "src/dcl/components/proto/";
const COMPONENT_BASE_DIR: &str = "src/dcl/components/proto/decentraland/sdk/components/";
const GROW_ONLY_SET_COMPONENTS: [&str; 3] =
    ["PointerEventsResult", "VideoEvent", "AvatarEmoteCommand"];

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
    let output_str = format!("impl SceneComponentId {{ {output_str} }}");
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
impl SceneCrdtStateProtoComponents {{
{custom_proto_methods}
}}
"
    );
    generate_file(dest_path, output_str.as_bytes());
}

fn main() -> io::Result<()> {
    let mut proto_components = vec![];
    let mut proto_files = vec![];
    let dir_path = Path::new(COMPONENT_BASE_DIR);
    for entry in fs::read_dir(dir_path)
        .expect("Failed to read directory")
        .flatten()
    {
        if let Some(extension) = entry.path().extension() {
            if extension == "proto" {
                proto_files.push(entry.path());

                proto_components.push(get_component_id_and_name(entry.path().to_str().unwrap()));
            }
        }
    }

    proto_files.push(
        format!("{PROTO_FILES_BASE_DIR}decentraland/kernel/comms/rfc5/ws_comms.proto").into(),
    );
    proto_files
        .push(format!("{PROTO_FILES_BASE_DIR}decentraland/kernel/comms/rfc4/comms.proto").into());
    proto_files.push(
        format!("{PROTO_FILES_BASE_DIR}decentraland/kernel/comms/v3/archipelago.proto").into(),
    );

    generate_enum(&proto_components);
    generate_impl_crdt(&proto_components);
    generate_dcl_component_impl(&proto_components);

    let mut protoc_path = std::env::current_dir()
        .unwrap()
        .join("../../.bin/protoc/bin/protoc");
    if std::env::consts::OS == "windows" {
        protoc_path.set_extension("exe");
    }
    let protoc_path = protoc_path
        .canonicalize()
        .expect("Failed to canonicalize protoc path");

    std::env::set_var("PROTOC", protoc_path);
    prost_build::compile_protos(&proto_files, &["src/dcl/components/proto/"])?;

    #[cfg(feature = "use_livekit")]
    if env::var("CARGO_CFG_TARGET_OS").unwrap() == "android" {
        webrtc_sys_build::configure_jni_symbols().unwrap();
    }

    for source in proto_files {
        let value = source.to_str().unwrap();
        println!("cargo:rerun-if-changed={value}");
    }

    Ok(())
}

fn generate_file<P: AsRef<Path>>(path: P, text: &[u8]) {
    let mut f = File::create(path).unwrap();
    f.write_all(text).unwrap()
}
