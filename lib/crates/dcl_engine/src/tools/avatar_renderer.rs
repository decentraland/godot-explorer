//! The avatar renderer (`--avatar-renderer --avatars <file>`): renders each avatar of the file to a
//! body PNG and, when asked, a face PNG.

use godot::classes::viewport::Msaa;
use godot::classes::{FileAccess, INode, Image, Json, Node};
use godot::prelude::*;
use serde_json::Value;

use super::offscreen::{self, Quality};
use crate::avatars::avatar::build::update_avatar;
use crate::avatars::avatar_type::DclAvatarWireFormat;
use crate::avatars::preview::AvatarPreview;
use crate::engine::local_task::{await_promise, next_frame, spawn_local};
use crate::fetcher::globals;
use crate::godot_classes::dcl_global::DclGlobal;
use crate::test_runner::testing_tools::ensure_parent_dir;

const TEST_INPUT: &str = "res://../tests/avatars-test-input.json";
/// Manual supersampling on top of the viewport's `scaling_3d_scale`: a 256x512 capture renders at
/// 1024x2048 internally (2 * 2).
const SSAA_FACTOR: i32 = 2;
const BODY_ORTHO_SIZE: f32 = 2.5;

#[derive(Debug, PartialEq)]
struct AvatarSpec {
    entity: String,
    dest_path: String,
    width: i32,
    height: i32,
    face_dest_path: String,
    face_width: i32,
    face_height: i32,
    face_zoom: f32,
    avatar: Value,
}

#[derive(Debug)]
struct AvatarFile {
    base_url: String,
    specs: Vec<AvatarSpec>,
}

fn int(value: Option<&Value>, fallback: i32) -> i32 {
    value.and_then(Value::as_f64).map_or(fallback, |n| n as i32)
}

fn text(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

impl AvatarSpec {
    /// `None` for an entry with no destination: there is nowhere to write it.
    fn parse(entry: &Value) -> Option<Self> {
        let fields = entry.as_object()?;
        let dest_path = fields.get("destPath")?.as_str()?.to_string();
        Some(Self {
            entity: text(fields.get("entity")),
            dest_path,
            width: int(fields.get("width"), 2048),
            height: int(fields.get("height"), 2048),
            face_dest_path: text(fields.get("faceDestPath")),
            face_width: int(fields.get("faceWidth"), 256),
            face_height: int(fields.get("faceHeight"), 256),
            face_zoom: fields
                .get("faceZoom")
                .and_then(Value::as_f64)
                .map_or(0.55, |z| z as f32),
            avatar: fields
                .get("avatar")
                .cloned()
                .unwrap_or(Value::Object(Default::default())),
        })
    }
}

impl AvatarFile {
    fn parse(text: &str) -> Result<Self, String> {
        let value: Value = serde_json::from_str(text)
            .map_err(|_| "the file has to be a valid json dictionary".to_string())?;
        let file = value
            .as_object()
            .ok_or("the file has to be a valid json dictionary")?;
        let (Some(base_url), Some(payload)) = (file.get("baseUrl"), file.get("payload")) else {
            return Err(
                "baseUrl and payload property has to be included in the the file dictionary"
                    .to_string(),
            );
        };
        let payload = payload.as_array().ok_or("payload has to be an array")?;
        Ok(Self {
            base_url: base_url.as_str().unwrap_or_default().to_string(),
            specs: payload.iter().filter_map(AvatarSpec::parse).collect(),
        })
    }
}

fn read_input() -> Option<AvatarFile> {
    let cli = globals::cli();
    let path = if cli.bind().use_test_input {
        TEST_INPUT.to_string()
    } else {
        cli.bind().avatars_file.to_string()
    };
    if path.is_empty() {
        return None;
    }
    let file = FileAccess::open(path.as_str(), godot::classes::file_access::ModeFlags::READ)?;
    AvatarFile::parse(&file.get_as_text().to_string())
        .inspect_err(|reason| godot_error!("{reason}"))
        .ok()
}

#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct AvatarRenderer {
    base: Base<Node>,
}

#[godot_api]
impl INode for AvatarRenderer {
    fn ready(&mut self) {
        godot_print!("spawning avatar renderer scene");
        let quit = |code: i32| {
            if let Some(mut tree) = DclGlobal::singleton().get_tree_or_null() {
                tree.quit_ex().exit_code(code).done();
            }
        };
        let Some(input) = read_input() else {
            godot_error!("param is missing or wrong, try with --avatars [file]");
            quit(1);
            return;
        };
        if input.specs.is_empty() {
            godot_error!("no avatars to process");
            quit(2);
            return;
        }
        globals::scene_runner().bind_mut().set_pause(true);
        globals::realm().bind_mut().content_base_url = input.base_url.as_str().into();
        let Some(mut preview) = self
            .base()
            .try_get_node_as::<AvatarPreview>("%AvatarPreview")
        else {
            godot_error!("the avatar renderer scene has no preview");
            quit(1);
            return;
        };
        preview.bind_mut().fit_avatar = false;
        // Off-screen capture with no realtime budget: stack every quality lever.
        offscreen::apply(
            &preview,
            Quality {
                msaa: Msaa::MSAA_8X,
                scaling_3d: 2.0,
            },
        );
        spawn_local(render_all(preview, input));
    }
}

async fn render_all(preview: Gd<AvatarPreview>, input: AvatarFile) {
    for (index, spec) in input.specs.iter().enumerate() {
        if spec.entity.is_empty() {
            godot_print!("processing payload {index}");
        } else {
            godot_print!("processing payload entity {}", spec.entity);
        }
        next_frame().await;
        render_one(&preview, spec).await;
        next_frame().await;
    }
    DclGlobal::singleton()
        .bind()
        .testing_tools
        .bind()
        .exit_gracefully(0);
}

async fn render_one(preview: &Gd<AvatarPreview>, spec: &AvatarSpec) {
    let Some(avatar) = preview.bind().avatar.clone() else {
        return;
    };
    let dictionary = Json::parse_string(spec.avatar.to_string().as_str())
        .try_to::<VarDictionary>()
        .unwrap_or_default();
    let wire = DclAvatarWireFormat::from_godot_dictionary(dictionary);
    update_avatar(avatar, wire, String::new(), None).await;
    godot::classes::RenderingServer::singleton()
        .set_default_clear_color(Color::from_rgba(0.0, 0.0, 0.0, 0.0));
    next_frame().await;

    let body = capture(
        preview,
        false,
        Vector2i::new(spec.width, spec.height),
        BODY_ORTHO_SIZE,
    )
    .await;
    save(body, &spec.dest_path);
    if !spec.face_dest_path.is_empty() {
        let size = Vector2i::new(spec.face_width, spec.face_height);
        let face = capture(preview, true, size, spec.face_zoom).await;
        save(face, &spec.face_dest_path);
    }
}

async fn capture(
    preview: &Gd<AvatarPreview>,
    face: bool,
    size: Vector2i,
    ortho_size: f32,
) -> Option<Gd<Image>> {
    let promise =
        preview
            .clone()
            .bind_mut()
            .async_get_viewport_image(face, size, ortho_size, SSAA_FACTOR);
    await_promise(promise).await.try_to::<Gd<Image>>().ok()
}

fn save(image: Option<Gd<Image>>, path: &str) {
    let path = if path.ends_with(".png") {
        path.to_string()
    } else {
        format!("{path}.png")
    };
    let Some(image) = image else {
        godot_error!("🔴 no image for {path}");
        return;
    };
    ensure_parent_dir(&path);
    image.save_png(path.as_str());
    godot_print!("🟢 {path}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries_without_a_destination_are_skipped() {
        let file = AvatarFile::parse(
            r#"{"baseUrl": "u", "payload": [{"destPath": "a"}, {"entity": "x"}, 3]}"#,
        )
        .unwrap();
        assert_eq!(file.specs.len(), 1);
        assert_eq!(file.specs[0].dest_path, "a");
    }

    #[test]
    fn sizes_and_zoom_have_defaults() {
        let spec = AvatarSpec::parse(&serde_json::json!({"destPath": "a"})).unwrap();
        assert_eq!((spec.width, spec.height), (2048, 2048));
        assert_eq!((spec.face_width, spec.face_height), (256, 256));
        assert!((spec.face_zoom - 0.55).abs() < 1e-6);
        assert!(spec.face_dest_path.is_empty());
    }

    #[test]
    fn the_file_needs_a_base_url_and_a_payload_list() {
        assert!(AvatarFile::parse("nope").is_err());
        assert!(AvatarFile::parse(r#"{"payload": []}"#).is_err());
        assert!(AvatarFile::parse(r#"{"baseUrl": "u", "payload": {}}"#).is_err());
    }
}
