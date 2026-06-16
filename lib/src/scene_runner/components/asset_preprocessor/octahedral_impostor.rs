//! Full octahedral impostor bake for small props.
//!
//! Why this exists: the runtime LOD chain handles 0–15 m; past 15 m
//! we want a single billboard quad to replace the prop entirely so
//! the GPU pays one draw call instead of dozens.
//!
//! Octahedral parameterization (Cigolle et al.; impostor pipeline per
//! Ryan Brucks, "Octahedral Impostors", shaderbits.com): a view
//! direction on the unit sphere unfolds into a 2D square. The atlas
//! is an N×N grid of cells; cell (i, j) holds the mesh rendered from
//! the direction whose octahedral UV falls at the cell center. At
//! runtime the shader inverts the mapping (instance→camera direction
//! → octahedral UV) and tri-samples the three nearest cells with
//! barycentric weights so view transitions stay smooth.
//!
//! Pipeline:
//! 1. `register_candidate` (worker thread, holding godot_single_thread):
//!    filters MIs to small opaque props, attaches a child `MeshInstance3D`
//!    with a placeholder atlas + a matched `VisibilityRange` swap on
//!    both source and impostor. Returns an `ImpostorJob`.
//! 2. Caller batches `ImpostorJob`s for a single GLTF and calls
//!    `enqueue_and_wait` which blocks the worker thread until the
//!    main-thread drain has swapped real atlases into every job's
//!    `ShaderMaterial`. `save_node_as_scene` runs AFTER this so the
//!    saved `.scn` captures the baked atlases.
//! 3. `drain_bake_queue_on_main` (main thread, every frame via
//!    `DclAssetServer::process`): builds one `SubViewport` per pending
//!    request, waits N frames, reads the viewport texture, writes
//!    the `atlas` shader uniform, frees the SubViewport, signals the
//!    responder.
//!
//! v2 mitigations against the 17-bake crash that killed the prior
//! (Y-sweep) attempt:
//! - **One bake in flight at a time** — eliminates concurrent
//!   SubViewport/World3D pressure on the RenderingServer.
//! - **Dedicated `World3D` per bake** — frees cleanly with the
//!   SubViewport; no shared-main-world instance leak.
//! - **Synchronous `free()` (not `queue_free()`)** of the SubViewport
//!   so its RenderingServer instances are reclaimed before the next
//!   bake materializes.
//! - **Shared `Shader` resource** across all impostor `ShaderMaterial`s
//!   — hundreds of duplicate `Shader::new_gd()` + `set_code()` calls
//!   was the previously-suspected pipeline-cache blowup.

use std::sync::mpsc;
use std::sync::Mutex;
use std::time::Duration;

use godot::classes::base_material_3d::Transparency;
use godot::classes::camera_3d::{KeepAspect, ProjectionType};
use godot::classes::geometry_instance_3d::VisibilityRangeFadeMode;
use godot::classes::image::CompressMode;
use godot::classes::sub_viewport::{ClearMode, UpdateMode};
use godot::classes::{
    ArrayMesh, BaseMaterial3D, Camera3D, Image, ImageTexture, MeshInstance3D, QuadMesh, Shader,
    ShaderMaterial, SubViewport, Texture2D, World3D,
};
use godot::global::Error;
use godot::obj::NewAlloc;
use godot::prelude::*;
use once_cell::sync::Lazy;

/// Candidate filter. Wider than the original 0.5–2.5 m window (which
/// caught only ~160 GP candidates), tighter than the 0.3–12 m window
/// that started impostoring walls / floor tiles / sky planes
/// (manifested as giant pink billboards). 6 m upper bound keeps
/// structural geometry out — anything bigger than that is a building
/// section the runtime LOD chain handles better than a billboard.
const MIN_AABB_DIAG_M: f32 = 0.25;
const MAX_AABB_DIAG_M: f32 = 6.0;
/// Max distance from the MI's node origin to the mesh AABB center,
/// as a multiple of the diagonal. The source MI's VisibilityRange is
/// anchored at the node origin while the impostor child is anchored at
/// the AABB center; when the visible geometry sits far from the origin
/// the two swap points diverge and you can stand next to the prop yet
/// still be outside the impostor's range → impostor never hands back to
/// the mesh. Base-pivoted props (origin at the foot) offset by ~0.5·diag
/// and stay in; far-flung geometry (node at parcel origin, mesh 20 m
/// away) is rejected.
const MAX_CENTER_OFFSET_DIAG_RATIO: f32 = 1.0;
/// Reject elongated/flat geometry (signs, decals, wall sections,
/// floor tiles) where one axis is way thinner than the others.
/// Looser now that we roll back the impostor on bake failure —
/// the cost of a "bad fit" candidate is a wasted bake attempt,
/// not a permanent magenta placeholder.
const MIN_AXIS_M: f32 = 0.08;
const MIN_AXIS_RATIO: f32 = 0.04;

/// Switch distance is `diag × SWITCH_DISTANCE_PER_DIAG`, floored at
/// `MIN_SWITCH_DISTANCE_M`. Keeping the ratio fixed means the
/// impostor takes over at roughly the same angular size for every
/// prop — small things swap closer to camera, large things further
/// out. A value of 6 ≈ 9.5° subtended angle at the swap point.
/// Aggressive on purpose: the runtime LOD chain already covers the
/// near range, so the impostor's job is to kill far-field draw cost
/// as early as the silhouette can plausibly read.
const SWITCH_DISTANCE_PER_DIAG: f32 = 6.0;
/// Floor so 0.5 m props don't try to swap inside the camera's near
/// plane. With `SWITCH_DISTANCE_PER_DIAG=6.0` this only binds for
/// diags under ~0.5 m, which the size filter already excludes.
const MIN_SWITCH_DISTANCE_M: f32 = 3.0;
/// Cross-fade band as a fraction of the swap distance. 0.25 gives a
/// fade band of 0.75–3.75 m across the candidate-size range —
/// noticeable on big props, snappy on small ones.
const FADE_FRACTION: f32 = 0.25;
/// Hard floor on the fade band so the smallest props still get a
/// short cross-fade instead of an instantaneous pop.
const MIN_FADE_MARGIN_M: f32 = 0.6;

fn switch_distance_for(diag: f32) -> f32 {
    (diag * SWITCH_DISTANCE_PER_DIAG).max(MIN_SWITCH_DISTANCE_M)
}

fn fade_margin_for(switch_distance: f32) -> f32 {
    (switch_distance * FADE_FRACTION).max(MIN_FADE_MARGIN_M)
}

/// Cells per atlas axis. 8 → 64 baked view directions. The shader
/// tri-blends three neighbors so apparent directional resolution
/// exceeds the raw cell count. Cell grid points cover N values from
/// uv=0 to uv=1, spaced by 1/(N-1) (Brucks convention).
const N_CELLS: i32 = 8;
/// Pixel size of one cell. 32 → 256×256 atlas, ETC2-compressed
/// ~85 KB per impostor with mipmaps. The atlas ships as a plain
/// ETC2 `ImageTexture` embedded in the `.scn` — the same form the
/// avatar-mesh textures use, which is the only one that survives the
/// `save_node_as_scene` → device-reload roundtrip without turning
/// magenta on Mali (see `stitch_atlas_texture`).
const CELL_PX: i32 = 32;
const ATLAS_DIM: i32 = N_CELLS * CELL_PX;
/// Packed atlas height: 2× `ATLAS_DIM`. Albedo occupies rows
/// `[0, ATLAS_DIM)` (top half), normal+ORM occupies `[ATLAS_DIM,
/// 2*ATLAS_DIM)` (bottom half). One texture sub-resource per impostor
/// (two separate atlases dropped the albedo sub-resource on .scn
/// load → white impostors). Shader samples `v*0.5` for albedo and
/// `v*0.5 + 0.5` for normal+ORM.
const PACKED_ATLAS_H: i32 = ATLAS_DIM * 2;
/// Padding multiplier on the per-cell ortho footprint so rotated
/// silhouettes never clip at cell edges. The worst case is the AABB
/// bounding-sphere diameter.
const CELL_SIZE_PADDING: f32 = 1.08;

/// Full-sphere by default. Hemi-sphere would halve the wasted cells
/// for ground-anchored props but DCL has plenty of meshes you do see
/// from below — ramps, ceiling fixtures, signs, picked-up items.
/// Not safe to generalize, so we pay for the full sphere coverage.
/// The shader's `is_hemi` uniform stays in place so a future per-prop
/// override can flip it without rebuilding the shader.
const HEMI_SPHERE_MODE: bool = false;

/// Frames between SubViewport spawn and texture readback. With the
/// explicit `RenderingServer::force_draw()` call in
/// `drain_bake_queue_on_main` synchronously flushing the render queue
/// before sampling, a single tick is enough — the previous 4-frame wait
/// was paranoia for the pre-force_draw era where llvmpipe could leave
/// commands queued indefinitely. 1 tick keeps the bake ~4× faster.
const FRAMES_TO_WAIT: u32 = 1;

/// Cap blocked worker threads. With godot_single_thread already
/// serializing to 1, only one worker is ever waiting — but keep a
/// timeout so a renderer hang doesn't deadlock the whole preprocess.
const BAKE_TIMEOUT: Duration = Duration::from_secs(60);

/// Strict 1-in-flight pacing. The previous attempt crashed after
/// ~17 bakes, suspected to be unbounded concurrent SubViewport
/// children. This serializes the renderer-touching slice of the
/// pipeline.
const MAX_IN_FLIGHT_REQUESTS: usize = 1;
const MAX_NEW_PER_TICK: usize = 1;

/// Shader source embedded in the lib so impostor materials don't
/// depend on a `res://` path being present in the device PCK.
/// Avoids `TAU`/`PI` identifiers (Godot's preprocessor already defines
/// them; redefining is a compile error).
///
/// Octahedral sampling follows the canonical Brucks form
/// (shaderbits.com/blog/octahedral-impostors) as adapted by
/// wojtekpil/Godot-Octahedral-Impostors: three-corner tri-blend across
/// the (0,0)→(1,1) diagonal of each grid cell, with the cell at grid
/// position (i, j) representing the view direction at uv =
/// (i/(N-1), j/(N-1)).
const SHADER_NORMAL_BAKE_RES_PATH: &str =
    "res://assets/shaders/dcl_octahedral_impostor_normal_bake.gdshader";

/// Path to the shader resource (shipped in the device + asset-server
/// PCKs). Single inline copy of the shader source lives in
/// `godot/assets/shaders/dcl_octahedral_impostor.gdshader` — all
/// ShaderMaterials saved to .scn files reference it by this path
/// (ExtResource), so the device loads + compiles the pipeline once,
/// not once per impostor.
const SHADER_RES_PATH: &str = "res://assets/shaders/dcl_octahedral_impostor.gdshader";

pub struct ImpostorJob {
    mesh: Gd<ArrayMesh>,
    material: Gd<ShaderMaterial>,
    /// Material override used as `material_override` on the normal
    /// bake's mesh copies — emits octa(NORMAL).xy + roughness/metallic
    /// packed RGBA8. Cloned per impostor so its uniforms carry the
    /// source material's roughness/metallic values.
    bake_material: Gd<godot::classes::Material>,
    /// Source MeshInstance3D — kept so the drain can roll back the
    /// `visibility_range_end` we set on it if the bake fails (no
    /// impostor → don't make the source vanish at distance).
    source_mi: Gd<MeshInstance3D>,
    /// Impostor MeshInstance3D — kept so the drain can `free()` it
    /// and detach from its parent if the bake fails.
    impostor_mi: Gd<MeshInstance3D>,
}

// SAFETY: `Gd<T>` is parked under a Mutex and only touched on the
// main thread or a worker that holds godot_single_thread.
unsafe impl Send for ImpostorJob {}

struct BakeRequest {
    jobs: Vec<ImpostorJob>,
    responder: mpsc::SyncSender<u32>,
}

unsafe impl Send for BakeRequest {}

static BAKE_QUEUE: Lazy<Mutex<Vec<BakeRequest>>> = Lazy::new(|| Mutex::new(Vec::new()));

struct InFlightSlot {
    /// Captures the raw unshaded mesh albedo via `DebugDraw::UNSHADED`.
    albedo_viewport: Gd<SubViewport>,
    /// Optional normal+ORM bake viewport. Set when we want
    /// per-pixel lighting on the impostor; disabled for now while
    /// we measure whether the 2-textures-per-impostor cost is what
    /// hangs the device load.
    normal_viewport: Option<Gd<SubViewport>>,
    material: Gd<ShaderMaterial>,
    source_mi: Gd<MeshInstance3D>,
    impostor_mi: Gd<MeshInstance3D>,
}

struct InFlightRequest {
    slots: Vec<InFlightSlot>,
    responder: mpsc::SyncSender<u32>,
    frames_remaining: u32,
}

unsafe impl Send for InFlightRequest {}

static IN_FLIGHT: Lazy<Mutex<Vec<InFlightRequest>>> = Lazy::new(|| Mutex::new(Vec::new()));

pub fn register_candidate(
    mi: &mut Gd<MeshInstance3D>,
    mesh: &Gd<ArrayMesh>,
    scene_root: &Gd<godot::classes::Node>,
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
    // Bounding-sphere diameter so the spherical billboard's footprint
    // contains the mesh from any view direction (matches the
    // octahedral bake's worst-case projection).
    let sphere_diam = (size.x * size.x + size.y * size.y + size.z * size.z).sqrt();
    let quad_size = sphere_diam.max(0.1);
    let switch_distance = switch_distance_for(sphere_diam);
    let fade_margin = fade_margin_for(switch_distance);

    let mut mat = ShaderMaterial::new_gd();
    mat.set_shader(&shader);
    let placeholder_atlas: Gd<godot::classes::Texture2D> = make_placeholder_atlas().upcast();
    mat.set_shader_parameter("atlas", &placeholder_atlas.to_variant());
    mat.set_shader_parameter("n_cells", &(N_CELLS as i64).to_variant());
    mat.set_shader_parameter("is_hemi", &HEMI_SPHERE_MODE.to_variant());

    let mut quad = QuadMesh::new_gd();
    quad.set_size(Vector2::new(quad_size, quad_size));

    let mut impostor_mi = MeshInstance3D::new_alloc();
    impostor_mi.set_name("dcl_impostor");
    impostor_mi.set_mesh(&quad.upcast::<godot::classes::Mesh>());
    impostor_mi.set_surface_override_material(0, &mat.clone().upcast::<godot::classes::Material>());
    impostor_mi.set_position(center_local);
    impostor_mi
        .set_cast_shadows_setting(godot::classes::geometry_instance_3d::ShadowCastingSetting::OFF);
    impostor_mi.set_visibility_range_begin(switch_distance);
    impostor_mi.set_visibility_range_begin_margin(fade_margin);
    impostor_mi.set_visibility_range_fade_mode(VisibilityRangeFadeMode::SELF);

    mi.add_child(&impostor_mi.clone().upcast::<godot::classes::Node>());

    // Without an explicit owner, PackedScene::pack drops the impostor
    // node when save_node_as_scene serializes the GLTF root — the
    // saved .scn would carry the source MI's VisibilityRange (which
    // makes it invisible past `switch_distance`) but no impostor, so
    // the device-side renderer shows nothing at distance and the
    // mesh pops in only when the camera moves back inside the
    // visibility range. mi.get_owner() returns null on freshly-
    // imported GLTF children, so the root is threaded down from the
    // walker explicitly.
    impostor_mi.set_owner(scene_root);

    mi.set_visibility_range_end(switch_distance);
    mi.set_visibility_range_end_margin(fade_margin);
    mi.set_visibility_range_fade_mode(VisibilityRangeFadeMode::SELF);

    mi.set_meta("dcl_impostor_attached", &true.to_variant());

    let bake_material = make_normal_bake_material(mi)?;

    Some(ImpostorJob {
        mesh: mesh.clone(),
        material: mat,
        bake_material,
        source_mi: mi.clone(),
        impostor_mi: impostor_mi.clone(),
    })
}

/// Park the jobs in the bake queue and block until the main-thread
/// drain reports them done. Returns the count of successfully baked
/// atlases.
pub fn enqueue_and_wait(jobs: Vec<ImpostorJob>) -> u32 {
    if jobs.is_empty() {
        return 0;
    }
    let (tx, rx) = mpsc::sync_channel::<u32>(1);
    let req = BakeRequest {
        jobs,
        responder: tx,
    };
    match BAKE_QUEUE.lock() {
        Ok(mut q) => q.push(req),
        Err(_) => return 0,
    }
    rx.recv_timeout(BAKE_TIMEOUT).unwrap_or(0)
}

/// Main-thread tick: advance in-flight bakes one frame, complete any
/// that finished waiting, and start at most one new bake.
pub fn drain_bake_queue_on_main(parent: &mut Gd<godot::classes::Node>) {
    // Stage 1: advance in-flight requests.
    let in_flight: Vec<InFlightRequest> = match IN_FLIGHT.lock() {
        Ok(mut q) => std::mem::take(&mut *q),
        Err(_) => return,
    };
    let mut survivors: Vec<InFlightRequest> = Vec::with_capacity(in_flight.len());
    // Track whether any request will read back this tick. Under llvmpipe
    // (CPU GL on the asset-server, no presentable surface), queued
    // SubViewport render commands aren't guaranteed to have executed by
    // the time `process()` returns — the readback then reads uninitialized
    // GPU memory and the impostor atlas comes back as per-allocation
    // noise. Forcing a synchronous draw flushes the queue before we sample
    // the textures.
    let mut needs_force_draw = false;
    for req in in_flight.iter() {
        if req.frames_remaining <= 1 {
            needs_force_draw = true;
            break;
        }
    }
    if needs_force_draw {
        godot::classes::RenderingServer::singleton().force_draw();
    }
    for mut req in in_flight {
        req.frames_remaining = req.frames_remaining.saturating_sub(1);
        if req.frames_remaining > 0 {
            survivors.push(req);
            continue;
        }
        let mut baked = 0u32;
        let mut fail_blank_albedo = 0u32;
        let mut fail_missing_normal = 0u32;
        let mut fail_stitch = 0u32;
        for slot in req.slots.iter() {
            // Read albedo (coverage-checked) + normal (no check) as raw
            // RGBA8 images, then stitch into ONE packed ETC2 ImageTexture
            // (albedo top half, normal+ORM bottom half). A single
            // sub-resource per impostor serializes cleanly; two separate
            // atlases dropped the albedo on load.
            let albedo_img = read_baked_image_raw(&slot.albedo_viewport, true);
            let normal_img = slot
                .normal_viewport
                .as_ref()
                .and_then(|vp| read_baked_image_raw(vp, false));
            let packed = match (&albedo_img, &normal_img) {
                (Some(a), Some(n)) => stitch_atlas_texture(a, n),
                _ => None,
            };
            match packed {
                Some(packed_tex) => {
                    let mut mat = slot.material.clone();
                    mat.set_shader_parameter("atlas", &packed_tex.to_variant());
                    baked += 1;
                }
                None => {
                    if albedo_img.is_none() {
                        fail_blank_albedo += 1;
                    } else if normal_img.is_none() {
                        fail_missing_normal += 1;
                    } else {
                        fail_stitch += 1;
                    }
                    // Bake failed (blank albedo / missing normal / stitch
                    // error). Roll back the impostor: detach + free the
                    // impostor MI, clear the source MI's
                    // visibility_range_end so the prop renders
                    // normally without a placeholder showing through.
                    let impostor = slot.impostor_mi.clone();
                    if let Some(p) = impostor.get_parent() {
                        let mut pp = p;
                        pp.remove_child(&impostor.clone().upcast::<godot::classes::Node>());
                    }
                    impostor.upcast::<godot::classes::Node>().free();
                    let mut src = slot.source_mi.clone();
                    src.set_visibility_range_end(0.0);
                    src.set_visibility_range_end_margin(0.0);
                }
            }
        }
        let total = req.slots.len() as u32;
        if total != baked {
            godot::global::godot_print!(
                "[impostor-bake] requested={} baked={} fail_blank_albedo={} fail_missing_normal={} fail_stitch={}",
                total,
                baked,
                fail_blank_albedo,
                fail_missing_normal,
                fail_stitch,
            );
        }
        let _ = req.responder.send(baked);
        // Synchronous free: drop the SubViewports (and their children +
        // dedicated World3Ds) before the next bake materializes so the
        // RenderingServer's instance tables stay bounded.
        for slot in req.slots {
            let albedo_node: Gd<godot::classes::Node> = slot.albedo_viewport.upcast();
            parent.remove_child(&albedo_node);
            albedo_node.free();
            if let Some(normal_vp) = slot.normal_viewport {
                let normal_node: Gd<godot::classes::Node> = normal_vp.upcast();
                parent.remove_child(&normal_node);
                normal_node.free();
            }
        }
    }
    if let Ok(mut q) = IN_FLIGHT.lock() {
        q.extend(survivors);
    }

    // Stage 2: start new requests, capped at one in flight.
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
            let Some(albedo) = setup_subviewport(
                &job.mesh,
                godot::classes::viewport::DebugDraw::UNSHADED,
                None,
            ) else {
                continue;
            };
            // Second pass: normal+ORM bake via the bake-material override
            // (writes octa(view_normal).rg + roughness B + metallic A as
            // ALBEDO, so capture UNSHADED/DISABLED, not NORMAL_BUFFER).
            // Two viewports during the bake, but stage 1 stitches both
            // images into ONE packed ETC2 ImageTexture before the .scn is
            // saved, so each impostor still owns a single texture sub-resource.
            let Some(normal) = setup_subviewport(
                &job.mesh,
                godot::classes::viewport::DebugDraw::DISABLED,
                Some(job.bake_material.clone()),
            ) else {
                continue;
            };
            parent.add_child(&albedo.clone().upcast::<godot::classes::Node>());
            parent.add_child(&normal.clone().upcast::<godot::classes::Node>());
            slots.push(InFlightSlot {
                albedo_viewport: albedo,
                normal_viewport: Some(normal),
                material: job.material,
                source_mi: job.source_mi,
                impostor_mi: job.impostor_mi,
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
}

/// Build a SubViewport hosting N×N rotated copies of the source mesh
/// laid out in a grid. A single orthographic +Z camera renders all
/// cells in one frame; cell (i, j) contains the mesh as seen from
/// octahedral direction `d_ij`. `debug_draw` selects what gets
/// captured — `UNSHADED` for raw albedo, `NORMAL_BUFFER` for the
/// view-space normal map.
fn setup_subviewport(
    mesh: &Gd<ArrayMesh>,
    debug_draw: godot::classes::viewport::DebugDraw,
    material_override: Option<Gd<godot::classes::Material>>,
) -> Option<Gd<SubViewport>> {
    let aabb = mesh.get_aabb();
    let size = aabb.size;
    let center = aabb.position + size * 0.5;
    let sphere_radius = (size.x * size.x + size.y * size.y + size.z * size.z).sqrt() * 0.5;
    if sphere_radius < 0.05 {
        return None;
    }
    let cell_world = sphere_radius * 2.0 * CELL_SIZE_PADDING;

    let mut subviewport = SubViewport::new_alloc();
    subviewport.set_size(Vector2i::new(ATLAS_DIM, ATLAS_DIM));
    subviewport.set_transparent_background(true);
    subviewport.set_update_mode(UpdateMode::ALWAYS);
    subviewport.set_clear_mode(ClearMode::ALWAYS);
    subviewport.set_disable_3d(false);

    // Dedicated world per bake. Avoids inheriting whatever else is
    // parked in the main scene root and guarantees that freeing the
    // SubViewport drops every RenderingServer instance it created.
    let world = World3D::new_gd();
    subviewport.set_world_3d(&world);
    subviewport.set_debug_draw(debug_draw);

    let n = N_CELLS as f32;
    let n_minus_one = (N_CELLS - 1).max(1) as f32;
    // Layout: cell (i, j) → world (col_x[i], row_y[j], 0).
    //   col_x[i] = (i + 0.5 - N/2) * cell_world      (i=0 → leftmost / smallest u_atlas)
    //   row_y[j] = (N/2 - j - 0.5) * cell_world      (j=0 → top of image / smallest v_atlas)
    // Camera at +Z looking at origin sees +Y as image-up; image v=0
    // is the top row, so highest-Y world position lands at j=0.
    //
    // Cell-center direction uses the Brucks/wojtekpil grid-point
    // convention (uv = i/(N-1), not (i+0.5)/N): boundary cells land
    // exactly on the octahedron's extreme directions, matching the
    // runtime shader's `grid * (N-1)` indexing.
    for j in 0..N_CELLS {
        for i in 0..N_CELLS {
            let uv = Vector2::new(i as f32 / n_minus_one, j as f32 / n_minus_one);
            let d = grid_uv_to_dir(uv);

            // Inverse of the d-camera's world orientation. Applied to
            // the mesh, the fixed +Z camera sees what a camera placed
            // at direction d would have seen.
            let r_d = view_basis_for_direction(d);
            let mesh_rot = r_d.transposed();

            let col_x = (i as f32 + 0.5 - n * 0.5) * cell_world;
            let row_y = (n * 0.5 - j as f32 - 0.5) * cell_world;
            let translation = Vector3::new(col_x, row_y, 0.0) - mesh_rot * center;

            let mut temp_mi = MeshInstance3D::new_alloc();
            temp_mi.set_mesh(&mesh.clone().upcast::<godot::classes::Mesh>());
            temp_mi.set_transform(Transform3D {
                basis: mesh_rot,
                origin: translation,
            });
            if let Some(ref mat) = material_override {
                temp_mi.set_material_override(mat);
            }
            subviewport.add_child(&temp_mi.upcast::<godot::classes::Node>());
        }
    }

    // Orthographic +Z camera covering the full N×N grid in HEIGHT-keep
    // mode (square viewport → width == height).
    let camera_height = cell_world * n;
    let cam_distance = sphere_radius * 8.0 + cell_world;
    let mut camera = Camera3D::new_alloc();
    camera.set_projection(ProjectionType::ORTHOGONAL);
    camera.set_keep_aspect_mode(KeepAspect::HEIGHT);
    camera.set_size(camera_height);
    camera.set_near(0.01);
    camera.set_far(cam_distance * 4.0);
    let cam_pos = Vector3::new(0.0, 0.0, cam_distance);
    let cam_target = Vector3::ZERO;
    camera.look_at_from_position(cam_pos, cam_target);
    camera.set_current(true);
    subviewport.add_child(&camera.upcast::<godot::classes::Node>());

    Some(subviewport)
}

/// World-space basis of a camera placed at direction `d` from origin
/// looking back at origin. Columns are the camera's local X, Y, Z
/// axes expressed in world coordinates.
///
/// Convention matches the runtime shader's billboard: prefer world-Y
/// as up; when `d` collapses onto Y (|d.y| > 0.999) fall back to
/// world-Z to avoid a degenerate cross product.
fn view_basis_for_direction(d: Vector3) -> Basis {
    let d = d.normalized();
    let up_world = if d.y.abs() > 0.999 {
        Vector3::new(0.0, 0.0, 1.0)
    } else {
        Vector3::new(0.0, 1.0, 0.0)
    };
    // z_cam points away from target (toward camera position D*d) → +d.
    let z_cam = d;
    let x_cam = up_world.cross(z_cam).normalized();
    let y_cam = z_cam.cross(x_cam);
    Basis::from_cols(x_cam, y_cam, z_cam)
}

/// Grid UV (in [0, 1]²) → unit view direction. Matches the runtime
/// shader's `dir_to_grid` inverse for the active sphere mode.
fn grid_uv_to_dir(uv: Vector2) -> Vector3 {
    if HEMI_SPHERE_MODE {
        hemi_grid_uv_to_dir(uv)
    } else {
        full_grid_uv_to_dir(uv)
    }
}

/// Full-sphere octahedral mapping inverse (Cigolle/Brucks). The y
/// component carries the hemisphere; the four corners of the unit
/// square fold onto the -Y pole (intentional degeneracy).
fn full_grid_uv_to_dir(uv: Vector2) -> Vector3 {
    let p = Vector2::new(uv.x * 2.0 - 1.0, uv.y * 2.0 - 1.0);
    let y = 1.0 - p.x.abs() - p.y.abs();
    let (x, z) = if y >= 0.0 {
        (p.x, p.y)
    } else {
        let sgn_x = if p.x >= 0.0 { 1.0 } else { -1.0 };
        let sgn_z = if p.y >= 0.0 { 1.0 } else { -1.0 };
        ((1.0 - p.y.abs()) * sgn_x, (1.0 - p.x.abs()) * sgn_z)
    };
    Vector3::new(x, y, z).normalized()
}

/// Hemi-sphere octahedral mapping inverse: zenith at uv (0.5, 0.5),
/// horizon at the unit square's edges (rotated 45° diamond unfolding).
fn hemi_grid_uv_to_dir(uv: Vector2) -> Vector3 {
    let coord = Vector2::new(uv.x, uv.y);
    let x = coord.x - coord.y;
    let z = -1.0 + coord.x + coord.y;
    let y = 1.0 - x.abs() - z.abs();
    Vector3::new(x, y, z).normalized()
}

/// Read a viewport's texture as a standalone, uncompressed RGBA8
/// `Image`. Detaching (copying pixels into a fresh Image) is required
/// so the later packed atlas is embeddable inline in the .scn — an
/// Image straight from `ViewportTexture::get_image()` carries lifetime
/// baggage `PackedScene::pack` refuses to embed. `check_coverage`
/// rejects near-empty albedo bakes (normal pass skips it — transparent
/// pixels dominate there).
fn read_baked_image_raw(subviewport: &Gd<SubViewport>, check_coverage: bool) -> Option<Gd<Image>> {
    let tex = subviewport.get_texture()?;
    let image = tex.get_image()?;
    let w = image.get_width();
    let h = image.get_height();
    if w == 0 || h == 0 {
        return None;
    }
    if check_coverage && image_is_blank(&image) {
        return None;
    }
    let format = image.get_format();
    let data = image.get_data();
    let mut detached = Image::create_from_data(w, h, false, format, &data)?;
    if detached.get_format() != godot::classes::image::Format::RGBA8 {
        detached.convert(godot::classes::image::Format::RGBA8);
    }
    if detached.has_mipmaps() {
        detached.clear_mipmaps();
    }
    Some(detached)
}

/// Stitch albedo (top half) + normal+ORM (bottom half) into one
/// `ATLAS_DIM × PACKED_ATLAS_H` ETC2 `ImageTexture`. The ETC2 image is
/// wrapped in a plain `ImageTexture` (not a `PortableCompressedTexture2D`):
/// on a real device GPU this round-trips correctly through the `.scn`
/// cache (`save_node_as_scene` → reload), whereas a PCT2 baked on the
/// asset server reloads as solid magenta on Mali — the same failure that
/// hit avatar-mesh textures (see `content::texture::create_compressed_texture`).
/// One sub-resource per impostor — two separate atlases dropped the
/// albedo on .scn load (white impostors).
fn stitch_atlas_texture(albedo: &Gd<Image>, normal: &Gd<Image>) -> Option<Gd<Texture2D>> {
    let w = ATLAS_DIM;
    if albedo.get_width() != w
        || albedo.get_height() != w
        || normal.get_width() != w
        || normal.get_height() != w
    {
        return None;
    }
    let mut packed = Image::create_empty(
        w,
        PACKED_ATLAS_H,
        false,
        godot::classes::image::Format::RGBA8,
    )?;
    let src_rect = godot::builtin::Rect2i::new(
        godot::builtin::Vector2i::new(0, 0),
        godot::builtin::Vector2i::new(w, w),
    );
    packed.blit_rect(albedo, src_rect, godot::builtin::Vector2i::new(0, 0));
    packed.blit_rect(normal, src_rect, godot::builtin::Vector2i::new(0, w));
    let _ = packed.generate_mipmaps();
    let result = packed.compress(CompressMode::ETC2);
    if result != Error::OK {
        // Fall through with the uncompressed image — create_from_image
        // still yields a valid (if heavier) atlas rather than a magenta
        // placeholder.
        tracing::warn!(
            "impostor atlas ETC2 compression failed ({:?}), using uncompressed",
            result
        );
    }
    ImageTexture::create_from_image(&packed).map(|t| t.upcast::<Texture2D>())
}

fn image_is_blank(image: &Gd<Image>) -> bool {
    image_alpha_coverage(image) < MIN_COVERAGE_TO_KEEP
}

/// Minimum fraction of opaque alpha pixels in the baked atlas to
/// keep the impostor. Sparse meshes (cables, antennas, lattices,
/// ropes) have AABBs that pass the size filter but produce mostly
/// transparent atlases — the impostor billboard is wasted there
/// because the runtime shader discards nearly every fragment.
const MIN_COVERAGE_TO_KEEP: f32 = 0.01;

/// Stride-sampled estimate of "fraction of pixels with alpha > 0.05".
/// Reading every fourth byte (the alpha channel) at a stride of 16
/// gives ~1/16 sampling density — enough resolution for a coverage
/// ratio without scanning the full image.
fn image_alpha_coverage(image: &Gd<Image>) -> f32 {
    let w = image.get_width();
    let h = image.get_height();
    if w == 0 || h == 0 {
        return 0.0;
    }
    let data = image.get_data();
    let len = data.len();
    let mut sampled: u32 = 0;
    let mut opaque: u32 = 0;
    let mut i = 3usize;
    let stride = 16usize;
    while i < len {
        if let Some(a) = data.get(i) {
            sampled += 1;
            if a > 12 {
                opaque += 1;
            }
        }
        i += stride;
    }
    if sampled == 0 {
        return 0.0;
    }
    opaque as f32 / sampled as f32
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
    // Reject geometry whose AABB center sits far from the node origin:
    // the impostor (anchored at the center) and the source MI (anchored
    // at the origin) would then swap at very different camera distances,
    // leaving large/off-center props stuck showing the impostor up close.
    let center_offset = (aabb.position + size * 0.5).length();
    if center_offset > diag * MAX_CENTER_OFFSET_DIAG_RATIO {
        return false;
    }
    // Reject sub-15 cm thin geometry (decals, posters, fullscreen
    // overlays) and elongated/flat shapes (wall sections, signs,
    // floor tiles) where one axis is way thinner than the others.
    // Impostor billboards for those just duplicate the same plane
    // and tend to show placeholder magenta when the bake can't
    // capture a silhouette from the side.
    let min_axis = size.x.min(size.y).min(size.z);
    let max_axis = size.x.max(size.y).max(size.z);
    if min_axis < MIN_AXIS_M {
        return false;
    }
    if max_axis > 0.0 && min_axis / max_axis < MIN_AXIS_RATIO {
        return false;
    }
    if mesh.get_blend_shape_count() > 0 {
        return false;
    }
    // Skip skinned meshes (no skeleton available at bake time).
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
    // Material filter: accept opaque AND alpha-tested (scissor/hash) —
    // foliage, decals, posters all use ALPHA_SCISSOR and the bake's
    // SubViewport captures their silhouette correctly with alpha
    // discard. The only kind we still reject is real alpha-blended
    // transparency (e.g., glass), where a billboard with a single
    // alpha channel can't reproduce the see-through layering.
    if let Some(mat) = mi.get_active_material(0) {
        if let Ok(base) = mat.try_cast::<BaseMaterial3D>() {
            let t = base.get_transparency();
            if t == Transparency::ALPHA {
                return false;
            }
        }
    }
    true
}

struct SharedShader(Option<Gd<Shader>>);

// SAFETY: parked under a Mutex; only the worker thread that holds
// godot_single_thread touches the inner Gd.
unsafe impl Send for SharedShader {}

static SHARED_SHADER: Lazy<Mutex<SharedShader>> = Lazy::new(|| Mutex::new(SharedShader(None)));

fn load_shader() -> Option<Gd<Shader>> {
    let mut guard = SHARED_SHADER.lock().ok()?;
    if guard.0.is_none() {
        // Load the shader as a `res://` external resource so every
        // ShaderMaterial in every saved .scn references the SAME
        // Shader object — only ONE Shader is shipped per device PCK
        // (instead of one inline copy per impostor scene), and
        // Mali's pipeline cache only compiles it once across all
        // impostors on the load.
        guard.0 = godot::classes::ResourceLoader::singleton()
            .load(SHADER_RES_PATH)
            .and_then(|res| res.try_cast::<Shader>().ok());
    }
    guard.0.clone()
}

/// Load the normal-bake Shader resource (shared across impostors).
/// Each impostor still clones a per-instance ShaderMaterial so its
/// uniforms can carry per-source roughness/metallic.
struct SharedNormalBakeShader(Option<Gd<Shader>>);
unsafe impl Send for SharedNormalBakeShader {}

static SHARED_NORMAL_BAKE_SHADER: Lazy<Mutex<SharedNormalBakeShader>> =
    Lazy::new(|| Mutex::new(SharedNormalBakeShader(None)));

fn load_normal_bake_shader() -> Option<Gd<Shader>> {
    let mut guard = SHARED_NORMAL_BAKE_SHADER.lock().ok()?;
    if guard.0.is_none() {
        let loaded: Option<Gd<Shader>> = godot::classes::ResourceLoader::singleton()
            .load(SHADER_NORMAL_BAKE_RES_PATH)
            .and_then(|res| res.try_cast::<Shader>().ok());
        guard.0 = loaded;
    }
    guard.0.clone()
}

/// Per-impostor ShaderMaterial wrapping the shared normal-bake shader,
/// with `src_roughness` / `src_metallic` uniforms set from the source
/// mesh's BaseMaterial3D values.
fn make_normal_bake_material(
    source_mi: &Gd<MeshInstance3D>,
) -> Option<Gd<godot::classes::Material>> {
    let shader = load_normal_bake_shader()?;
    let mut mat = ShaderMaterial::new_gd();
    mat.set_shader(&shader);

    // Read roughness/metallic from the source material if it's a
    // BaseMaterial3D. Defaults are conservative — most DCL props
    // are dielectric (metallic=0) with mid-roughness (~0.7).
    let mut roughness: f32 = 0.7;
    let mut metallic: f32 = 0.0;
    if let Some(src_mat) = source_mi.get_active_material(0) {
        if let Ok(base) = src_mat.try_cast::<BaseMaterial3D>() {
            roughness = base.get_roughness();
            metallic = base.get_metallic();
        }
    }
    mat.set_shader_parameter("src_roughness", &roughness.to_variant());
    mat.set_shader_parameter("src_metallic", &metallic.to_variant());

    Some(mat.upcast::<godot::classes::Material>())
}

fn make_placeholder_atlas() -> Gd<ImageTexture> {
    let img = Image::create_from_data(
        1,
        1,
        false,
        godot::classes::image::Format::RGBA8,
        // DIAG: GREEN placeholder. If user still sees MAGENTA on
        // device, the magenta is Godot's shader-compile-fail
        // fallback (or ETC2 sample fail), not our placeholder. If
        // user sees GREEN, the bake's atlas swap is silently failing.
        &PackedByteArray::from(&[0u8, 255, 0, 255][..]),
    )
    .expect("create 1x1 image");
    ImageTexture::create_from_image(&img).expect("create image texture")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32, eps: f32) -> bool {
        (a - b).abs() < eps
    }

    /// Round-trip: oct UV → dir → oct UV should land back at the same
    /// continuous coordinate (modulo the lower-hemisphere fold).
    #[test]
    fn octahedral_roundtrip_upper_hemisphere() {
        // Sample interior of the diamond |x| + |y| <= 1 in [-1, 1]² ;
        // these correspond to the upper hemisphere with no fold.
        let samples = [
            Vector2::new(0.5, 0.5),
            Vector2::new(0.25, 0.5),
            Vector2::new(0.75, 0.5),
            Vector2::new(0.5, 0.25),
            Vector2::new(0.5, 0.75),
        ];
        for uv in samples {
            let d = full_grid_uv_to_dir(uv);
            // Length is normalized
            let len_sq = d.x * d.x + d.y * d.y + d.z * d.z;
            assert!(approx_eq(len_sq, 1.0, 1e-4), "uv={uv:?} len_sq={len_sq}");
            // Upper hemisphere: y >= 0
            assert!(d.y >= -1e-4, "uv={uv:?} y={}", d.y);
        }
    }

    #[test]
    fn octahedral_center_is_y_up() {
        let d = full_grid_uv_to_dir(Vector2::new(0.5, 0.5));
        assert!(approx_eq(d.x, 0.0, 1e-4));
        assert!(approx_eq(d.y, 1.0, 1e-4));
        assert!(approx_eq(d.z, 0.0, 1e-4));
    }

    /// Hemi mode: center UV → +Y, corners → horizon (y=0).
    #[test]
    fn hemi_grid_center_is_y_up() {
        let d = hemi_grid_uv_to_dir(Vector2::new(0.5, 0.5));
        assert!(approx_eq(d.x, 0.0, 1e-4), "x={}", d.x);
        assert!(approx_eq(d.y, 1.0, 1e-4), "y={}", d.y);
        assert!(approx_eq(d.z, 0.0, 1e-4), "z={}", d.z);
    }

    #[test]
    fn hemi_grid_corners_are_horizon() {
        for uv in [
            Vector2::new(0.0, 0.0),
            Vector2::new(1.0, 0.0),
            Vector2::new(0.0, 1.0),
            Vector2::new(1.0, 1.0),
        ] {
            let d = hemi_grid_uv_to_dir(uv);
            assert!(approx_eq(d.y, 0.0, 1e-4), "uv={uv:?} y={}", d.y);
            let len_sq = d.x * d.x + d.y * d.y + d.z * d.z;
            assert!(approx_eq(len_sq, 1.0, 1e-4));
        }
    }

    /// Full sphere: cell (0, 0) at uv (0, 0) should map to -Y pole.
    #[test]
    fn full_grid_corner_is_minus_y() {
        let d = full_grid_uv_to_dir(Vector2::new(0.0, 0.0));
        assert!(approx_eq(d.y, -1.0, 1e-4), "y={}", d.y);
    }
}
