//! The hardware benchmark that picks the first graphics profile: it renders a synthetic scene for
//! a fixed number of frames, measures the render time and maps it, with the device's RAM, to the
//! highest profile whose requirements both are met.

use godot::classes::base_material_3d::Transparency;
use godot::classes::environment::{AmbientSource, BgMode};
use godot::classes::geometry_instance_3d::ShadowCastingSetting;
use godot::classes::sub_viewport::UpdateMode;
use godot::classes::{
    BoxMesh, Camera3D, CylinderMesh, DirectionalLight3D, Environment, INode, Mesh, MeshInstance3D,
    OmniLight3D, Os, ProceduralSkyMaterial, RenderingServer, Sky, SphereMesh, StandardMaterial3D,
    SubViewport, Time, WorldEnvironment,
};
use godot::prelude::*;

use crate::engine::local_task::{next_frame, spawn_local};
use crate::godot_classes::dcl_android_plugin::DclAndroidPlugin;
use crate::godot_classes::dcl_ios_plugin::DclIosPlugin;

/// What a profile needs: `gpu_max_ms` is the render time in milliseconds (lower is better).
/// Calibrated on an A54 (Mali-G68): a benchmark score of about 12 ms was about 47 ms in Genesis
/// Plaza, roughly 4x. Targeting 30 FPS sustained, 33 ms in game, means 8 ms for Low.
#[derive(Clone, Copy, Debug)]
pub struct Threshold {
    pub gpu_max_ms: f32,
    pub ram_min_gb: f32,
    pub name: &'static str,
}

/// Very Low, Low, Medium, High. Very Low is the fallback when nothing else qualifies; Medium
/// needs real GPU headroom (at 4 ms or less the game stays under 17 ms: 60 FPS).
pub const THRESHOLDS: [Threshold; 4] = [
    Threshold {
        gpu_max_ms: f32::INFINITY,
        ram_min_gb: 0.0,
        name: "Very Low",
    },
    Threshold {
        gpu_max_ms: 8.0,
        ram_min_gb: 2.0,
        name: "Low",
    },
    Threshold {
        gpu_max_ms: 4.0,
        ram_min_gb: 4.0,
        name: "Medium",
    },
    Threshold {
        gpu_max_ms: 2.0,
        ram_min_gb: 6.0,
        name: "High",
    },
];

const FRAMES_TO_RENDER: usize = 60;
const WARMUP_FRAMES: usize = 10;
/// A 10 x 4 x 10 grid of meshes.
const GRID_SIZE: i32 = 10;
const GRID_HEIGHT: i32 = 4;
const MESHES_PER_BATCH: i32 = 20;
const FALLBACK_RAM_GB: f32 = 4.0;
const MIN_PLAUSIBLE_RAM_BYTES: i64 = 536_870_912;

/// The highest profile whose GPU and RAM requirements are both met. A pure function so the Sentry
/// seeder can re-derive a device tier from persisted scores.
pub fn determine_profile(gpu_score: f32, ram_gb: f32) -> usize {
    THRESHOLDS
        .iter()
        .rposition(|t| gpu_score <= t.gpu_max_ms && ram_gb >= t.ram_min_gb)
        .unwrap_or(0)
}

/// The mean frame time; a very high score (low performance) when nothing was measured.
pub fn average_frame_time(frames: &[f32]) -> f32 {
    if frames.is_empty() {
        return 999.0;
    }
    frames.iter().sum::<f32>() / frames.len() as f32
}

#[derive(GodotClass)]
#[class(base=Node, init)]
pub struct HardwareBenchmark {
    base: Base<Node>,
    running: bool,
    frame_times: Vec<f32>,
    frames_rendered: usize,
    viewport: Option<Gd<SubViewport>>,
    started_ms: u64,
}

#[godot_api]
impl INode for HardwareBenchmark {
    fn ready(&mut self) {
        self.base_mut().set_process(false);
    }

    fn process(&mut self, _delta: f64) {
        let Some(viewport) = self.viewport.as_ref().filter(|_| self.running) else {
            return;
        };
        let rid = viewport.get_viewport_rid();
        let server = RenderingServer::singleton();
        let cpu = server.viewport_get_measured_render_time_cpu(rid) as f32;
        let gpu = server.viewport_get_measured_render_time_gpu(rid) as f32;
        // The first frames are shader-compilation warm-up.
        if self.frames_rendered >= WARMUP_FRAMES {
            self.frame_times.push(cpu.max(gpu));
        }
        self.frames_rendered += 1;
        if self.frames_rendered >= FRAMES_TO_RENDER + WARMUP_FRAMES {
            self.finish();
        }
    }

    fn exit_tree(&mut self) {
        self.cleanup();
    }
}

#[godot_api]
impl HardwareBenchmark {
    #[signal]
    pub fn benchmark_completed(profile: i64, gpu_score: f64, ram_gb: f64);

    #[func]
    pub fn run_benchmark(&mut self) {
        if self.running {
            return;
        }
        self.started_ms = Time::singleton().get_ticks_msec();
        godot_print!(
            "[Startup] hardware_benchmark.run_benchmark start: {}ms",
            self.started_ms
        );
        self.running = true;
        self.frame_times.clear();
        self.frames_rendered = 0;

        let mut viewport = SubViewport::new_alloc();
        viewport.set_size(Vector2i::new(1280, 720));
        viewport.set_update_mode(UpdateMode::ALWAYS);
        viewport.set_disable_3d(false);
        // Its own world, isolated from the main scene.
        viewport.set_use_own_world_3d(true);
        self.base_mut().add_child(&viewport);
        self.viewport = Some(viewport.clone());

        let me = self.to_gd();
        spawn_local(async move {
            build_scene(viewport).await;
            let mut me = me;
            if !me.is_instance_valid() {
                return;
            }
            me.bind_mut().begin_measuring();
        });
    }
}

impl HardwareBenchmark {
    /// Starts counting once the scene is built. A cancel meanwhile drops the viewport.
    fn begin_measuring(&mut self) {
        let Some(viewport) = self.viewport.as_ref() else {
            return;
        };
        godot_print!(
            "[Startup] hardware_benchmark scene created: {}ms",
            Time::singleton().get_ticks_msec() - self.started_ms
        );
        RenderingServer::singleton()
            .viewport_set_measure_render_time(viewport.get_viewport_rid(), true);
        self.base_mut().set_process(true);
        godot_print!("[HardwareBenchmark] Starting benchmark...");
    }

    fn finish(&mut self) {
        self.base_mut().set_process(false);
        self.running = false;
        let now = Time::singleton().get_ticks_msec();
        godot_print!(
            "[Startup] hardware_benchmark._finish_benchmark: {}ms (duration: {}ms)",
            now,
            now - self.started_ms
        );
        let gpu = average_frame_time(&self.frame_times);
        let ram = system_ram_gb();
        let profile = determine_profile(gpu, ram);
        godot_print!(
            "[HardwareBenchmark] Complete: GPU={gpu:.1}ms, RAM={ram:.1}GB -> Profile={profile} ({})",
            THRESHOLDS[profile].name
        );
        self.cleanup();
        self.base_mut().emit_signal(
            "benchmark_completed",
            &[
                (profile as i64).to_variant(),
                (gpu as f64).to_variant(),
                (ram as f64).to_variant(),
            ],
        );
    }

    fn cleanup(&mut self) {
        if let Some(mut viewport) = self.viewport.take() {
            viewport.queue_free();
        }
    }
}

/// Total RAM: the mobile plugins know best, then the OS, then an assumed 4 GB.
fn system_ram_gb() -> f32 {
    let plugin_mb = if DclAndroidPlugin::is_available() {
        DclAndroidPlugin::get_total_ram_mb()
    } else if DclIosPlugin::is_available() {
        DclIosPlugin::get_total_ram_mb()
    } else {
        -1
    };
    if plugin_mb > 0 {
        return plugin_mb as f32 / 1024.0;
    }
    let physical = Os::singleton()
        .get_memory_info()
        .get("physical")
        .and_then(|v| v.try_to::<i64>().ok())
        .unwrap_or(0);
    if physical > MIN_PLAUSIBLE_RAM_BYTES {
        physical as f32 / 1_073_741_824.0
    } else {
        FALLBACK_RAM_GB
    }
}

fn material(color: Color, metallic: f32, roughness: f32) -> Gd<StandardMaterial3D> {
    let mut m = StandardMaterial3D::new_gd();
    m.set_albedo(color);
    m.set_metallic(metallic);
    m.set_roughness(roughness);
    m
}

/// The synthetic scene: shadowed lights, an environment with bloom, and a grid of meshes over four
/// material paths (metallic, rough, emissive, transparent) for shader coverage. It is built over
/// many frames so the UI does not freeze, and a cancel meanwhile frees the viewport under it.
async fn build_scene(mut viewport: Gd<SubViewport>) {
    let mut camera = Camera3D::new_alloc();
    camera.set_position(Vector3::new(0.0, 8.0, 20.0));
    viewport.add_child(&camera);
    camera.look_at(Vector3::ZERO);

    let mut sun = DirectionalLight3D::new_alloc();
    sun.set_position(Vector3::new(10.0, 20.0, 10.0));
    sun.set_shadow(true);
    sun.set_param(godot::classes::light_3d::Param::ENERGY, 1.2);
    sun.set_param(godot::classes::light_3d::Param::SHADOW_BIAS, 0.02);
    viewport.add_child(&sun);
    sun.look_at(Vector3::ZERO);

    let mut fill = DirectionalLight3D::new_alloc();
    fill.set_position(Vector3::new(-10.0, 5.0, -5.0));
    fill.set_param(godot::classes::light_3d::Param::ENERGY, 0.3);
    viewport.add_child(&fill);
    fill.look_at(Vector3::ZERO);

    for i in 0..4 {
        let angle = i as f32 * std::f32::consts::FRAC_PI_2;
        let mut light = OmniLight3D::new_alloc();
        light.set_position(Vector3::new(angle.cos() * 8.0, 3.0, angle.sin() * 8.0));
        light.set_param(godot::classes::light_3d::Param::ENERGY, 2.0);
        light.set_param(godot::classes::light_3d::Param::RANGE, 10.0);
        light.set_shadow(true);
        viewport.add_child(&light);
    }

    let mut sky_material = ProceduralSkyMaterial::new_gd();
    sky_material.set_sky_top_color(Color::from_rgb(0.3, 0.5, 0.8));
    sky_material.set_sky_horizon_color(Color::from_rgb(0.6, 0.7, 0.9));
    sky_material.set_ground_bottom_color(Color::from_rgb(0.2, 0.2, 0.2));
    let mut sky = Sky::new_gd();
    sky.set_material(&sky_material);
    let mut environment = Environment::new_gd();
    environment.set_background(BgMode::SKY);
    environment.set_sky(&sky);
    environment.set_ambient_source(AmbientSource::SKY);
    environment.set_ambient_light_energy(0.5);
    environment.set_glow_enabled(true);
    environment.set_glow_intensity(0.8);
    environment.set_glow_bloom(0.3);
    let mut world_env = WorldEnvironment::new_alloc();
    world_env.set_environment(&environment);
    viewport.add_child(&world_env);
    next_frame().await;

    let mut box_mesh = BoxMesh::new_gd();
    box_mesh.set_size(Vector3::splat(0.8));
    let mut sphere = SphereMesh::new_gd();
    sphere.set_radius(0.5);
    sphere.set_height(1.0);
    sphere.set_radial_segments(32);
    sphere.set_rings(16);
    let mut cylinder = CylinderMesh::new_gd();
    cylinder.set_top_radius(0.4);
    cylinder.set_bottom_radius(0.4);
    cylinder.set_height(1.0);
    let meshes: [Gd<Mesh>; 3] = [box_mesh.upcast(), sphere.upcast(), cylinder.upcast()];

    let mut emissive = material(Color::from_rgb(0.2, 0.8, 0.3), 0.0, 1.0);
    emissive.set_feature(godot::classes::base_material_3d::Feature::EMISSION, true);
    emissive.set_emission(Color::from_rgb(0.2, 0.8, 0.3));
    emissive.set_emission_energy_multiplier(2.0);
    let mut transparent = material(Color::from_rgba(0.8, 0.8, 0.2, 0.5), 0.5, 0.3);
    transparent.set_transparency(Transparency::ALPHA);
    let materials = [
        material(Color::from_rgb(0.9, 0.2, 0.2), 0.9, 0.1),
        material(Color::from_rgb(0.2, 0.4, 0.9), 0.1, 0.8),
        emissive,
        transparent,
    ];

    let half = GRID_SIZE / 2;
    let mut count = 0usize;
    for x in -half..half {
        for y in 0..GRID_HEIGHT {
            for z in -half..half {
                if !viewport.is_instance_valid() {
                    return;
                }
                let mut instance = MeshInstance3D::new_alloc();
                instance.set_mesh(&meshes[count % meshes.len()]);
                instance.set_position(Vector3::new(x as f32 * 1.5, y as f32 * 1.5, z as f32 * 1.5));
                instance.set_cast_shadows_setting(ShadowCastingSetting::ON);
                instance.set_material_override(&materials[count % materials.len()]);
                viewport.add_child(&instance);
                count += 1;
                if count as i32 % MESHES_PER_BATCH == 0 {
                    next_frame().await;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_strong_device_gets_high() {
        assert_eq!(determine_profile(1.5, 8.0), 3);
    }

    #[test]
    fn every_requirement_must_hold() {
        // Fast GPU, little RAM: the RAM floor of Medium and High is not met.
        assert_eq!(determine_profile(1.0, 3.0), 1);
        // Plenty of RAM, slow GPU.
        assert_eq!(determine_profile(9.0, 8.0), 0);
        assert_eq!(determine_profile(3.0, 5.0), 2);
    }

    #[test]
    fn nothing_measured_is_the_slowest_score() {
        assert_eq!(average_frame_time(&[]), 999.0);
        assert_eq!(determine_profile(average_frame_time(&[]), 8.0), 0);
        assert_eq!(average_frame_time(&[2.0, 4.0]), 3.0);
    }

    #[test]
    fn thresholds_tighten_with_each_profile() {
        for pair in THRESHOLDS.windows(2) {
            assert!(pair[0].gpu_max_ms > pair[1].gpu_max_ms);
            assert!(pair[0].ram_min_gb < pair[1].ram_min_gb);
        }
    }
}
