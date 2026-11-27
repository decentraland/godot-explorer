//! `PBPostProcessing`: the image effects a scene sets on `engine.CameraEntity` or on a virtual
//! camera. `sky::post_processing` applies them to the view.

use std::f32::consts::PI;

use crate::{
    dcl::{
        components::{
            proto_components::sdk::components::{PbPostProcessing, Tonemapping},
            SceneComponentId,
        },
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
    },
    scene_runner::scene::Scene,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TonemapCurve {
    /// the explorer's own curve
    #[default]
    Default,
    /// no curve and none of the explorer's grading
    None,
    Aces,
}

/// A scene's `PBPostProcessing`, with non-finite values dropped and the rest clamped to the
/// documented ranges. `None` keeps the explorer's default.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PostProcessing {
    pub tonemapping: Option<TonemapCurve>,
    /// stops
    pub exposure: Option<f32>,
    pub saturation: Option<f32>,
    pub contrast: Option<f32>,
    /// radians
    pub hue_shift: Option<f32>,
    pub temperature: Option<f32>,
    pub tint: Option<f32>,
    pub bloom_intensity: Option<f32>,
    pub dof_intensity: Option<f32>,
    /// meters
    pub dof_focus_start: Option<f32>,
    /// meters
    pub dof_focus_end: Option<f32>,
}

impl PostProcessing {
    /// Each field of `self` that is set, else the field of `base`.
    pub fn over(self, base: PostProcessing) -> PostProcessing {
        PostProcessing {
            tonemapping: self.tonemapping.or(base.tonemapping),
            exposure: self.exposure.or(base.exposure),
            saturation: self.saturation.or(base.saturation),
            contrast: self.contrast.or(base.contrast),
            hue_shift: self.hue_shift.or(base.hue_shift),
            temperature: self.temperature.or(base.temperature),
            tint: self.tint.or(base.tint),
            bloom_intensity: self.bloom_intensity.or(base.bloom_intensity),
            dof_intensity: self.dof_intensity.or(base.dof_intensity),
            dof_focus_start: self.dof_focus_start.or(base.dof_focus_start),
            dof_focus_end: self.dof_focus_end.or(base.dof_focus_end),
        }
    }
}

/// furthest focus distance a scene can set, in meters
pub const MAX_FOCUS_DISTANCE: f32 = 10_000.0;

fn clamped(value: Option<f32>, min: f32, max: f32) -> Option<f32> {
    value.filter(|v| v.is_finite()).map(|v| v.clamp(min, max))
}

impl From<&PbPostProcessing> for PostProcessing {
    fn from(value: &PbPostProcessing) -> Self {
        // the accessor reads an unknown curve as the default
        let tonemapping = value.tonemapping.map(|_| match value.tonemapping() {
            Tonemapping::TonemapDefault => TonemapCurve::Default,
            Tonemapping::TonemapNone => TonemapCurve::None,
            Tonemapping::TonemapAces => TonemapCurve::Aces,
        });
        let grading = value.color_grading.clone().unwrap_or_default();
        let dof = value.depth_of_field.clone().unwrap_or_default();
        Self {
            tonemapping,
            exposure: clamped(value.exposure, -5.0, 5.0),
            saturation: clamped(grading.saturation, 0.0, 2.0),
            contrast: clamped(grading.contrast, 0.0, 2.0),
            hue_shift: clamped(grading.hue_shift, -PI, PI),
            temperature: clamped(grading.temperature, -1.0, 1.0),
            tint: clamped(grading.tint, -1.0, 1.0),
            bloom_intensity: clamped(value.bloom_intensity, 0.0, 4.0),
            dof_intensity: clamped(dof.intensity, 0.0, 1.0),
            dof_focus_start: clamped(dof.focus_start, 0.0, MAX_FOCUS_DISTANCE),
            dof_focus_end: clamped(dof.focus_end, 0.0, MAX_FOCUS_DISTANCE),
        }
    }
}

/// Keeps `scene.post_processing` in step with the component: one entry per entity that has it.
pub fn update_post_processing(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    // a dead entity's components are dropped without marking them dirty
    for entity in &scene.current_dirty.entities.died {
        scene.post_processing.remove(entity);
    }
    let Some(dirty) = scene
        .current_dirty
        .lww_components
        .get(&SceneComponentId::POST_PROCESSING)
    else {
        return;
    };
    let component = SceneCrdtStateProtoComponents::get_post_processing(crdt_state);
    for entity in dirty {
        match component.get(entity).and_then(|entry| entry.value.as_ref()) {
            Some(value) => {
                scene.post_processing.insert(*entity, value.into());
            }
            None => {
                scene.post_processing.remove(entity);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::dcl::components::proto_components::sdk::components::{ColorGrading, DepthOfField};

    use super::*;

    #[test]
    fn clamps_ranges_and_drops_non_finite_values() {
        let pp = PostProcessing::from(&PbPostProcessing {
            tonemapping: Some(Tonemapping::TonemapNone as i32),
            exposure: Some(9.0),
            color_grading: Some(ColorGrading {
                saturation: Some(f32::NAN),
                contrast: Some(-1.0),
                hue_shift: Some(4.0),
                temperature: None,
                tint: Some(f32::INFINITY),
            }),
            bloom_intensity: Some(10.0),
            depth_of_field: Some(DepthOfField {
                intensity: Some(0.5),
                focus_start: Some(-3.0),
                focus_end: None,
            }),
        });
        assert_eq!(pp.tonemapping, Some(TonemapCurve::None));
        assert_eq!(pp.exposure, Some(5.0));
        assert_eq!(pp.saturation, None);
        assert_eq!(pp.contrast, Some(0.0));
        assert_eq!(pp.hue_shift, Some(PI));
        assert_eq!(pp.temperature, None);
        assert_eq!(pp.tint, None);
        assert_eq!(pp.bloom_intensity, Some(4.0));
        assert_eq!(pp.dof_intensity, Some(0.5));
        assert_eq!(pp.dof_focus_start, Some(0.0));
        assert_eq!(pp.dof_focus_end, None);
    }

    #[test]
    fn unknown_curve_keeps_the_default() {
        let pp = PostProcessing::from(&PbPostProcessing {
            tonemapping: Some(99),
            ..Default::default()
        });
        assert_eq!(pp.tonemapping, Some(TonemapCurve::Default));
    }

    #[test]
    fn override_fields_fall_through() {
        let base = PostProcessing {
            exposure: Some(1.0),
            saturation: Some(0.5),
            ..Default::default()
        };
        let layer = PostProcessing {
            exposure: Some(-1.0),
            ..Default::default()
        };
        let merged = layer.over(base);
        assert_eq!(merged.exposure, Some(-1.0));
        assert_eq!(merged.saturation, Some(0.5));
    }
}
