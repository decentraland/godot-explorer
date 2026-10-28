//! Times how long the meshes in view stay undrawn (async shader compilation skips a surface until
//! its pipeline compiles) after the loading screen lifts or a graphics setting changes, and
//! reports it as a RENDER_READY metric and a `[RenderReady]` log line.

use godot::builtin::{Aabb, Plane};
use godot::classes::geometry_instance_3d::ShadowCastingSetting;
use godot::classes::rendering_server::RenderingInfo;
use godot::classes::{Camera3D, INode, MeshInstance3D, Node, RenderingServer, Time};
use godot::obj::InstanceId;
use godot::prelude::*;

use super::feature_flags::FLAG_RENDER_READY_PROBE;
use crate::fetcher::globals;
use crate::godot_classes::dcl_global::DclGlobal;

const TIMEOUT_MSEC: u64 = 15000;
const MAX_SAMPLES: usize = 64;
/// What a graphics profile applies that changes scene shader variants or render passes.
const SETTINGS: [&str; 7] = [
    "graphic_profile",
    "anti_aliasing",
    "shadow_quality",
    "bloom_quality",
    "skybox",
    "resolution_3d_scale",
    "particle_quality",
];
const PIPELINE_COUNTERS: [(&str, RenderingInfo); 4] = [
    ("pipelines_mesh", RenderingInfo::PIPELINE_COMPILATIONS_MESH),
    (
        "pipelines_surface",
        RenderingInfo::PIPELINE_COMPILATIONS_SURFACE,
    ),
    ("pipelines_draw", RenderingInfo::PIPELINE_COMPILATIONS_DRAW),
    (
        "pipelines_specialization",
        RenderingInfo::PIPELINE_COMPILATIONS_SPECIALIZATION,
    ),
];

/// The readiness timeline of one measurement. Times are `-1` until reached.
struct Timeline {
    ready_at_start_pct: i64,
    t50_msec: i64,
    t90_msec: i64,
}

#[derive(Debug, PartialEq)]
enum Verdict {
    Pending,
    Ready,
    TimedOut,
}

impl Timeline {
    fn new() -> Self {
        Self {
            ready_at_start_pct: -1,
            t50_msec: -1,
            t90_msec: -1,
        }
    }

    /// With nothing in view the fraction is zero, so an empty view only ends at the timeout.
    fn observe(&mut self, elapsed_msec: u64, in_view: usize, drawn: usize) -> Verdict {
        let fraction = if in_view > 0 {
            drawn as f32 / in_view as f32
        } else {
            0.0
        };
        if self.ready_at_start_pct < 0 {
            self.ready_at_start_pct = (fraction * 100.0).round() as i64;
        }
        if self.t50_msec < 0 && fraction >= 0.5 {
            self.t50_msec = elapsed_msec as i64;
        }
        if self.t90_msec < 0 && fraction >= 0.9 {
            self.t90_msec = elapsed_msec as i64;
        }
        if fraction >= 1.0 {
            Verdict::Ready
        } else if elapsed_msec >= TIMEOUT_MSEC {
            Verdict::TimedOut
        } else {
            Verdict::Pending
        }
    }
}

/// The ids of the nearest `MAX_SAMPLES` candidates, given as (squared distance, id).
fn nearest_ids(mut candidates: Vec<(f32, i64)>) -> Vec<i64> {
    candidates.sort_by(|a, b| a.0.total_cmp(&b.0));
    candidates
        .into_iter()
        .take(MAX_SAMPLES)
        .map(|(_, id)| id)
        .collect()
}

fn in_frustum(planes: &[Plane], aabb: Aabb) -> bool {
    planes
        .iter()
        .all(|plane| !plane.is_point_over(aabb.get_support(-plane.normal)))
}

fn pipeline_counts() -> [u64; 4] {
    let rendering = RenderingServer::singleton();
    PIPELINE_COUNTERS.map(|(_, info)| rendering.get_rendering_info(info))
}

fn setting_int(name: &str) -> i64 {
    globals::engine()
        .bind()
        .setting(name.into())
        .try_to::<i64>()
        .unwrap_or(0)
}

fn now_msec() -> u64 {
    Time::singleton().get_ticks_msec()
}

fn mesh_from_id(id: i64) -> Option<Gd<MeshInstance3D>> {
    Gd::<Object>::try_from_instance_id(InstanceId::from_i64(id))
        .ok()?
        .try_cast::<MeshInstance3D>()
        .ok()
}

/// The nearest meshes the camera would draw, as object ids so a freed mesh just drops out.
fn sample_meshes(camera: &Gd<Camera3D>) -> Vec<i64> {
    let eye = camera.get_global_position();
    let Some(scenario) = camera.get_world_3d().map(|world| world.get_scenario()) else {
        return Vec::new();
    };
    let culled = RenderingServer::singleton()
        .instances_cull_convex_ex(&camera.get_frustum())
        .scenario(scenario)
        .done();
    let cull_mask = camera.get_cull_mask();
    let candidates = culled
        .as_slice()
        .iter()
        .filter_map(|&id| {
            let mesh = mesh_from_id(id)?;
            if mesh.get_mesh().is_none() || !mesh.is_visible_in_tree() {
                return None;
            }
            // Never drawn by this camera (other layer, shadow-only) or drawn only within a range:
            // they would never be ready.
            if mesh.get_layer_mask() & cull_mask == 0
                || mesh.get_cast_shadows_setting() == ShadowCastingSetting::SHADOWS_ONLY
                || mesh.get_visibility_range_end() > 0.0
            {
                return None;
            }
            Some((eye.distance_squared_to(mesh.get_global_position()), id))
        })
        .collect();
    nearest_ids(candidates)
}

#[derive(GodotClass)]
#[class(base=Node, init)]
pub struct RenderReadyProbe {
    base: Base<Node>,
    reason: &'static str,
    profile: i64,
    from_profile: i64,
    menu_open: bool,
    triggered_msec: u64,
    start_msec: Option<u64>,
    last_frame_msec: u64,
    max_frame_msec: u64,
    frames: u32,
    pipelines_before: [u64; 4],
    sample: Vec<i64>,
    timeline: Option<Timeline>,
}

#[godot_api]
impl INode for RenderReadyProbe {
    fn ready(&mut self) {
        self.base_mut().set_process(false);
        self.profile = setting_int("graphic_profile");
        let me = self.to_gd();
        DclGlobal::singleton()
            .signals()
            .loading_finished()
            .connect_other(&me, |this: &mut Self| this.start("loading"));
        globals::engine().signals().setting_changed().connect_other(
            &me,
            |this: &mut Self, name| {
                if SETTINGS.contains(&name.to_string().as_str()) {
                    this.start("settings");
                }
            },
        );
    }

    fn process(&mut self, _delta: f64) {
        self.tick();
    }
}

impl RenderReadyProbe {
    /// A profile apply emits several settings in one frame; each restarts the measurement, and the
    /// profile is set last, so `from_profile` still holds the one before the apply.
    fn start(&mut self, reason: &'static str) {
        if !dcl_core::flags::is_enabled(FLAG_RENDER_READY_PROBE, true) {
            return;
        }
        self.reason = reason;
        self.from_profile = self.profile;
        self.profile = setting_int("graphic_profile");
        self.menu_open = self
            .base()
            .get_viewport()
            .is_some_and(|viewport| viewport.is_3d_disabled());
        self.triggered_msec = now_msec();
        self.start_msec = None;
        self.frames = 0;
        self.max_frame_msec = 0;
        self.timeline = Some(Timeline::new());
        self.pipelines_before = pipeline_counts();
        self.base_mut().set_process(true);
    }

    fn stop(&mut self) {
        self.base_mut().set_process(false);
    }

    fn tick(&mut self) {
        let now = now_msec();
        let Some(viewport) = self.base().get_viewport() else {
            return;
        };
        let Some(camera) = viewport.get_camera_3d() else {
            return self.stop();
        };
        let Some(start_msec) = self.start_msec else {
            // Settings are changed from the menu, which turns 3D off: their cost lands once it
            // closes.
            if viewport.is_3d_disabled() {
                return;
            }
            self.start_msec = Some(if self.menu_open {
                now
            } else {
                self.triggered_msec
            });
            self.last_frame_msec = now;
            self.sample = sample_meshes(&camera);
            if self.sample.is_empty() {
                self.stop();
            }
            // Readiness describes the last rendered frame, so the first check waits for one.
            return;
        };

        self.frames += 1;
        self.max_frame_msec = self.max_frame_msec.max(now - self.last_frame_msec);
        self.last_frame_msec = now;
        let planes: Vec<Plane> = camera.get_frustum().iter_shared().collect();
        let rendering = RenderingServer::singleton();
        let (mut in_view, mut drawn) = (0, 0);
        for id in &self.sample {
            let Some(mesh) = mesh_from_id(*id).filter(|m| m.is_visible_in_tree()) else {
                continue;
            };
            // Readiness needs a color-pass draw, so meshes the camera turned away from don't count.
            if !in_frustum(&planes, mesh.get_global_transform() * mesh.get_aabb()) {
                continue;
            }
            in_view += 1;
            if rendering.instance_geometry_is_draw_ready(mesh.get_instance()) {
                drawn += 1;
            }
        }

        let elapsed = now - start_msec;
        let Some(timeline) = self.timeline.as_mut() else {
            return self.stop();
        };
        match timeline.observe(elapsed, in_view, drawn) {
            Verdict::Pending => {}
            Verdict::Ready => self.report(in_view, drawn, elapsed as i64),
            Verdict::TimedOut => self.report(in_view, drawn, -1),
        }
    }

    fn report(&mut self, in_view: usize, drawn: usize, t100_msec: i64) {
        self.stop();
        let Some(timeline) = self.timeline.as_ref() else {
            return;
        };
        let fetcher = globals::scene_fetcher();
        let (position, scene_id) = {
            let fetcher = fetcher.bind();
            (
                fetcher.get_current_position(),
                fetcher.get_current_scene_entity_id().to_string(),
            )
        };
        let launch_count = dcl_core::Core::try_get()
            .map(|core| dcl_core::review::global(core).sessions())
            .unwrap_or(0);
        let mut data = serde_json::json!({
            "reason": self.reason,
            "profile": self.profile,
            "from_profile": self.from_profile,
            "anti_aliasing": setting_int("anti_aliasing"),
            "shadow_quality": setting_int("shadow_quality"),
            "bloom_quality": setting_int("bloom_quality"),
            "async_shaders": super::async_shaders::is_enabled(),
            "samples": self.sample.len(),
            "in_view": in_view,
            "not_drawn": in_view - drawn,
            "ready_at_start_pct": timeline.ready_at_start_pct,
            "t50_ms": timeline.t50_msec,
            "t90_ms": timeline.t90_msec,
            "t100_ms": t100_msec,
            "menu_ms": self.start_msec.unwrap_or(self.triggered_msec) - self.triggered_msec,
            "frames": self.frames,
            "max_frame_ms": self.max_frame_msec,
            "launch_count": launch_count,
            "scene_id": scene_id,
            "position": format!("{},{}", position.x, position.y),
        });
        let after = pipeline_counts();
        for (i, (key, _)) in PIPELINE_COUNTERS.iter().enumerate() {
            data[*key] = (after[i] - self.pipelines_before[i]).into();
        }
        let json = data.to_string();
        godot_print!("[RenderReady] {json}");
        globals::metrics()
            .bind_mut()
            .track_screen_viewed("RENDER_READY".to_string(), json);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_view_only_ends_at_the_timeout() {
        let mut timeline = Timeline::new();
        assert_eq!(timeline.observe(100, 0, 0), Verdict::Pending);
        assert_eq!(timeline.observe(TIMEOUT_MSEC, 0, 0), Verdict::TimedOut);
        assert_eq!(timeline.ready_at_start_pct, 0);
    }

    #[test]
    fn thresholds_are_stamped_once_at_their_first_crossing() {
        let mut timeline = Timeline::new();
        assert_eq!(timeline.observe(10, 10, 2), Verdict::Pending);
        assert_eq!(timeline.observe(20, 10, 5), Verdict::Pending);
        assert_eq!(timeline.observe(30, 10, 9), Verdict::Pending);
        assert_eq!(timeline.observe(40, 10, 10), Verdict::Ready);
        assert_eq!(timeline.observe(50, 10, 10), Verdict::Ready);
        assert_eq!(timeline.ready_at_start_pct, 20);
        assert_eq!((timeline.t50_msec, timeline.t90_msec), (20, 30));
    }

    #[test]
    fn already_drawn_reports_ready_at_start() {
        let mut timeline = Timeline::new();
        assert_eq!(timeline.observe(5, 4, 4), Verdict::Ready);
        assert_eq!(timeline.ready_at_start_pct, 100);
        assert_eq!((timeline.t50_msec, timeline.t90_msec), (5, 5));
    }

    #[test]
    fn the_sample_is_the_nearest_meshes() {
        let candidates: Vec<(f32, i64)> = (0..100).rev().map(|i| (i as f32, i)).collect();
        let ids = nearest_ids(candidates);
        assert_eq!(ids.len(), MAX_SAMPLES);
        assert_eq!(ids.first(), Some(&0));
        assert_eq!(ids.last(), Some(&(MAX_SAMPLES as i64 - 1)));
    }

    #[test]
    fn a_box_behind_a_plane_is_out_of_the_frustum() {
        let planes = [Plane::new(Vector3::new(0.0, 0.0, -1.0), 0.0)];
        let inside = Aabb::new(Vector3::new(-1.0, -1.0, 1.0), Vector3::splat(2.0));
        let outside = Aabb::new(Vector3::new(-1.0, -1.0, -3.0), Vector3::splat(2.0));
        assert!(in_frustum(&planes, inside));
        assert!(!in_frustum(&planes, outside));
    }
}
