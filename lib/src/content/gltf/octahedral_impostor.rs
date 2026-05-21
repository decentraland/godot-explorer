//! Octahedral (8-angle Y-sweep) impostor bake for small props.
//!
//! Pipeline overview:
//! 1. **Filter** (`register_candidate`): pick small low-poly props
//!    that don't have a baked LOD chain (the LOD-baked meshes are
//!    already efficient at distance — putting a billboard quad on
//!    top would just add a draw call).
//! 2. **Spawn impostor** (placeholder atlas): attach a `QuadMesh`
//!    child to the source `MeshInstance3D` with a `ShaderMaterial`
//!    pointing at `octahedral_impostor.gdshader`. The atlas starts
//!    as a 1×1 magenta sentinel so we can see "bake never ran".
//!    Set matched `VisibilityRange`: original mesh visible 0–N m,
//!    impostor visible N–∞ m, with cross-fade.
//! 3. **Bake atlas** (main-thread drain): for every queued job,
//!    build a `SubViewport` sized
//!    `N_ANGLES × CELL_PX` wide by `CELL_PX` tall, drop 8 copies of
//!    the source mesh along the X axis (each rotated by
//!    `i * τ/8` around Y), set one orthographic camera centered on
//!    the strip, render once, copy the viewport texture into the
//!    `atlas` shader uniform. Result: one render produces all 8
//!    views simultaneously.
//! 4. **Runtime** (`octahedral_impostor.gdshader`): the shader
//!    Y-billboards the quad, computes the horizontal angle from
//!    instance to camera in the instance's local frame, picks the
//!    matching cell index, and samples the atlas there.
//!
//! Why Y-axis sweep and not full octahedral (N×N): DCL scenes are
//! ground-level, so the camera pitch range is tight — 8 yaw samples
//! is enough for parallax to read right. A full N×N grid would
//! quadruple atlas memory for marginal benefit on small props.

use std::sync::mpsc;
use std::sync::Mutex;
use std::time::Duration;

use godot::classes::base_material_3d::Transparency;
use godot::classes::camera_3d::{KeepAspect, ProjectionType};
use godot::classes::geometry_instance_3d::VisibilityRangeFadeMode;
use godot::classes::sub_viewport::{ClearMode, UpdateMode};
use godot::classes::{
    ArrayMesh, BaseMaterial3D, Camera3D, Engine, Image, ImageTexture, MeshInstance3D, QuadMesh,
    RenderingServer, SceneTree, Shader, ShaderMaterial, SubViewport,
};
use godot::obj::NewAlloc;
use godot::prelude::*;
use once_cell::sync::Lazy;

const MIN_AABB_DIAG_M: f32 = 0.5;
const MAX_AABB_DIAG_M: f32 = 2.5;
const IMPOSTOR_SWITCH_DISTANCE_M: f32 = 15.0;
const FADE_MARGIN_M: f32 = 3.0;

/// Number of viewing angles baked around the Y axis. The atlas is a
/// horizontal strip of `N_ANGLES` cells.
const N_ANGLES: i32 = 8;
/// Pixel size of each cell. 64 keeps the strip at 512×64 — 128 KB
/// RGBA8 per mesh, low enough to ship per-prop atlases without
/// blowing texture memory.
const CELL_PX: i32 = 64;
/// Atlas width in pixels.
const ATLAS_W: i32 = N_ANGLES * CELL_PX;
/// Padding multiplier on the per-cell ortho size so silhouettes don't
/// clip at cell edges.
const CELL_SIZE_PADDING: f32 = 1.15;
/// Shader source for the impostor `ShaderMaterial`. Embedded as a
/// string instead of loaded from `res://` so it ships in the lib
/// without depending on a re-exported `.pck`. Mirrors
/// `godot/assets/shaders/octahedral_impostor.gdshader`.
const SHADER_SOURCE: &str = r#"shader_type spatial;
render_mode unshaded, cull_disabled, depth_prepass_alpha;

uniform sampler2D atlas : source_color, filter_linear, repeat_disable;
uniform int n_angles = 8;

varying flat int v_cell;
varying vec2 v_uv;

void vertex() {
	vec3 inst_pos_world = MODEL_MATRIX[3].xyz;
	vec3 cam_pos_world = INV_VIEW_MATRIX[3].xyz;
	vec3 to_cam_world = cam_pos_world - inst_pos_world;

	mat3 model_rot = mat3(
		normalize(MODEL_MATRIX[0].xyz),
		normalize(MODEL_MATRIX[1].xyz),
		normalize(MODEL_MATRIX[2].xyz)
	);
	vec3 to_cam_local = transpose(model_rot) * to_cam_world;

	float angle = atan(to_cam_local.x, to_cam_local.z);
	float t = angle / 6.28318530717959 + 0.5;
	v_cell = int(floor(t * float(n_angles) + 0.5)) % n_angles;
	v_uv = UV;

	vec3 forward = vec3(to_cam_world.x, 0.0, to_cam_world.z);
	if (length(forward) < 0.0001) {
		forward = vec3(0.0, 0.0, 1.0);
	} else {
		forward = normalize(forward);
	}
	vec3 up = vec3(0.0, 1.0, 0.0);
	vec3 right = normalize(cross(up, forward));

	vec3 local = VERTEX;
	VERTEX = right * local.x + up * local.y;
	NORMAL = -forward;
}

void fragment() {
	float u = (float(v_cell) + v_uv.x) / float(n_angles);
	vec4 c = texture(atlas, vec2(u, v_uv.y));
	if (c.a < 0.1) {
		discard;
	}
	ALBEDO = c.rgb;
	ALPHA = c.a;
	ALPHA_SCISSOR_THRESHOLD = 0.5;
}
"#;

pub struct ImpostorJob {
    mesh: Gd<ArrayMesh>,
    material: Gd<ShaderMaterial>,
}

/// Worker-thread side request: the job batch + a sync responder the
/// main-thread drain fires when the atlas has been swapped into each
/// job's material.
struct BakeRequest {
    jobs: Vec<ImpostorJob>,
    responder: mpsc::SyncSender<u32>,
}

// SAFETY: the `Gd<T>`s are parked under the mutex until the main thread
// (the only place they're touched) drains the queue.
unsafe impl Send for BakeRequest {}

static BAKE_QUEUE: Lazy<Mutex<Vec<BakeRequest>>> = Lazy::new(|| Mutex::new(Vec::new()));

/// SubViewports + materials waiting on a render that hasn't completed
/// yet. The drain advances `frames_remaining` and reads textures when
/// the count reaches zero.
struct InFlightSlot {
    subviewport: Gd<SubViewport>,
    material: Gd<ShaderMaterial>,
}

struct InFlightRequest {
    slots: Vec<InFlightSlot>,
    responder: mpsc::SyncSender<u32>,
    frames_remaining: u32,
}

unsafe impl Send for InFlightRequest {}

static IN_FLIGHT: Lazy<Mutex<Vec<InFlightRequest>>> = Lazy::new(|| Mutex::new(Vec::new()));

/// Frames the bake waits between SubViewport setup and texture read.
/// Matches the avatar impostor capturer (`AvatarPreview.async_get_viewport_image`
/// in GDScript awaits three `process_frame`s before reading the
/// rendered texture).
const FRAMES_TO_WAIT: u32 = 3;

/// Bake timeout — generous because the queue may have many requests
/// stacking up behind earlier ones. 30 s covers ~1800 frames at 60 Hz.
const BAKE_TIMEOUT: Duration = Duration::from_secs(30);

/// Filter the MI and, on success, attach the impostor billboard +
/// VisibilityRange swap with a placeholder atlas. Returns a job the
/// batch bake uses to swap the placeholder atlas to a real one once
/// the SubViewport render completes.
pub fn register_candidate(
    mi: &mut Gd<MeshInstance3D>,
    mesh: &Gd<ArrayMesh>,
) -> Option<ImpostorJob> {
    if mi.has_meta("dcl_impostor_attached") {
        return None;
    }
    if !passes_filters(mi, mesh) {
        return None;
    }
    let shader = load_shader()?;

    let aabb = mesh.get_aabb();
    let size = aabb.size;
    let center_local = aabb.position + size * 0.5;
    let quad_w = size.x.max(size.z).max(0.1);
    let quad_h = size.y.max(0.1);

    let mut mat = ShaderMaterial::new_gd();
    mat.set_shader(&shader);
    let placeholder_atlas: Gd<godot::classes::Texture2D> = make_placeholder_atlas().upcast();
    mat.set_shader_parameter("atlas", &placeholder_atlas.to_variant());
    mat.set_shader_parameter("n_angles", &(N_ANGLES as i64).to_variant());

    let mut quad = QuadMesh::new_gd();
    quad.set_size(Vector2::new(quad_w, quad_h));

    let mut impostor_mi = MeshInstance3D::new_alloc();
    impostor_mi.set_name("dcl_impostor");
    impostor_mi.set_mesh(&quad.upcast::<godot::classes::Mesh>());
    impostor_mi.set_surface_override_material(0, &mat.clone().upcast::<godot::classes::Material>());
    impostor_mi.set_position(center_local);
    impostor_mi.set_cast_shadows_setting(
        godot::classes::geometry_instance_3d::ShadowCastingSetting::OFF,
    );
    impostor_mi.set_visibility_range_begin(IMPOSTOR_SWITCH_DISTANCE_M);
    impostor_mi.set_visibility_range_begin_margin(FADE_MARGIN_M);
    impostor_mi.set_visibility_range_fade_mode(VisibilityRangeFadeMode::SELF);

    mi.add_child(&impostor_mi.upcast::<godot::classes::Node>());

    mi.set_visibility_range_end(IMPOSTOR_SWITCH_DISTANCE_M);
    mi.set_visibility_range_end_margin(FADE_MARGIN_M);
    mi.set_visibility_range_fade_mode(VisibilityRangeFadeMode::SELF);

    mi.set_meta("dcl_impostor_attached", &true.to_variant());

    Some(ImpostorJob {
        mesh: mesh.clone(),
        material: mat,
    })
}

/// Worker-thread entry: park the jobs in the queue + block on the
/// responder. Returns the number of atlases successfully baked.
pub fn enqueue_and_wait(jobs: Vec<ImpostorJob>) -> u32 {
    if jobs.is_empty() {
        return 0;
    }
    let (tx, rx) = mpsc::sync_channel::<u32>(1);
    let req = BakeRequest {
        jobs,
        responder: tx,
    };
    if let Ok(mut q) = BAKE_QUEUE.lock() {
        q.push(req);
    } else {
        return 0;
    }
    rx.recv_timeout(BAKE_TIMEOUT).unwrap_or(0)
}

/// Main-thread entry: advance the bake pipeline by one frame.
///
/// Stage 1: any in-flight request whose `frames_remaining` hits zero
/// has its SubViewport texture read into an `ImageTexture` and the
/// `atlas` uniform on each job's `ShaderMaterial` updated. Survivors
/// keep counting down.
///
/// Stage 2: new requests from `BAKE_QUEUE` get their SubViewports
/// built + parented under the given `parent` node + queued as in-flight.
pub fn drain_bake_queue_on_main(parent: &mut Gd<godot::classes::Node>) {
    // Stage 1.
    let in_flight: Vec<InFlightRequest> = match IN_FLIGHT.lock() {
        Ok(mut q) => std::mem::take(&mut *q),
        Err(_) => return,
    };
    let mut survivors: Vec<InFlightRequest> = Vec::with_capacity(in_flight.len());
    for mut req in in_flight {
        req.frames_remaining = req.frames_remaining.saturating_sub(1);
        if req.frames_remaining > 0 {
            survivors.push(req);
            continue;
        }
        let mut baked = 0u32;
        for slot in req.slots.iter() {
            if let Some(tex) = read_baked_texture(&slot.subviewport) {
                let tex2d: Gd<godot::classes::Texture2D> = tex.upcast();
                let mut mat = slot.material.clone();
                mat.set_shader_parameter("atlas", &tex2d.to_variant());
                baked += 1;
            }
        }
        let _ = req.responder.send(baked);
        for slot in req.slots {
            let mut sv_as_node: Gd<godot::classes::Node> = slot.subviewport.upcast();
            parent.remove_child(&sv_as_node);
            sv_as_node.queue_free();
        }
    }
    if let Ok(mut q) = IN_FLIGHT.lock() {
        q.extend(survivors);
    }

    // Stage 2. Cap how many requests we start per tick to keep the
    // SubViewport-resource backlog bounded. With 1000+ scenes the
    // worker threads can stack up hundreds of pending bakes; trying
    // to materialize them all in one tick leaks resources faster
    // than `queue_free` reclaims them and the renderer eventually
    // segfaults. One-at-a-time pacing keeps total in-flight slots
    // ≤ `MAX_IN_FLIGHT_REQUESTS × FRAMES_TO_WAIT`.
    const MAX_NEW_PER_TICK: usize = 1;
    const MAX_IN_FLIGHT_REQUESTS: usize = 8;
    let in_flight_count = IN_FLIGHT.lock().map(|q| q.len()).unwrap_or(0);
    let slots_left = MAX_IN_FLIGHT_REQUESTS.saturating_sub(in_flight_count);
    let take_n = MAX_NEW_PER_TICK.min(slots_left);
    if take_n == 0 {
        return;
    }
    let new_reqs: Vec<BakeRequest> = match BAKE_QUEUE.lock() {
        Ok(mut q) => {
            let n = take_n.min(q.len());
            q.drain(..n).collect()
        }
        Err(_) => return,
    };
    if new_reqs.is_empty() {
        return;
    }
    let mut new_in_flight = Vec::with_capacity(new_reqs.len());
    for req in new_reqs {
        let mut slots = Vec::with_capacity(req.jobs.len());
        for job in req.jobs {
            let Some(sv) = setup_subviewport(&job.mesh) else {
                continue;
            };
            parent.add_child(&sv.clone().upcast::<godot::classes::Node>());
            slots.push(InFlightSlot {
                subviewport: sv,
                material: job.material,
            });
        }
        new_in_flight.push(InFlightRequest {
            slots,
            responder: req.responder,
            frames_remaining: FRAMES_TO_WAIT,
        });
    }
    if let Ok(mut q) = IN_FLIGHT.lock() {
        q.extend(new_in_flight);
    }

    // Nudge the renderer so newly-parented SubViewports rasterize
    // their first frame this tick. UPDATE_ALWAYS would eventually
    // catch them, but `force_draw` avoids a wasted tick.
    RenderingServer::singleton().force_draw();
}

/// Build a SubViewport that hosts `N_ANGLES` copies of the source
/// mesh along the X axis, each rotated by `i * τ/N` around Y, framed
/// by one orthographic camera. The resulting render is the atlas.
fn setup_subviewport(mesh: &Gd<ArrayMesh>) -> Option<Gd<SubViewport>> {
    let aabb = mesh.get_aabb();
    let size = aabb.size;
    let center = aabb.position + size * 0.5;
    // Cell world size = bounding sphere radius so the rotated mesh
    // always fits inside its cell regardless of yaw. Using the AABB
    // diagonal as a safe upper bound — the mesh's silhouette under
    // any Y rotation can be at most diag/2 from the center on the
    // horizontal plane.
    let half_diag_h = (size.x * size.x + size.z * size.z).sqrt() * 0.5;
    let half_height = size.y * 0.5;
    let cell_half = half_diag_h.max(half_height) * CELL_SIZE_PADDING;
    if cell_half < 0.05 {
        return None;
    }
    let cell_world = cell_half * 2.0;

    let mut subviewport = SubViewport::new_alloc();
    subviewport.set_size(Vector2i::new(ATLAS_W, CELL_PX));
    subviewport.set_transparent_background(true);
    subviewport.set_update_mode(UpdateMode::ALWAYS);
    subviewport.set_clear_mode(ClearMode::ALWAYS);
    subviewport.set_disable_3d(false);
    if let Some(main_loop) = Engine::singleton().get_main_loop() {
        if let Ok(tree) = main_loop.try_cast::<SceneTree>() {
            if let Some(root) = tree.get_root() {
                if let Some(world) = root.get_world_3d() {
                    subviewport.set_world_3d(&world);
                }
            }
        }
    }
    // UNSHADED forces every surface to render its raw albedo (texture
    // + color) without lighting — exactly what we want in the atlas.
    subviewport.set_debug_draw(godot::classes::viewport::DebugDraw::UNSHADED);

    // Lay 8 copies of the mesh along X. Each MI's local origin is
    // shifted by `-center` so the mesh's AABB center sits at its
    // slot position. Each MI is rotated by `i * τ/N` around Y; the
    // runtime shader inverts that rotation to pick the matching cell.
    let strip_left_x = -((N_ANGLES as f32 - 1.0) * 0.5) * cell_world;
    let n = N_ANGLES;
    for i in 0..n {
        let slot_x = strip_left_x + (i as f32) * cell_world;
        let mut temp_mi = MeshInstance3D::new_alloc();
        temp_mi.set_mesh(&mesh.clone().upcast::<godot::classes::Mesh>());
        // Rotation first (around the mesh's own AABB center), then
        // translation to the slot. Use a Transform3D so rotation is
        // applied in mesh-local space and the AABB center ends up at
        // (slot_x, 0, 0) in world.
        let yaw = (i as f32) * std::f32::consts::TAU / (N_ANGLES as f32);
        let basis = Basis::from_euler(EulerOrder::XYZ, Vector3::new(0.0, yaw, 0.0));
        let translation = Vector3::new(slot_x, 0.0, 0.0) - basis * center;
        let tf = Transform3D {
            basis,
            origin: translation,
        };
        temp_mi.set_transform(tf);
        subviewport.add_child(&temp_mi.upcast::<godot::classes::Node>());
    }

    // Camera: orthographic, looking down -Z at the strip from the
    // +Z direction. `KEEP_HEIGHT` means `set_size` controls the
    // viewport's *vertical* extent — width is implied by the
    // viewport's aspect ratio (here 8:1 so the width = 8 × size).
    let strip_center_x = 0.0; // strip is centered around X=0
    let cam_distance = cell_world * 4.0;
    let mut camera = Camera3D::new_alloc();
    camera.set_projection(ProjectionType::ORTHOGONAL);
    camera.set_keep_aspect_mode(KeepAspect::HEIGHT);
    camera.set_size(cell_world);
    camera.set_near(0.01);
    camera.set_far(cam_distance * 4.0);
    let cam_pos = Vector3::new(strip_center_x, 0.0, cam_distance);
    let cam_target = Vector3::new(strip_center_x, 0.0, 0.0);
    camera.look_at_from_position(cam_pos, cam_target);
    camera.set_current(true);
    subviewport.add_child(&camera.upcast::<godot::classes::Node>());

    Some(subviewport)
}

fn read_baked_texture(subviewport: &Gd<SubViewport>) -> Option<Gd<ImageTexture>> {
    let tex = subviewport.get_texture()?;
    let image = tex.get_image()?;
    let w = image.get_width();
    let h = image.get_height();
    let blank = image_is_blank(&image);
    static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    if n < 8 {
        let p = format!("/app/output/_impostor_dbg_{}.png", n);
        let path = GString::from(p.as_str());
        let err = image.save_png(&path);
        godot::global::godot_print!(
            "[impostor-bake] dbg#{} w={} h={} blank={} save_err={:?}",
            n,
            w,
            h,
            blank,
            err
        );
    }
    if w == 0 || h == 0 || blank {
        return None;
    }
    ImageTexture::create_from_image(&image)
}

/// Reject only if not a single sample has visible alpha. Stride 256
/// is dense enough on a 1024×128 atlas to catch any visible cell
/// (each cell is 128×128 = 16384 alpha bytes, so even one cell with
/// content fires).
fn image_is_blank(image: &Gd<Image>) -> bool {
    let w = image.get_width();
    let h = image.get_height();
    if w == 0 || h == 0 {
        return true;
    }
    let data = image.get_data();
    let len = data.len();
    let mut i = 3usize;
    let stride = 16usize;
    while i < len {
        if let Some(a) = data.get(i) {
            if a > 12 {
                return false;
            }
        }
        i += stride;
    }
    true
}

fn passes_filters(mi: &Gd<MeshInstance3D>, mesh: &Gd<ArrayMesh>) -> bool {
    if mi.get_visibility_range_end() > 0.0 {
        return false;
    }
    let aabb = mesh.get_aabb();
    let size = aabb.size;
    let diag = (size.x * size.x + size.y * size.y + size.z * size.z).sqrt();
    if !(MIN_AABB_DIAG_M..=MAX_AABB_DIAG_M).contains(&diag) {
        return false;
    }
    let min_axis = size.x.min(size.y).min(size.z);
    if min_axis < 0.2 {
        return false;
    }
    if mesh.get_blend_shape_count() > 0 {
        return false;
    }
    // Skip skinned meshes. Re-running the bake on a skinned mesh
    // would need the skeleton, which we don't have here.
    let surface_count = mesh.get_surface_count();
    for s in 0..surface_count {
        let arrays = mesh.surface_get_arrays(s);
        let bones_idx = godot::classes::mesh::ArrayType::BONES.ord() as usize;
        if arrays.len() <= bones_idx {
            continue;
        }
        if let Ok(bi) = arrays.at(bones_idx).try_to::<PackedInt32Array>() {
            if !bi.is_empty() {
                return false;
            }
        }
    }
    // Note: the size-based filter above already targets the right
    // window (small props 0.5–2.5 m diag). We deliberately do NOT
    // filter by index count — meshes that have LODs baked still
    // benefit from the impostor swap because the LOD3 silhouette is
    // never as cheap as a single billboard quad. The runtime LOD
    // chain handles 0–15 m, the impostor takes over past 15 m.
    if let Some(mat) = mi.get_active_material(0) {
        if let Ok(base) = mat.try_cast::<BaseMaterial3D>() {
            if base.get_transparency() != Transparency::DISABLED {
                return false;
            }
        }
    }
    true
}

/// Lazy-init holder for the impostor shader so every impostor MI
/// shares the same `Shader` resource. Without this, each
/// `register_candidate` calls `Shader::new_gd() + set_code()` and
/// hundreds of duplicate Shader resources blow up Godot's pipeline
/// cache (suspected cause of the signal-11 crash after ~17 impostor
/// bakes on Vulkan).
struct SharedShader(Option<Gd<Shader>>);

// SAFETY: `Gd<T>` is not Send, but this static is parked under a
// Mutex and only touched from the worker thread that ran into
// `register_candidate`. Same pattern as the bake-queue Send impl.
unsafe impl Send for SharedShader {}

static SHARED_SHADER: Lazy<Mutex<SharedShader>> = Lazy::new(|| Mutex::new(SharedShader(None)));

fn load_shader() -> Option<Gd<Shader>> {
    let mut guard = SHARED_SHADER.lock().ok()?;
    if guard.0.is_none() {
        let mut s = Shader::new_gd();
        s.set_code(SHADER_SOURCE);
        guard.0 = Some(s);
    }
    guard.0.clone()
}

/// 1×1 magenta texture used as the `atlas` uniform until the bake
/// finishes. Makes "bake never completed" visible at runtime instead
/// of showing an invisible / black quad.
fn make_placeholder_atlas() -> Gd<ImageTexture> {
    let img = Image::create_from_data(
        1,
        1,
        false,
        godot::classes::image::Format::RGBA8,
        &PackedByteArray::from(&[255u8, 0, 255, 255][..]),
    )
    .expect("create 1x1 image");
    ImageTexture::create_from_image(&img).expect("create image texture")
}

/// Octahedral direction → 2D UV mapping. Currently unused (the
/// 8-angle Y-sweep doesn't need it) — kept for the eventual N×N
/// full-octahedral bake.
#[allow(dead_code)]
pub fn octahedral_dir_to_uv(dir: Vector3) -> Vector2 {
    let abs_sum = dir.x.abs() + dir.y.abs() + dir.z.abs();
    if abs_sum < 1e-6 {
        return Vector2::new(0.5, 0.5);
    }
    let inv = 1.0 / abs_sum;
    let mut nx = dir.x * inv;
    let mut nz = dir.z * inv;
    if dir.y < 0.0 {
        let ox = nx;
        let oz = nz;
        nx = (1.0 - oz.abs()).copysign(ox);
        nz = (1.0 - ox.abs()).copysign(oz);
    }
    Vector2::new(nx * 0.5 + 0.5, nz * 0.5 + 0.5)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn octahedral_uv_canonical_directions() {
        let up = octahedral_dir_to_uv(Vector3::new(0.0, 1.0, 0.0));
        assert!((up.x - 0.5).abs() < 1e-5);
        assert!((up.y - 0.5).abs() < 1e-5);
        let dn = octahedral_dir_to_uv(Vector3::new(0.0, -1.0, 0.0));
        assert!((dn.x - 0.5).abs() < 1e-5);
        assert!((dn.y - 0.5).abs() < 1e-5);
    }
}
