//! `DclLightSourceComponent`: the Godot light of one SDK `LightSource`, plus the settings and
//! registry the manager uses to activate, budget and fade every light in the world.

use std::cell::RefCell;

use godot::classes::image::Interpolation;
use godot::classes::light_3d::Param;
use godot::classes::{
    HttpRequest, INode3D, Image, ImageTexture, Light3D, Node3D, OmniLight3D, ResourceLoader,
    SpotLight3D, Texture2D,
};
use godot::global::Error;
use godot::prelude::*;

use super::gizmo::{Gizmo, GizmoView};
use super::logic;

pub const DEFAULT_PROJECTOR_TEXTURE: &str = "res://assets/white_pixel.png";
const PROJECTOR_CACHE_MAX_ENTRIES: usize = 16;
/// Larger masks are shrunk (never enlarged) to bound the shadow map cost on mobile.
const PROJECTOR_MAX_SIZE: i32 = 512;
const SHADOW_CASTER_MASK_ALL: u32 = 0xFFFF_FFFF;
const SHADOW_CASTER_MASK_NONE: u32 = 0;

#[derive(Clone, Copy)]
pub struct Settings {
    pub lights_enabled: bool,
    pub shadows_off: bool,
    pub auto_range: bool,
    pub auto_range_cap: f32,
    pub use_budget: bool,
    pub max_lights: usize,
    pub debug_gizmo: bool,
}

struct Shared {
    settings: Settings,
    registered: Vec<Gd<DclLightSourceComponent>>,
    reference: Option<Vector3>,
    /// Camera position and forward.
    camera: Option<(Vector3, Vector3)>,
    recompute_pending: bool,
    /// Insertion-ordered: the first entry is the oldest and goes first when the cache is full.
    projector_cache: Vec<(String, Gd<Texture2D>)>,
}

thread_local! {
    static SHARED: RefCell<Shared> = const { RefCell::new(Shared {
        settings: Settings {
            lights_enabled: true,
            shadows_off: false,
            auto_range: true,
            auto_range_cap: 40.0,
            use_budget: true,
            max_lights: 4,
            debug_gizmo: false,
        },
        registered: Vec::new(),
        reference: None,
        camera: None,
        recompute_pending: true,
        projector_cache: Vec::new(),
    }) };
}

pub fn settings() -> Settings {
    SHARED.with(|s| s.borrow().settings)
}

fn edit_settings(change: impl FnOnce(&mut Settings)) {
    SHARED.with(|s| change(&mut s.borrow_mut().settings));
    request_recompute();
}

pub fn request_recompute() {
    SHARED.with(|s| s.borrow_mut().recompute_pending = true);
}

pub fn take_recompute_pending() -> bool {
    SHARED.with(|s| std::mem::replace(&mut s.borrow_mut().recompute_pending, false))
}

pub fn is_recompute_pending() -> bool {
    SHARED.with(|s| s.borrow().recompute_pending)
}

/// A snapshot, so lights can be bound one at a time without holding the registry.
pub fn registered_lights() -> Vec<Gd<DclLightSourceComponent>> {
    SHARED.with(|s| s.borrow().registered.clone())
}

pub fn set_view(reference: Option<Vector3>, camera: Option<(Vector3, Vector3)>) {
    SHARED.with(|s| {
        let mut s = s.borrow_mut();
        s.reference = reference;
        s.camera = camera;
    });
}

fn view() -> (Option<Vector3>, Option<(Vector3, Vector3)>) {
    SHARED.with(|s| {
        let s = s.borrow();
        (s.reference, s.camera)
    })
}

fn cached_projector(url: &str) -> Option<Gd<Texture2D>> {
    SHARED.with(|s| {
        s.borrow()
            .projector_cache
            .iter()
            .find(|(key, tex)| key == url && tex.is_instance_valid())
            .map(|(_, tex)| tex.clone())
    })
}

fn cache_projector(url: &str, texture: Gd<Texture2D>) {
    SHARED.with(|s| {
        let cache = &mut s.borrow_mut().projector_cache;
        cache.retain(|(key, _)| key != url);
        cache.push((url.to_string(), texture));
        while cache.len() > PROJECTOR_CACHE_MAX_ENTRIES {
            cache.remove(0);
        }
    });
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Spot,
    Point,
}

/// The last parameters the scene set, in SDK units.
#[derive(Clone, Copy)]
struct Authored {
    kind: Kind,
    intensity: f32,
    range: f32,
    inner_angle: f32,
    outer_angle: f32,
}

#[derive(GodotClass)]
#[class(base=Node3D)]
pub struct DclLightSourceComponent {
    base: Base<Node3D>,
    light: Option<Gd<Light3D>>,
    authored: Option<Authored>,
    projector_texture: Option<Gd<Texture2D>>,
    projector_texture_path: String,
    projector_texture_display_path: String,
    pending_http: Option<Gd<HttpRequest>>,
    /// External runtime limits: a light can be disabled without fighting the range logic.
    #[var]
    runtime_light_enabled: bool,
    #[var]
    runtime_shadows_enabled: bool,
    budget_enabled: bool,
    budget_rank: i32,
    budget_candidate: bool,
    distance_to_reference: f32,
    /// Fades ramp the energy between 0 and `base_energy` instead of toggling visibility.
    base_energy: f32,
    energy_scale: f32,
    render_target: bool,
    fade_active: bool,
    gizmo: Option<Gizmo>,
}

#[godot_api]
impl INode3D for DclLightSourceComponent {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
            light: None,
            authored: None,
            projector_texture: None,
            projector_texture_path: String::new(),
            projector_texture_display_path: String::new(),
            pending_http: None,
            runtime_light_enabled: true,
            runtime_shadows_enabled: true,
            budget_enabled: false,
            budget_rank: -1,
            budget_candidate: false,
            distance_to_reference: -1.0,
            base_energy: 0.0,
            energy_scale: 0.0,
            render_target: false,
            fade_active: false,
            gizmo: None,
        }
    }

    fn ready(&mut self) {
        let me = self.to_gd();
        SHARED.with(|s| {
            let registered = &mut s.borrow_mut().registered;
            if !registered.contains(&me) {
                registered.push(me);
            }
        });
        if self.projector_texture_path.is_empty() {
            self.set_projector(DEFAULT_PROJECTOR_TEXTURE, "");
        }
        if settings().debug_gizmo {
            self.ensure_gizmo();
        }
        request_recompute();
    }

    fn exit_tree(&mut self) {
        let me = self.to_gd();
        SHARED.with(|s| s.borrow_mut().registered.retain(|l| *l != me));
        request_recompute();
    }
}

#[godot_api]
impl DclLightSourceComponent {
    #[func]
    pub fn apply_graphic_profile_settings(
        lights_enabled: bool,
        shadows_enabled: bool,
        max_lights: i64,
        range_cap: f32,
    ) {
        edit_settings(|s| {
            s.lights_enabled = lights_enabled;
            s.shadows_off = !shadows_enabled;
            s.max_lights = max_lights.max(0) as usize;
            s.auto_range_cap = range_cap;
        });
    }

    /// The dev-tools override of the profile's light settings.
    #[func]
    pub fn apply_light_settings(
        lights_enabled: bool,
        shadows_enabled: bool,
        max_lights: i64,
        debug_enabled: bool,
        auto_range_enabled: bool,
        budget_enabled: bool,
    ) {
        edit_settings(|s| {
            s.lights_enabled = lights_enabled;
            s.shadows_off = !shadows_enabled;
            s.max_lights = max_lights.max(0) as usize;
            s.debug_gizmo = debug_enabled;
            s.auto_range = auto_range_enabled;
            s.use_budget = budget_enabled;
        });
        for mut light in registered_lights() {
            light.bind_mut().apply_debug(debug_enabled);
        }
    }

    #[func]
    pub fn get_light_settings() -> VarDictionary {
        let s = settings();
        VarDictionary::from_iter([
            ("lights_enabled".to_variant(), s.lights_enabled.to_variant()),
            (
                "shadows_enabled".to_variant(),
                (!s.shadows_off).to_variant(),
            ),
            (
                "max_lights".to_variant(),
                (s.max_lights as i64).to_variant(),
            ),
            ("debug_enabled".to_variant(), s.debug_gizmo.to_variant()),
            (
                "auto_activation_range".to_variant(),
                s.auto_range.to_variant(),
            ),
            (
                "use_global_light_budget".to_variant(),
                s.use_budget.to_variant(),
            ),
        ])
    }

    #[func]
    pub fn get_max_active_lights() -> i64 {
        settings().max_lights as i64
    }

    /// Whether the light exists and is not authored off; the debug stats count those.
    #[func]
    pub fn emits_light(&self) -> bool {
        self.runtime_light_enabled && self.light.is_some()
    }

    #[func]
    pub fn set_spot(
        &mut self,
        color: Color,
        intensity: f32,
        light_range: f32,
        inner_angle: f32,
        outer_angle: f32,
    ) {
        let existing = self
            .light
            .as_ref()
            .and_then(|l| l.clone().try_cast::<SpotLight3D>().ok());
        let mut spot = match existing {
            Some(spot) => spot,
            None => {
                self.clear_light();
                let mut spot = SpotLight3D::new_alloc();
                spot.set_name("DCL_SpotLight");
                self.base_mut().add_child(&spot);
                self.light = Some(spot.clone().upcast());
                spot
            }
        };
        spot.set_color(color);
        self.base_energy = logic::convert_intensity(intensity);
        self.apply_fade();
        spot.set_param(Param::RANGE, light_range);
        let (angle, attenuation) = logic::spot_angles(inner_angle, outer_angle);
        spot.set_param(Param::SPOT_ANGLE, angle);
        spot.set_param(Param::SPOT_ATTENUATION, attenuation);

        self.authored = Some(Authored {
            kind: Kind::Spot,
            intensity,
            range: light_range,
            inner_angle,
            outer_angle,
        });
        if self.projector_texture.is_none() && !self.projector_texture_path.is_empty() {
            let path = self.projector_texture_path.clone();
            self.set_projector(&path, "");
        }
        self.apply_projector();
        self.refresh_gizmo_geometry();
        request_recompute();
    }

    #[func]
    pub fn set_point(&mut self, color: Color, intensity: f32, light_range: f32) {
        let existing = self
            .light
            .as_ref()
            .and_then(|l| l.clone().try_cast::<OmniLight3D>().ok());
        let mut omni = match existing {
            Some(omni) => omni,
            None => {
                self.clear_light();
                let mut omni = OmniLight3D::new_alloc();
                omni.set_name("DCL_OmniLight");
                self.base_mut().add_child(&omni);
                self.light = Some(omni.clone().upcast());
                omni
            }
        };
        omni.set_color(color);
        self.base_energy = logic::convert_intensity(intensity);
        self.apply_fade();
        omni.set_param(Param::RANGE, light_range);

        self.authored = Some(Authored {
            kind: Kind::Point,
            intensity,
            range: light_range,
            inner_angle: 0.0,
            outer_angle: 0.0,
        });
        self.refresh_gizmo_geometry();
        request_recompute();
    }

    #[func]
    pub fn remove_light(&mut self) {
        self.clear_light();
        if let Some(gizmo) = self.gizmo.as_mut() {
            gizmo.set_visible(false);
        }
    }

    #[func]
    pub fn set_shadow_enabled(&mut self, enabled: bool) {
        self.runtime_shadows_enabled = enabled;
        self.update_rendering();
    }

    #[func]
    pub fn set_light_enabled(&mut self, enabled: bool) {
        self.runtime_light_enabled = enabled;
        request_recompute();
        if let Some(gizmo) = self.gizmo.as_mut() {
            gizmo.set_visible(settings().debug_gizmo);
        }
    }

    #[func]
    pub fn get_projector_texture_path(&self) -> GString {
        self.projector_texture_path.as_str().into()
    }

    #[func]
    pub fn set_projector_texture(&mut self, path: GString, display_path: GString) {
        self.set_projector(&path.to_string(), &display_path.to_string());
    }

    #[func]
    pub fn set_projector_texture_display_path(&mut self, display_path: GString) {
        self.projector_texture_display_path = display_path.to_string();
    }
}

impl DclLightSourceComponent {
    fn clear_light(&mut self) {
        if let Some(mut light) = self.light.take() {
            light.queue_free();
        }
    }

    fn apply_fade(&mut self) {
        if let Some(light) = self.light.as_mut() {
            light.set_visible(self.energy_scale > 0.0);
            light.set_param(Param::ENERGY, self.base_energy * self.energy_scale);
        }
    }

    fn activation_range(&self) -> f32 {
        let Some(a) = self.authored else {
            return 0.0;
        };
        let s = settings();
        logic::activation_range(a.range, a.intensity, s.auto_range, s.auto_range_cap)
    }

    fn global_position(&self) -> Option<Vector3> {
        self.base()
            .is_inside_tree()
            .then(|| self.base().get_global_position())
    }

    /// -1 without a reference or range, 0 outside the activation range, 1 inside.
    fn range_state(&self) -> i32 {
        let (reference, _) = view();
        let distance = match (reference, self.global_position()) {
            (Some(reference), Some(position)) => position.distance_to(reference),
            _ => 0.0,
        };
        logic::avatar_range_state(
            reference.is_some() && self.global_position().is_some(),
            self.activation_range(),
            distance,
        )
    }

    fn in_camera_view(&self) -> bool {
        let (_, camera) = view();
        let Some(position) = self.global_position() else {
            return true;
        };
        let (camera_position, forward) = camera.unwrap_or((Vector3::ZERO, Vector3::FORWARD));
        logic::in_camera_view(
            camera.is_some(),
            camera_position,
            forward,
            position,
            self.activation_range(),
        )
    }

    /// Resets the budget fields and returns the distance to the reference when this light competes
    /// for a budget slot.
    pub fn begin_budget_pass(&mut self) -> Option<f32> {
        self.budget_enabled = false;
        self.budget_rank = -1;
        self.budget_candidate = false;
        self.distance_to_reference = -1.0;
        let competes = self.light.is_some()
            && self.authored.is_some()
            && self.runtime_light_enabled
            && self.range_state() == 1
            && self.in_camera_view();
        if !competes {
            self.update_rendering();
            return None;
        }
        let (reference, _) = view();
        let distance = self.global_position()?.distance_to(reference?);
        self.distance_to_reference = distance;
        self.budget_candidate = true;
        Some(distance)
    }

    pub fn set_budget(&mut self, rank: Option<usize>, enabled: bool) {
        self.budget_enabled = enabled;
        if let Some(rank) = rank {
            self.budget_rank = rank as i32;
        }
        self.update_rendering();
    }

    /// Without a budget every light that passes the range test renders.
    pub fn open_budget(&mut self) {
        self.budget_enabled = true;
        self.budget_rank = -1;
        self.budget_candidate = false;
        self.distance_to_reference = -1.0;
        self.update_rendering();
    }

    fn update_rendering(&mut self) {
        if self.light.is_none() {
            return;
        }
        let s = settings();
        if !s.lights_enabled || !self.runtime_light_enabled {
            self.set_render_state(false);
            return;
        }
        let state = self.range_state();
        if state == -1 {
            self.set_render_state(false);
            return;
        }
        let mut render = state == 1 && self.in_camera_view();
        if s.use_budget {
            render = render && self.budget_enabled;
        }
        self.set_render_state(render);
    }

    fn set_render_state(&mut self, render: bool) {
        let Some(mut light) = self.light.clone() else {
            return;
        };
        if render == self.render_target && !self.fade_active {
            return;
        }
        self.render_target = render;
        // Shadows switch at once: a shadow popping in after the light faded would look worse.
        if !render {
            light.set_shadow(false);
            light.set_shadow_caster_mask(SHADOW_CASTER_MASK_NONE);
        } else {
            // Godot needs shadows on for a projector to work, so projector lights keep them.
            let has_projector = light.clone().try_cast::<SpotLight3D>().is_ok()
                && !self.projector_texture_path.is_empty()
                && self.projector_texture_path != DEFAULT_PROJECTOR_TEXTURE;
            light.set_shadow(self.runtime_shadows_enabled || has_projector);
            light.set_shadow_caster_mask(if settings().shadows_off {
                SHADOW_CASTER_MASK_NONE
            } else {
                SHADOW_CASTER_MASK_ALL
            });
        }
        self.fade_active = true;
    }

    pub fn tick_fade(&mut self, delta: f32) {
        if self.light.is_none() {
            self.fade_active = false;
            return;
        }
        let (scale, active) = logic::fade_step(self.energy_scale, self.render_target, delta);
        self.energy_scale = scale;
        self.fade_active = active;
        self.apply_fade();
    }

    pub fn is_fading(&self) -> bool {
        self.fade_active
    }

    // Projector texture.

    fn set_projector(&mut self, path: &str, display_path: &str) {
        if path == DEFAULT_PROJECTOR_TEXTURE {
            self.projector_texture_display_path.clear();
        } else if !display_path.is_empty() {
            self.projector_texture_display_path = display_path.to_string();
        } else if self.projector_texture_display_path.is_empty() && looks_authored(path) {
            self.projector_texture_display_path = path.to_string();
        }

        if self.projector_texture_path == path
            && (self.projector_texture.is_some() || self.pending_http.is_some())
        {
            return;
        }
        self.projector_texture_path = path.to_string();
        self.projector_texture = None;

        if path.is_empty() {
            if let Some(spot) = self.spot() {
                spot.upcast::<Light3D>().set_projector(Gd::null_arg());
            }
            return;
        }
        if path.starts_with("https://") {
            self.load_projector_http(path);
            return;
        }
        if !ResourceLoader::singleton().exists(path) {
            return;
        }
        self.projector_texture = ResourceLoader::singleton()
            .load(path)
            .and_then(|r| r.try_cast::<Texture2D>().ok());
        self.apply_projector();
    }

    fn spot(&self) -> Option<Gd<SpotLight3D>> {
        self.light
            .as_ref()
            .and_then(|l| l.clone().try_cast::<SpotLight3D>().ok())
    }

    fn apply_projector(&mut self) {
        if let (Some(mut spot), Some(texture)) = (self.spot(), self.projector_texture.as_ref()) {
            spot.set_projector(texture);
        }
    }

    fn load_projector_http(&mut self, url: &str) {
        if let Some(texture) = cached_projector(url) {
            self.projector_texture = Some(texture);
            self.apply_projector();
            return;
        }
        if let Some(mut pending) = self.pending_http.take() {
            pending.queue_free();
        }
        let mut request = HttpRequest::new_alloc();
        self.base_mut().add_child(&request);
        let me = self.to_gd();
        let url_owned = url.to_string();
        let handle = request.clone();
        request
            .signals()
            .request_completed()
            .connect(move |result, code, _headers, body| {
                if me.is_instance_valid() {
                    let mut me = me.clone();
                    me.bind_mut()
                        .on_projector_response(&handle, &url_owned, result, code, &body);
                }
            });
        if request.request(url) == Error::OK {
            self.pending_http = Some(request);
        } else {
            request.queue_free();
        }
    }

    fn on_projector_response(
        &mut self,
        request: &Gd<HttpRequest>,
        url: &str,
        result: i64,
        code: i64,
        body: &PackedByteArray,
    ) {
        if self.pending_http.as_ref() == Some(request) {
            self.pending_http = None;
        }
        request.clone().queue_free();
        if self.projector_texture_path != url {
            return;
        }
        if let Some(texture) = cached_projector(url) {
            self.projector_texture = Some(texture);
            self.apply_projector();
            return;
        }
        if result != 0 || !(200..300).contains(&code) {
            return;
        }
        let Some(texture) = decode_projector(body) else {
            return;
        };
        cache_projector(url, texture.clone());
        self.projector_texture = Some(texture);
        self.apply_projector();
    }

    // Debug gizmo.

    fn ensure_gizmo(&mut self) {
        if self.gizmo.is_none() {
            let mut node = self.to_gd().upcast::<Node3D>();
            self.gizmo = Some(Gizmo::new(&mut node, settings().debug_gizmo));
        }
    }

    fn apply_debug(&mut self, enabled: bool) {
        if enabled {
            self.ensure_gizmo();
            self.refresh_gizmo_geometry();
        }
        if let Some(gizmo) = self.gizmo.as_mut() {
            gizmo.set_visible(enabled);
        }
    }

    fn refresh_gizmo_geometry(&mut self) {
        if !settings().debug_gizmo {
            return;
        }
        self.ensure_gizmo();
        self.refresh_gizmo();
    }

    /// The status dot and labels follow the budget, so the manager calls this after each pass.
    pub fn refresh_gizmo(&mut self) {
        let (Some(authored), true) = (self.authored, self.gizmo.is_some()) else {
            return;
        };
        let s = settings();
        let texture_text = if !self.projector_texture_path.is_empty()
            && self.projector_texture_path != DEFAULT_PROJECTOR_TEXTURE
        {
            let shown = if self.projector_texture_display_path.is_empty() {
                &self.projector_texture_path
            } else {
                &self.projector_texture_display_path
            };
            format!("tex:{}", logic::short_path(shown, 34))
        } else {
            "tex:n".to_string()
        };
        let budget_text = if s.use_budget && self.budget_candidate {
            let mut text = format!("rank:{}", self.budget_rank);
            if self.distance_to_reference >= 0.0 {
                text += &format!(" d:{}", logic::fmtf(self.distance_to_reference));
            }
            text
        } else {
            "budget:-".to_string()
        };
        let gizmo_view = GizmoView {
            spot: authored.kind == Kind::Spot,
            intensity: authored.intensity,
            range: authored.range,
            inner_angle: authored.inner_angle,
            outer_angle: authored.outer_angle,
            activation_range: self.activation_range(),
            in_range: self.range_state() == 1,
            budget_blocked: s.use_budget && !self.budget_enabled,
            texture_text,
            budget_text,
        };
        if let Some(gizmo) = self.gizmo.as_mut() {
            gizmo.update(&gizmo_view);
        }
    }

    pub fn has_gizmo(&self) -> bool {
        self.gizmo.is_some()
    }
}

fn looks_authored(path: &str) -> bool {
    if path.is_empty()
        || path == DEFAULT_PROJECTOR_TEXTURE
        || path.starts_with("https://")
        || path.starts_with("ipfs://")
    {
        return false;
    }
    path.starts_with('/') || path.starts_with("res://") || path.starts_with("user://")
}

fn decode_projector(body: &PackedByteArray) -> Option<Gd<Texture2D>> {
    let mut image = Image::new_gd();
    if image.load_png_from_buffer(body) != Error::OK
        && image.load_jpg_from_buffer(body) != Error::OK
        && image.load_webp_from_buffer(body) != Error::OK
    {
        return None;
    }
    let (w, h) = (image.get_width(), image.get_height());
    if w > PROJECTOR_MAX_SIZE || h > PROJECTOR_MAX_SIZE {
        image
            .resize_ex(w.min(PROJECTOR_MAX_SIZE), h.min(PROJECTOR_MAX_SIZE))
            .interpolation(Interpolation::LANCZOS)
            .done();
    }
    ImageTexture::create_from_image(&image).map(|t| t.upcast())
}
