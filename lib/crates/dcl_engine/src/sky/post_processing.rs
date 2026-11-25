//! Applies scenes' `PBPostProcessing` to the view: the world environment's tonemapper, exposure,
//! adjustments and glow, a generated color correction LUT for the hue and white balance, and the
//! world's camera attributes for the depth of field.
//!
//! The explorer's own values stay the baseline. While settings apply, the environment is read back
//! each frame and a property keeps the explorer's value unless something other than this node
//! changed it (the bloom setting, a new sky); the settings are written over that. When no settings
//! apply, or the node leaves the tree, the baseline is written back.
//!
//! Godot differs from the desktop explorer here: contrast, saturation and the LUT apply after the
//! curve, `TONEMAP_NONE` switches at the middle of a blend instead of fading, and the far blur
//! rises along a smoothstep. Texture cameras don't exist on this client.

use std::cell::Cell;
use std::f32::consts::{FRAC_PI_3, TAU};
use std::sync::OnceLock;

use godot::classes::environment::ToneMapper;
use godot::classes::image::Format;
use godot::classes::{
    Camera3D, CameraAttributes, CameraAttributesPractical, Environment, INode, Image,
    ImageTexture3D, Node, Texture, World3D,
};
use godot::prelude::*;
use serde_json::{json, Value};

use crate::dcl::{components::SceneEntityId, SceneId};
use crate::fetcher::globals;
use crate::godot_classes::dcl_virtual_camera::DclVirtualCamera;
use crate::player::camera_controller::DEFAULT_TRANSITION_TIME;
use crate::scene_runner::components::post_processing::{PostProcessing, TonemapCurve};
use crate::scene_runner::scene_manager::SceneManager;

/// seconds to blend between scenes' settings, as the desktop explorer does
const SCENE_TRANSITION_TIME: f32 = 1.0;
/// Godot's `dof_blur_amount` (a 64 px radius at 1) for the largest blur: the desktop explorer's
/// 20 px circle of confusion.
const MAX_BLUR_AMOUNT: f32 = 10.0 / 64.0;
/// This client has no depth of field setting: an unset intensity is the desktop explorer's Low,
/// about 9 of its 20 px.
const DEFAULT_DOF_INTENSITY: f32 = 0.45;
/// The desktop explorer's focus: this far beyond the player's eyes, at most `FOCUS_MAX`.
const FOCUS_EXTRA: f32 = 50.0;
const FOCUS_MAX: f32 = 100.0;
const EYE_HEIGHT: f32 = 1.81;
const DEFAULT_FOCUS_END: f32 = 250.0;
/// Godot's largest `glow_intensity`
const MAX_GLOW_INTENSITY: f32 = 8.0;
const LUT_SIZE: usize = 32;
const ENCODE_STEPS: usize = 4096;
/// CIE 1931 x of the D65 white point, as the desktop explorer's temperature uses it
const D65_X: f32 = 0.31271;
/// bevy's white balance: the D65 white point in CIE 1931 xy and in LMS, and linear sRGB to LMS
/// (CAM16) and back, as rows
const D65_XY: (f32, f32) = (0.31272, 0.32903);
const D65_LMS: [f32; 3] = [0.975538, 1.01648, 1.08475];
const RGB_TO_LMS: [[f32; 3]; 3] = [
    [0.311692, 0.652085, 0.0362225],
    [0.0905138, 0.901341, 0.00814478],
    [0.00764433, 0.0486554, 0.943700],
];
const LMS_TO_RGB: [[f32; 3]; 3] = [
    [4.06305, -2.93241, -0.130646],
    [-0.40791, 1.40437, 0.00353630],
    [-0.0118812, -0.0486532, 1.0605344],
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Curve {
    /// the explorer's tonemapper and grading
    Explorer,
    Aces,
    /// linear, without the explorer's grading
    None,
}

/// What the view shows over the explorer's look; what blends between sources.
#[derive(Clone, Copy, PartialEq, Debug)]
struct Output {
    curve: Curve,
    /// stops
    exposure: f32,
    contrast: f32,
    saturation: f32,
    /// radians
    hue: f32,
    temperature: f32,
    tint: f32,
    bloom_scale: f32,
    dof: Option<Focus>,
}

/// Far blur: none before `start`, rising to `strength` (a fraction of the largest) at `end`.
#[derive(Clone, Copy, PartialEq, Debug)]
struct Focus {
    start: f32,
    end: f32,
    strength: f32,
}

impl Focus {
    /// Distances blend geometrically, so the focus moves at an even pace from far to near.
    fn lerp(self, other: Self, t: f32) -> Self {
        fn geometric(a: f32, b: f32, t: f32) -> f32 {
            let (a, b) = (a.max(0.01), b.max(0.01));
            a * (b / a).powf(t)
        }
        Self {
            start: geometric(self.start, other.start, t),
            end: geometric(self.end, other.end, t),
            strength: lerp(self.strength, other.strength, t),
        }
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// The output of `settings`; `auto_focus` is where the blur starts when the scene doesn't say.
fn compose(settings: &PostProcessing, auto_focus: f32) -> Output {
    Output {
        curve: match settings.tonemapping {
            Some(TonemapCurve::None) => Curve::None,
            Some(TonemapCurve::Aces) => Curve::Aces,
            Some(TonemapCurve::Default) | None => Curve::Explorer,
        },
        exposure: settings.exposure.unwrap_or(0.0),
        contrast: settings.contrast.unwrap_or(1.0),
        saturation: settings.saturation.unwrap_or(1.0),
        hue: settings.hue_shift.unwrap_or(0.0),
        temperature: settings.temperature.unwrap_or(0.0),
        tint: settings.tint.unwrap_or(0.0),
        bloom_scale: settings.bloom_intensity.unwrap_or(1.0),
        dof: focus(settings, auto_focus),
    }
}

/// The depth of field the settings give, or `None` when they don't set any of it.
fn focus(settings: &PostProcessing, auto_focus: f32) -> Option<Focus> {
    if settings.dof_intensity.is_none()
        && settings.dof_focus_start.is_none()
        && settings.dof_focus_end.is_none()
    {
        return None;
    }
    // distances a scene sets are absolute; an end at or before the start blurs all beyond it
    let start = settings.dof_focus_start.unwrap_or(auto_focus);
    let end = settings
        .dof_focus_end
        .unwrap_or(DEFAULT_FOCUS_END)
        .max(start + 0.1);
    Some(Focus {
        start,
        end,
        strength: settings.dof_intensity.unwrap_or(DEFAULT_DOF_INTENSITY),
    })
}

fn lerp_output(from: &Output, to: &Output, t: f32) -> Output {
    // the explorer has no depth of field of its own: the blur fades in or out where it is
    let dof = match (from.dof, to.dof) {
        (Some(a), Some(b)) => Some(a.lerp(b, t)),
        (Some(a), None) => Some(a.lerp(Focus { strength: 0.0, ..a }, t)),
        (None, Some(b)) => Some(Focus { strength: 0.0, ..b }.lerp(b, t)),
        (None, None) => None,
    };
    Output {
        curve: if t < 0.5 { from.curve } else { to.curve },
        exposure: lerp(from.exposure, to.exposure, t),
        contrast: lerp(from.contrast, to.contrast, t),
        saturation: lerp(from.saturation, to.saturation, t),
        hue: lerp(from.hue, to.hue, t),
        temperature: lerp(from.temperature, to.temperature, t),
        tint: lerp(from.tint, to.tint, t),
        bloom_scale: lerp(from.bloom_scale, to.bloom_scale, t),
        dof,
    }
}

/// The environment properties a scene's settings write.
#[derive(Clone, PartialEq, Debug)]
struct Look {
    tonemapper: ToneMapper,
    exposure: f32,
    adjustment: bool,
    brightness: f32,
    contrast: f32,
    saturation: f32,
    color_correction: Option<Gd<Texture>>,
    glow: bool,
    glow_intensity: f32,
}

impl Look {
    fn read(env: &Gd<Environment>) -> Self {
        Self {
            tonemapper: env.get_tonemapper(),
            exposure: env.get_tonemap_exposure(),
            adjustment: env.is_adjustment_enabled(),
            brightness: env.get_adjustment_brightness(),
            contrast: env.get_adjustment_contrast(),
            saturation: env.get_adjustment_saturation(),
            color_correction: env.get_adjustment_color_correction(),
            glow: env.is_glow_enabled(),
            glow_intensity: env.get_glow_intensity(),
        }
    }

    /// Sets the properties that differ from `current`.
    fn write(&self, env: &mut Gd<Environment>, current: &Look) {
        if self.tonemapper != current.tonemapper {
            env.set_tonemapper(self.tonemapper);
        }
        if self.exposure != current.exposure {
            env.set_tonemap_exposure(self.exposure);
        }
        if self.adjustment != current.adjustment {
            env.set_adjustment_enabled(self.adjustment);
        }
        if self.brightness != current.brightness {
            env.set_adjustment_brightness(self.brightness);
        }
        if self.contrast != current.contrast {
            env.set_adjustment_contrast(self.contrast);
        }
        if self.saturation != current.saturation {
            env.set_adjustment_saturation(self.saturation);
        }
        if self.color_correction != current.color_correction {
            env.set_adjustment_color_correction(self.color_correction.as_ref());
        }
        if self.glow != current.glow {
            env.set_glow_enabled(self.glow);
        }
        if self.glow_intensity != current.glow_intensity {
            env.set_glow_intensity(self.glow_intensity);
        }
    }
}

/// Property by property, the explorer's value: `own` where `current` still holds what was
/// `written` over it, else what is there now.
fn rebase(own: &Look, written: &Look, current: &Look) -> Look {
    fn pick<T: PartialEq + Clone>(own: &T, written: &T, current: &T) -> T {
        if current == written {
            own.clone()
        } else {
            current.clone()
        }
    }
    Look {
        tonemapper: pick(&own.tonemapper, &written.tonemapper, &current.tonemapper),
        exposure: pick(&own.exposure, &written.exposure, &current.exposure),
        adjustment: pick(&own.adjustment, &written.adjustment, &current.adjustment),
        brightness: pick(&own.brightness, &written.brightness, &current.brightness),
        contrast: pick(&own.contrast, &written.contrast, &current.contrast),
        saturation: pick(&own.saturation, &written.saturation, &current.saturation),
        color_correction: pick(
            &own.color_correction,
            &written.color_correction,
            &current.color_correction,
        ),
        glow: pick(&own.glow, &written.glow, &current.glow),
        glow_intensity: pick(
            &own.glow_intensity,
            &written.glow_intensity,
            &current.glow_intensity,
        ),
    }
}

/// The look of `output` over the explorer's `base`; `lut` carries the hue and white balance.
fn look(output: &Output, base: &Look, lut: Option<Gd<Texture>>) -> Look {
    let keep_grading = output.curve != Curve::None;
    let (exposure, adjustment) = if keep_grading {
        (base.exposure, base.adjustment)
    } else {
        (1.0, false)
    };
    let (brightness, contrast, saturation, own_lut) = if adjustment {
        (
            base.brightness,
            base.contrast,
            base.saturation,
            base.color_correction.clone(),
        )
    } else {
        (1.0, 1.0, 1.0, None)
    };
    let contrast = contrast * output.contrast;
    let saturation = saturation * output.saturation;
    let color_correction = lut.or(own_lut);
    // the player's bloom off wins; a scene's 0 turns glow off, which also skips its passes
    let glow = base.glow && output.bloom_scale > 1e-3;
    Look {
        tonemapper: match output.curve {
            Curve::Explorer => base.tonemapper,
            Curve::Aces => ToneMapper::ACES,
            Curve::None => ToneMapper::LINEAR,
        },
        exposure: exposure * output.exposure.exp2(),
        adjustment: adjustment
            || contrast != 1.0
            || saturation != 1.0
            || color_correction.is_some(),
        brightness,
        contrast,
        saturation,
        color_correction,
        glow,
        glow_intensity: if glow {
            (base.glow_intensity * output.bloom_scale).min(MAX_GLOW_INTENSITY)
        } else {
            base.glow_intensity
        },
    }
}

/// bevy's white point offsets (`ColorGradingGlobal::temperature` and `tint`, CIE 1931 xy) for a
/// scene's temperature and tint: temperature moves along the daylight locus, so warm is orange
/// rather than magenta; tint moves off it.
fn white_balance(temperature: f32, tint: f32) -> (f32, f32) {
    fn locus_y(x: f32) -> f32 {
        2.87 * x - 3.0 * x * x - 0.275_095_07
    }
    let t = temperature / 0.65;
    let x = D65_X - t * if t < 0.0 { 0.1 } else { 0.05 };
    let y = locus_y(x) + tint / 0.65 * 0.05;
    (D65_X - x, y - locus_y(D65_X))
}

/// bevy's white balance matrix on linear sRGB: adapts the shifted white point to D65 in LMS.
fn balance_matrix(temperature: f32, tint: f32) -> [[f32; 3]; 3] {
    let (dx, dy) = white_balance(temperature, tint);
    let (x, y) = (D65_XY.0 - dx, D65_XY.1 + dy);
    let white = [
        0.701634 + (-0.051461 + 0.452749 * x) / y,
        1.15856 + (0.045854 - 0.296122 * x) / y,
        -0.904175 + (0.953127 - 0.955206 * x) / y,
    ];
    let scale: [f32; 3] = std::array::from_fn(|i| D65_LMS[i] / white[i]);
    std::array::from_fn(|row| {
        std::array::from_fn(|col| {
            (0..3)
                .map(|k| LMS_TO_RGB[row][k] * scale[k] * RGB_TO_LMS[k][col])
                .sum()
        })
    })
}

/// bevy's hue rotation, through HSV.
fn rotate_hue([r, g, b]: [f32; 3], angle: f32) -> [f32; 3] {
    let max = r.max(g).max(b);
    let chroma = max - r.min(g).min(b);
    if chroma <= 0.0 {
        return [r, g, b];
    }
    let sector = if max == r {
        (g - b) / chroma
    } else if max == g {
        (b - r) / chroma + 2.0
    } else {
        (r - g) / chroma + 4.0
    };
    let hue = (FRAC_PI_3 * sector + angle).rem_euclid(TAU);
    let saturation = chroma / max;
    let channel = |n: f32| {
        let k = (n + hue / FRAC_PI_3) % 6.0;
        max - max * saturation * k.min(4.0 - k).clamp(0.0, 1.0)
    };
    [channel(5.0), channel(3.0), channel(1.0)]
}

fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(l: f32) -> f32 {
    if l <= 0.003_130_8 {
        l * 12.92
    } else {
        1.055 * l.powf(1.0 / 2.4) - 0.055
    }
}

/// Godot samples the LUT at the color itself, without a half-texel inset: texel `i` holds the
/// color at its center, except the first and last, which hold 0 and 1 so black and white stay.
fn lut_coordinate(i: usize) -> f32 {
    match i {
        0 => 0.0,
        i if i == LUT_SIZE - 1 => 1.0,
        i => (i as f32 + 0.5) / LUT_SIZE as f32,
    }
}

/// An sRGB 3D LUT (red along x, green along y, blue across slices) that rotates the hue and then
/// white-balances, in linear light, as bevy grades.
fn lut_bytes(hue: f32, temperature: f32, tint: f32) -> Vec<u8> {
    static ENCODE: OnceLock<Vec<u8>> = OnceLock::new();
    let encode = ENCODE.get_or_init(|| {
        (0..ENCODE_STEPS)
            .map(|i| {
                let srgb = linear_to_srgb(i as f32 / (ENCODE_STEPS - 1) as f32);
                (srgb * 255.0).round() as u8
            })
            .collect()
    });
    let linear: Vec<f32> = (0..LUT_SIZE)
        .map(|i| srgb_to_linear(lut_coordinate(i)))
        .collect();
    let balance = balance_matrix(temperature, tint);
    let mut bytes = Vec::with_capacity(LUT_SIZE * LUT_SIZE * LUT_SIZE * 4);
    for b in 0..LUT_SIZE {
        for g in 0..LUT_SIZE {
            for r in 0..LUT_SIZE {
                let mut color = [linear[r], linear[g], linear[b]];
                if hue != 0.0 {
                    color = rotate_hue(color, hue);
                }
                for row in &balance {
                    let value = row[0] * color[0] + row[1] * color[1] + row[2] * color[2];
                    let index = (value.clamp(0.0, 1.0) * (ENCODE_STEPS - 1) as f32).round();
                    bytes.push(encode[index as usize]);
                }
                bytes.push(255);
            }
        }
    }
    bytes
}

/// The desktop explorer's focus: a little beyond the player's eyes, following them.
fn auto_focus(camera: Option<&Gd<Camera3D>>) -> f32 {
    let (Some(camera), Some(player)) = (camera, crate::world::state::player()) else {
        return FOCUS_MAX;
    };
    let eyes = player.get_global_position() + Vector3::UP * EYE_HEIGHT;
    (camera.get_global_position().distance_to(eyes) + FOCUS_EXTRA).min(FOCUS_MAX)
}

/// The scene whose `engine.CameraEntity` settings apply: the parcel scene the player is in, else
/// the global scene with the lowest hash, as the desktop explorer picks it.
fn source_scene(runner: &SceneManager) -> Option<(SceneId, PostProcessing)> {
    let on_camera = |id: &SceneId| {
        runner
            .get_scene(id)
            .and_then(|scene| scene.post_processing.get(&SceneEntityId::CAMERA).copied())
    };
    let parcel = SceneId(runner.get_current_parcel_scene_id());
    if let Some(found) = on_camera(&parcel) {
        return Some((parcel, found));
    }
    runner
        .get_global_scene_ids()
        .iter()
        .filter_map(|id| {
            let hash = &runner.get_scene(id)?.scene_entity_definition.id;
            Some((*id, on_camera(id)?, hash))
        })
        .min_by(|a, b| a.2.cmp(b.2))
        .map(|(id, found, _)| (id, found))
}

/// The current parcel scene's virtual camera, while it is the active camera and has settings.
fn active_virtual_camera(
    runner: &SceneManager,
) -> Option<(SceneEntityId, PostProcessing, Gd<DclVirtualCamera>)> {
    let scene = runner.get_scene(&SceneId(runner.get_current_parcel_scene_id()))?;
    let camera = scene.virtual_camera.clone();
    let id = camera.bind().get_entity_id();
    if id <= 0 {
        return None;
    }
    let entity = SceneEntityId::from_i32(id);
    // the camera controller only moves to an entity that has a node
    scene.godot_dcl_scene.get_node_or_null_3d(&entity)?;
    Some((entity, *scene.post_processing.get(&entity)?, camera))
}

/// The camera controller's time to move to a virtual camera: its own time, else the distance at
/// its speed; with neither it snaps.
fn transition_seconds(camera: &DclVirtualCamera, distance: Option<f32>) -> f32 {
    let (time, speed) = (camera.get_transition_time(), camera.get_transition_speed());
    if time > 0.0 {
        time
    } else if speed > 0.0 {
        distance.map_or(0.0, |distance| distance / speed)
    } else {
        0.0
    }
}

#[derive(Default)]
struct Blend {
    scene: Option<SceneId>,
    virtual_camera: Option<SceneEntityId>,
    from: Option<Output>,
    last: Option<Output>,
    start: f64,
    duration: f32,
}

struct Lut {
    texture: Gd<ImageTexture3D>,
    key: [f32; 3],
}

#[derive(Clone, Copy, Default)]
struct Reading {
    scene: Option<SceneId>,
    virtual_camera: Option<SceneEntityId>,
    progress: f32,
    settings: PostProcessing,
    focus: Option<Focus>,
}

thread_local! {
    static READING: Cell<Reading> = Cell::new(Reading::default());
}

#[derive(GodotClass)]
#[class(base=Node, init)]
pub struct ScenePostProcessing {
    base: Base<Node>,
    elapsed: f64,
    blend: Blend,
    /// the environment written to, the explorer's look on it and the look written over it
    applied: Option<(Gd<Environment>, Look, Look)>,
    lut: Option<Lut>,
    dof_attributes: Option<Gd<CameraAttributesPractical>>,
    /// while the depth of field is on: the world and the camera attributes it had
    dof_applied: Option<(Gd<World3D>, Option<Gd<CameraAttributes>>)>,
}

#[godot_api]
impl INode for ScenePostProcessing {
    fn process(&mut self, delta: f64) {
        self.elapsed += delta;
        self.tick();
    }

    fn exit_tree(&mut self) {
        self.restore();
    }
}

impl ScenePostProcessing {
    fn tick(&mut self) {
        let Some(viewport) = self.base().get_viewport() else {
            return;
        };
        let Some(world) = viewport.find_world_3d() else {
            return;
        };
        let view = viewport.get_camera_3d();

        let runner = globals::scene_runner();
        let runner = runner.bind();
        let source = source_scene(&runner);
        let layer = active_virtual_camera(&runner);
        let scene = source.map(|(id, _)| id);
        let base_settings = source.map(|(_, found)| found).unwrap_or_default();
        let settings = layer
            .as_ref()
            .map_or(base_settings, |(_, found, _)| found.over(base_settings));
        let virtual_camera = layer.as_ref().map(|(entity, _, _)| *entity);

        if scene != self.blend.scene || virtual_camera != self.blend.virtual_camera {
            self.blend.duration = if scene != self.blend.scene {
                SCENE_TRANSITION_TIME
            } else {
                match &layer {
                    // the camera controller has just started its move from the view's camera
                    Some((entity, _, camera)) => {
                        let target = runner.get_scene_entity_node_or_null_3d(
                            runner.get_current_parcel_scene_id(),
                            entity.as_i32() as u32,
                        );
                        let distance = view.as_ref().zip(target).map(|(view, target)| {
                            view.get_global_position()
                                .distance_to(target.get_global_position())
                        });
                        transition_seconds(&camera.bind(), distance)
                    }
                    None => DEFAULT_TRANSITION_TIME,
                }
            };
            self.blend.from = self.blend.last.take();
            self.blend.start = self.elapsed;
            self.blend.scene = scene;
            self.blend.virtual_camera = virtual_camera;
        }
        drop(runner);

        let progress = if self.blend.duration <= 0.0 {
            1.0
        } else {
            ((self.elapsed - self.blend.start) as f32 / self.blend.duration).clamp(0.0, 1.0)
        };
        let target = compose(&settings, auto_focus(view.as_ref()));
        let output = match (&self.blend.from, progress < 1.0) {
            (Some(from), true) => lerp_output(from, &target, progress),
            _ => target,
        };
        self.blend.last = Some(output);
        READING.set(Reading {
            scene,
            virtual_camera,
            progress,
            settings,
            focus: output.dof,
        });

        if settings != PostProcessing::default() || progress < 1.0 {
            self.apply(&world, &output);
        } else {
            self.restore();
        }
    }

    fn apply(&mut self, world: &Gd<World3D>, output: &Output) {
        if let Some(mut env) = world.get_environment() {
            // another sky's environment: the previous one gets its look back first
            if self
                .applied
                .as_ref()
                .is_some_and(|(applied, _, _)| *applied != env)
            {
                self.restore_environment();
            }
            let current = Look::read(&env);
            let own = match &self.applied {
                Some((_, own, written)) => rebase(own, written, &current),
                None => current.clone(),
            };
            let lut = self.lut_texture(output);
            let written = look(output, &own, lut);
            written.write(&mut env, &current);
            self.applied = Some((env, own, written));
        }
        self.apply_dof(world, output.dof);
    }

    fn lut_texture(&mut self, output: &Output) -> Option<Gd<Texture>> {
        let key = [output.hue, output.temperature, output.tint];
        if key.iter().all(|value| value.abs() < 1e-4) {
            return None;
        }
        let fresh = self
            .lut
            .as_ref()
            .is_some_and(|lut| lut.key.iter().zip(key).all(|(a, b)| (a - b).abs() < 1e-4));
        if !fresh {
            let size = LUT_SIZE as i32;
            let images: Array<Gd<Image>> = lut_bytes(key[0], key[1], key[2])
                .chunks(LUT_SIZE * LUT_SIZE * 4)
                .filter_map(|slice| {
                    Image::create_from_data(
                        size,
                        size,
                        false,
                        Format::RGBA8,
                        &PackedByteArray::from(slice),
                    )
                })
                .collect();
            match &mut self.lut {
                Some(lut) => {
                    lut.texture.update(&images);
                    lut.key = key;
                }
                None => {
                    let mut texture = ImageTexture3D::new_gd();
                    texture.create(Format::RGBA8, size, size, size, false, &images);
                    self.lut = Some(Lut { texture, key });
                }
            }
        }
        self.lut
            .as_ref()
            .map(|lut| lut.texture.clone().upcast::<Texture>())
    }

    fn apply_dof(&mut self, world: &Gd<World3D>, focus: Option<Focus>) {
        let Some(focus) = focus.filter(|focus| focus.strength > 1e-3) else {
            self.restore_dof();
            return;
        };
        if self
            .dof_applied
            .as_ref()
            .is_some_and(|(applied, _)| applied != world)
        {
            self.restore_dof();
        }
        let attributes = self
            .dof_attributes
            .get_or_insert_with(CameraAttributesPractical::new_gd);
        attributes.set_dof_blur_far_enabled(true);
        attributes.set_dof_blur_far_distance(focus.start);
        // kept positive: a negative transition is Godot's physically based blur
        attributes.set_dof_blur_far_transition((focus.end - focus.start).max(0.1));
        attributes.set_dof_blur_amount(focus.strength * MAX_BLUR_AMOUNT);
        let ours = attributes.clone().upcast::<CameraAttributes>();
        let mut world = world.clone();
        let current = world.get_camera_attributes();
        if current.as_ref() != Some(&ours) {
            if self.dof_applied.is_none() {
                self.dof_applied = Some((world.clone(), current));
            }
            world.set_camera_attributes(&ours);
        }
    }

    fn restore(&mut self) {
        self.restore_environment();
        self.restore_dof();
    }

    fn restore_environment(&mut self) {
        if let Some((mut env, own, written)) = self.applied.take() {
            let current = Look::read(&env);
            rebase(&own, &written, &current).write(&mut env, &current);
        }
    }

    fn restore_dof(&mut self) {
        if let Some((mut world, own)) = self.dof_applied.take() {
            world.set_camera_attributes(own.as_ref());
        }
    }
}

/// The debug hub's `post_processing` reading: where the settings come from, the blend, each
/// setting and the depth of field shown this frame.
pub fn debug_reading() -> Value {
    let reading = READING.get();
    let settings = reading.settings;
    json!({
        "scene_id": reading.scene.map(|id| id.0),
        "virtual_camera_entity": reading.virtual_camera.map(|entity| entity.as_i32()),
        "blend": reading.progress,
        "tonemapping": settings.tonemapping.map(|curve| format!("{curve:?}")),
        "exposure": settings.exposure,
        "saturation": settings.saturation,
        "contrast": settings.contrast,
        "hue_shift": settings.hue_shift,
        "temperature": settings.temperature,
        "tint": settings.tint,
        "bloom_intensity": settings.bloom_intensity,
        "dof_intensity": settings.dof_intensity,
        "dof_focus_start": settings.dof_focus_start,
        "dof_focus_end": settings.dof_focus_end,
        "applied_focus": reading.focus.map(|focus| [focus.start, focus.end]),
        "applied_blur": reading.focus.map(|focus| focus.strength),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn explorer_look() -> Look {
        Look {
            tonemapper: ToneMapper::ACES,
            exposure: 1.0,
            adjustment: false,
            brightness: 1.0,
            contrast: 1.0,
            saturation: 1.0,
            color_correction: None,
            glow: true,
            glow_intensity: 1.5,
        }
    }

    #[test]
    fn no_settings_keep_the_explorer_look() {
        let out = compose(&PostProcessing::default(), 60.0);
        assert_eq!(out.curve, Curve::Explorer);
        assert_eq!(out.dof, None);
        let base = explorer_look();
        assert_eq!(look(&out, &base, None), base);
    }

    #[test]
    fn exposure_contrast_and_saturation_scale_the_explorer_values() {
        let settings = PostProcessing {
            exposure: Some(1.0),
            contrast: Some(1.5),
            saturation: Some(0.0),
            ..Default::default()
        };
        let base = Look {
            exposure: 0.8,
            adjustment: true,
            contrast: 1.2,
            ..explorer_look()
        };
        let written = look(&compose(&settings, 60.0), &base, None);
        assert!((written.exposure - 1.6).abs() < 1e-6);
        assert!((written.contrast - 1.8).abs() < 1e-6);
        assert_eq!(written.saturation, 0.0);
        assert!(written.adjustment);
    }

    #[test]
    fn no_curve_is_linear_without_the_explorer_grading() {
        let settings = PostProcessing {
            tonemapping: Some(TonemapCurve::None),
            exposure: Some(1.0),
            ..Default::default()
        };
        let base = Look {
            exposure: 0.8,
            adjustment: true,
            contrast: 1.2,
            ..explorer_look()
        };
        let written = look(&compose(&settings, 60.0), &base, None);
        assert_eq!(written.tonemapper, ToneMapper::LINEAR);
        assert_eq!(written.exposure, 2.0);
        assert_eq!(written.contrast, 1.0);
        assert!(!written.adjustment);
        // bloom stays
        assert!(written.glow);
        let aces = PostProcessing {
            tonemapping: Some(TonemapCurve::Aces),
            ..Default::default()
        };
        let base = Look {
            tonemapper: ToneMapper::FILMIC,
            ..explorer_look()
        };
        assert_eq!(
            look(&compose(&aces, 60.0), &base, None).tonemapper,
            ToneMapper::ACES
        );
    }

    #[test]
    fn bloom_scales_the_glow_and_the_player_off_wins() {
        let bloom = |scale: f32, base: &Look| {
            let settings = PostProcessing {
                bloom_intensity: Some(scale),
                ..Default::default()
            };
            let written = look(&compose(&settings, 60.0), base, None);
            (written.glow, written.glow_intensity)
        };
        let on = explorer_look();
        assert_eq!(bloom(2.0, &on), (true, 3.0));
        assert_eq!(
            bloom(
                4.0,
                &Look {
                    glow_intensity: 3.0,
                    ..on.clone()
                }
            ),
            (true, 8.0)
        );
        // 0 turns glow off and keeps the explorer's intensity for later
        assert_eq!(bloom(0.0, &on), (false, 1.5));
        let off = Look {
            glow: false,
            ..explorer_look()
        };
        assert_eq!(bloom(4.0, &off), (false, 1.5));
    }

    #[test]
    fn rebase_keeps_changes_made_by_others() {
        let own = explorer_look();
        let written = Look {
            exposure: 2.0,
            glow_intensity: 3.0,
            ..own.clone()
        };
        // the bloom setting changed the intensity since this node wrote it
        let current = Look {
            glow_intensity: 0.7,
            ..written.clone()
        };
        let rebased = rebase(&own, &written, &current);
        assert_eq!(rebased.exposure, 1.0);
        assert_eq!(rebased.glow_intensity, 0.7);
    }

    #[test]
    fn focus_distances_are_absolute_and_follow_the_player_when_unset() {
        let at = |start: Option<f32>, end: Option<f32>, intensity: Option<f32>| {
            let settings = PostProcessing {
                dof_focus_start: start,
                dof_focus_end: end,
                dof_intensity: intensity,
                ..Default::default()
            };
            focus(&settings, 55.0).unwrap()
        };
        let near = at(Some(1.0), None, None);
        assert_eq!((near.start, near.end), (1.0, DEFAULT_FOCUS_END));
        assert_eq!(near.strength, DEFAULT_DOF_INTENSITY);
        assert_eq!(at(None, Some(300.0), Some(1.0)).start, 55.0);
        // an end at or before the start blurs everything past the start
        let step = at(Some(6.0), Some(4.0), Some(1.0));
        assert!(
            step.end > step.start && step.end - step.start < 0.2,
            "{step:?}"
        );
        assert_eq!(focus(&PostProcessing::default(), 55.0), None);
    }

    #[test]
    fn depth_of_field_fades_from_none_and_moves_the_focus_evenly() {
        let none = compose(&PostProcessing::default(), 55.0);
        let near = compose(
            &PostProcessing {
                dof_intensity: Some(1.0),
                dof_focus_start: Some(5.0),
                dof_focus_end: Some(20.0),
                ..Default::default()
            },
            55.0,
        );
        let far = compose(
            &PostProcessing {
                dof_intensity: Some(0.5),
                dof_focus_start: Some(80.0),
                dof_focus_end: Some(320.0),
                ..Default::default()
            },
            55.0,
        );
        let fading_in = lerp_output(&none, &near, 0.5).dof.unwrap();
        assert_eq!((fading_in.start, fading_in.end), (5.0, 20.0));
        assert!((fading_in.strength - 0.5).abs() < 1e-6);
        let half = lerp_output(&near, &far, 0.5).dof.unwrap();
        assert!((half.start - 20.0).abs() < 1e-3, "{half:?}");
        assert!((half.end - 80.0).abs() < 1e-3, "{half:?}");
        assert!((half.strength - 0.75).abs() < 1e-6);
    }

    #[test]
    fn the_curve_switches_at_the_middle_of_a_blend() {
        let from = compose(&PostProcessing::default(), 55.0);
        let to = compose(
            &PostProcessing {
                tonemapping: Some(TonemapCurve::None),
                exposure: Some(2.0),
                ..Default::default()
            },
            55.0,
        );
        let early = lerp_output(&from, &to, 0.25);
        assert_eq!(early.curve, Curve::Explorer);
        assert_eq!(early.exposure, 0.5);
        assert_eq!(lerp_output(&from, &to, 0.5).curve, Curve::None);
    }

    #[test]
    fn white_balance_follows_the_daylight_locus() {
        assert_eq!(white_balance(0.0, 0.0), (0.0, 0.0));
        let (x, y) = white_balance(1.0, 0.0);
        assert!(x > 0.0 && y < 0.0, "{x} {y}");
        let (x, y) = white_balance(-1.0, 0.0);
        assert!(x < 0.0 && y > 0.0, "{x} {y}");
        let (x, y) = white_balance(0.0, 1.0);
        assert!(x == 0.0 && y > 0.0, "{x} {y}");

        let identity = balance_matrix(0.0, 0.0);
        for (row, values) in identity.iter().enumerate() {
            for (col, value) in values.iter().enumerate() {
                let expected = if row == col { 1.0 } else { 0.0 };
                assert!((value - expected).abs() < 1e-3, "{identity:?}");
            }
        }
        // warm: grey turns redder and less blue
        let warm = balance_matrix(1.0, 0.0);
        let grey: Vec<f32> = warm
            .iter()
            .map(|row| row.iter().sum::<f32>() * 0.5)
            .collect();
        assert!(grey[0] > grey[2], "{grey:?}");
    }

    #[test]
    fn hue_rotates_through_hsv() {
        let rotated = rotate_hue([1.0, 0.0, 0.0], TAU / 3.0);
        assert!(
            rotated[0].abs() < 1e-5 && (rotated[1] - 1.0).abs() < 1e-5 && rotated[2].abs() < 1e-5
        );
        let back = rotate_hue([0.2, 0.6, 0.4], -1.0);
        let again = rotate_hue(back, 1.0);
        for (a, b) in again.iter().zip([0.2, 0.6, 0.4]) {
            assert!((a - b).abs() < 1e-5, "{again:?}");
        }
        assert_eq!(rotate_hue([0.5, 0.5, 0.5], 1.0), [0.5, 0.5, 0.5]);
    }

    #[test]
    fn an_identity_lut_keeps_every_color_and_black_and_white() {
        let bytes = lut_bytes(0.0, 0.0, 0.0);
        assert_eq!(bytes.len(), LUT_SIZE * LUT_SIZE * LUT_SIZE * 4);
        let texel = |r: usize, g: usize, b: usize| {
            let i = ((b * LUT_SIZE + g) * LUT_SIZE + r) * 4;
            [bytes[i], bytes[i + 1], bytes[i + 2]]
        };
        assert_eq!(texel(0, 0, 0), [0, 0, 0]);
        let last = LUT_SIZE - 1;
        assert_eq!(texel(last, last, last), [255, 255, 255]);
        for i in 0..LUT_SIZE {
            let expected = (lut_coordinate(i) * 255.0).round() as i32;
            let [r, g, b] = texel(i, last - i, i);
            assert!((r as i32 - expected).abs() <= 1, "{i}: {r} vs {expected}");
            assert!((b as i32 - expected).abs() <= 1, "{i}: {b} vs {expected}");
            let expected_g = (lut_coordinate(last - i) * 255.0).round() as i32;
            assert!(
                (g as i32 - expected_g).abs() <= 1,
                "{i}: {g} vs {expected_g}"
            );
        }
    }
}
