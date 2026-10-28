//! Quality settings shared by the off-screen renderers (avatar and asset images): nothing there is
//! realtime, so every anti-aliasing lever is pulled.

use godot::classes::viewport::{Msaa, ScreenSpaceAa};
use godot::classes::{RenderingServer, SubViewport};
use godot::prelude::*;

use crate::avatars::preview::AvatarPreview;

/// The extra boost of the LDR framebuffer of the compatibility renderer, which has no rendering
/// device: the same exposure lands about 1.7x dimmer there after the gamma round trip.
const COMPATIBILITY_EXPOSURE_BOOST: f32 = 2.8;
const BASE_EXPOSURE: f32 = 1.4;

pub struct Quality {
    pub msaa: Msaa,
    /// Clamped by the engine to 0.1..2.0.
    pub scaling_3d: f32,
}

/// Configures the preview's viewport and replaces its brightness adjustment: the toon shader's
/// emission floor needs compensating, and `Environment.adjustment_*` is silently dropped by the
/// compatibility renderer (Godot issue #92853), so `tonemap_exposure` does it, scoped to the tool.
pub fn apply(preview: &Gd<AvatarPreview>, quality: Quality) {
    let (viewport, environment) = {
        let preview = preview.bind();
        (
            preview.subviewport.clone(),
            preview.world_environment.clone(),
        )
    };
    if let Some(mut viewport) = viewport {
        configure(&mut viewport, quality);
    }
    RenderingServer::singleton().screen_space_roughness_limiter_set_active(true, 4.0, 1.0);
    let Some(mut world_environment) = environment else {
        return;
    };
    let Some(shared) = world_environment.get_environment() else {
        return;
    };
    let mut copy: Gd<godot::classes::Environment> = shared.duplicate_resource();
    copy.set_adjustment_enabled(false);
    let compatibility = RenderingServer::singleton()
        .get_rendering_device()
        .is_none();
    copy.set_tonemap_exposure(if compatibility {
        BASE_EXPOSURE * COMPATIBILITY_EXPOSURE_BOOST
    } else {
        BASE_EXPOSURE
    });
    world_environment.set_environment(&copy);
}

fn configure(viewport: &mut Gd<SubViewport>, quality: Quality) {
    viewport.set_msaa_3d(quality.msaa);
    viewport.set_scaling_3d_scale(quality.scaling_3d);
    viewport.set_screen_space_aa(ScreenSpaceAa::FXAA);
    viewport.set_mesh_lod_threshold(0.0);
    viewport.set_use_debanding(true);
}
