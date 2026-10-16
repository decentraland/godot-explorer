//! The camera framing of the avatar preview: which part of the avatar a category focuses on,
//! and the orthographic size and height that fit it into the visible part of the viewport.
//! Pure math, so the framing is tested without a scene.

use std::collections::HashMap;

use godot::prelude::*;

pub const MIN_CAMERA_SIZE_OVERALL: f32 = 2.0;
pub const MIN_CAMERA_SIZE_PART: f32 = 0.8;
pub const MAX_CAMERA_SIZE: f32 = 5.0;

/// Regions of the avatar, by name: "overall", "head", "torso", "legs", "feet", "hands",
/// "head_base", "head_base_facial", "body_base" and the `*_base` parts.
pub type Aabbs = HashMap<String, Aabb>;

/// The camera focus a wearable category selects.
pub fn focus_for_category(category: &str) -> &'static str {
    match category {
        "facial_hair" => "head_base_facial",
        "eyes" | "eyebrows" | "mouth" => "head_base",
        "hair" | "eyewear" | "tiara" | "facial" | "hat" | "earring" | "mask" | "helmet"
        | "top_head" | "all_extras" => "head",
        "upper_body" => "torso",
        "hands_wear" | "hands" => "hands",
        "lower_body" => "legs",
        "feet" => "feet",
        _ => "overall",
    }
}

pub fn focus_padding(focus: &str) -> f32 {
    match focus {
        "hands" => 0.3,
        "torso" => 0.2,
        "head" | "head_base" | "head_base_facial" => 0.1,
        _ => 0.0,
    }
}

pub fn focus_extra_margin(focus: &str) -> i32 {
    match focus {
        "hands" => 80,
        "feet" | "head" | "head_base" | "head_base_facial" | "torso" => 40,
        _ => 0,
    }
}

pub fn focus_aabb_center_y(focus: &str, aabb: Aabb) -> f32 {
    if matches!(focus, "head" | "head_base" | "head_base_facial") {
        aabb.position.y + aabb.size.y * 0.3
    } else {
        aabb.center().y
    }
}

/// The region a mesh belongs to, from its node name, empty for none.
pub fn mesh_category(mesh_name: &str) -> &'static str {
    let has = |needle: &str| mesh_name.contains(needle);
    if has("__hair")
        || has("__facial_hair")
        || has("__eyewear")
        || has("__earring")
        || has("__tiara")
        || has("__helmet")
        || has("__mask")
        || has("__top_head")
        || has("_head_basemesh")
        || has("_mask_")
    {
        "head"
    } else if has("__upper_body") || has("_ubody_basemesh") {
        "torso"
    } else if has("__lower_body") || has("_lbody_basemesh") {
        "legs"
    } else if has("__feet") || has("_feet_basemesh") {
        "feet"
    } else {
        ""
    }
}

/// Mirrors the box around the avatar's axis so a lopsided pose does not shift the framing.
pub fn symmetrize_aabb_x(aabb: Aabb) -> Aabb {
    let left = -aabb.position.x;
    let right = aabb.position.x + aabb.size.x;
    let half = left.max(right);
    Aabb::new(
        Vector3::new(-half, aabb.position.y, aabb.position.z),
        Vector3::new(half * 2.0, aabb.size.y, aabb.size.z),
    )
}

pub fn aabb_endpoint(aabb: Aabb, index: u8) -> Vector3 {
    let end = aabb.position + aabb.size;
    Vector3::new(
        if index & 1 == 0 {
            aabb.position.x
        } else {
            end.x
        },
        if index & 2 == 0 {
            aabb.position.y
        } else {
            end.y
        },
        if index & 4 == 0 {
            aabb.position.z
        } else {
            end.z
        },
    )
}

/// The visible part of the viewport: its size, the side margins, and the top and bottom margins
/// already including the nodes the host laid over it.
#[derive(Clone, Copy, Debug)]
pub struct FitView {
    pub size: Vector2,
    pub margin_left: f32,
    pub margin_right: f32,
    pub top: f32,
    pub bottom: f32,
}

/// Where the camera goes: its orthographic size and the height it looks at.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraTarget {
    pub size: f32,
    pub center_y: f32,
}

#[derive(Debug, PartialEq)]
pub enum OverallFit {
    /// Nothing measured yet.
    NoBody,
    /// The layout has not settled (a zero-size viewport or margins eating it): try again.
    Deferred,
    Fit(CameraTarget),
}

fn world_y(avatar: Transform3D, local_y: f32) -> f32 {
    (avatar * Vector3::new(0.0, local_y, 0.0)).y
}

/// The camera that frames the body, not "overall": wearables can be arbitrarily large and the
/// avatar height must not depend on what is equipped.
pub fn fit_to_overall(aabbs: &Aabbs, view: FitView, avatar: Transform3D) -> OverallFit {
    let Some(overall) = aabbs.get("overall") else {
        return OverallFit::NoBody;
    };
    if view.size.x <= 0.0 || view.size.y <= 0.0 {
        return OverallFit::Deferred;
    }
    let aabb = aabbs.get("body_base").copied().unwrap_or(*overall);
    let (vp_w, vp_h) = (view.size.x, view.size.y);
    let raw_inner_h = vp_h - view.top - view.bottom;
    let raw_inner_w = vp_w - view.margin_left - view.margin_right;
    // Margins claiming almost the whole viewport mean the parent layout pass has not settled:
    // defer rather than commit a zoomed-out size that makes the avatar tiny.
    if raw_inner_h < 50.0 || raw_inner_w < 50.0 {
        return OverallFit::Deferred;
    }
    let inner_h = raw_inner_h.max(1.0);
    let inner_w = raw_inner_w.max(1.0);
    let size = (aabb.size.y * vp_h / inner_h)
        .max(aabb.size.x * vp_h / inner_w)
        .max(MIN_CAMERA_SIZE_OVERALL);
    let center_y = aabb.center().y + (view.top - view.bottom) * size / (2.0 * vp_h);
    let bottom_w = world_y(avatar, aabb.position.y);
    let top_w = world_y(avatar, aabb.position.y + aabb.size.y);
    let pan_min = bottom_w + size * (0.5 - view.bottom / vp_h);
    let pan_max = top_w - size * (0.5 - view.top / vp_h);
    OverallFit::Fit(CameraTarget {
        size,
        center_y: center_y.clamp(pan_min.min(pan_max), pan_min.max(pan_max)),
    })
}

/// The zoom-out limit and the focus height of a region: how far the user may zoom out and the
/// height the camera starts from.
pub fn fit_limits(
    aabb: Aabb,
    extra_margin: i32,
    view: FitView,
    focus: &str,
    aabbs: &Aabbs,
) -> Option<(f32, f32)> {
    if view.size.x <= 0.0 || view.size.y <= 0.0 {
        return None;
    }
    let mut aabb = aabb;
    let padding = focus_padding(focus);
    if padding > 0.0 {
        aabb = aabb.grow(aabb.size.x.max(aabb.size.y) * padding * 0.5);
    }
    let extra = extra_margin as f32 * 2.0;
    let inner_h = (view.size.y - view.top - view.bottom - extra).max(1.0);
    let inner_w = (view.size.x - view.margin_left - view.margin_right - extra).max(1.0);
    let limit = aabbs.get("body_base").copied().unwrap_or(aabb);
    let cam_size = (limit.size.y * view.size.y / inner_h).max(limit.size.x * view.size.y / inner_w);
    Some((
        cam_size.max(MIN_CAMERA_SIZE_PART),
        focus_aabb_center_y(focus, aabb),
    ))
}

/// The box that decides when a region needs a new fit: the body parts, not the wearables.
pub fn stable_aabb(focus: &str, aabbs: &Aabbs) -> Aabb {
    let fallback = aabbs
        .get("overall")
        .copied()
        .unwrap_or(Aabb::new(Vector3::ZERO, Vector3::ONE * 2.0));
    let key = match focus {
        "head" | "head_base" | "head_base_facial" => "head_base",
        "torso" => "torso_base",
        "legs" => "legs_base",
        "feet" => "feet_base",
        "hands" => "hands_base",
        _ => "body_base",
    };
    aabbs.get(key).copied().unwrap_or(fallback)
}

/// The camera for a region, given the zoom-out limit. `capped_extra` is the extra margin already
/// capped to leave a positive inner area.
pub fn fit_camera(
    stable: Aabb,
    aabb: Aabb,
    capped_extra: i32,
    view: FitView,
    avatar: Transform3D,
    fitted_camera_size: f32,
    min_camera_size: f32,
) -> CameraTarget {
    let (vp_w, vp_h) = (view.size.x, view.size.y);
    let _ = vp_w;
    let available_h = vp_h - view.top - view.bottom;
    let raw_inner_w =
        view.size.x - view.margin_left - view.margin_right - capped_extra as f32 * 2.0;
    let raw_inner_h = available_h - capped_extra as f32 * 2.0;
    let inner_h = raw_inner_h.max(1.0);
    let inner_w = raw_inner_w.max(1.0);
    let size = (stable.size.y * vp_h / inner_h)
        .max(stable.size.x * vp_h / inner_w)
        .clamp(min_camera_size, fitted_camera_size.max(min_camera_size));
    let center_y = stable.center().y + (view.top - view.bottom) * size / (2.0 * vp_h);
    let focus_bottom = world_y(avatar, aabb.position.y);
    let focus_top = world_y(avatar, aabb.position.y + aabb.size.y);
    let pan_min = focus_bottom + size * (0.5 - view.bottom / vp_h);
    let pan_max = focus_top - size * (0.5 - view.top / vp_h);
    CameraTarget {
        size,
        center_y: center_y.clamp(pan_min.min(pan_max), pan_min.max(pan_max)),
    }
}

/// The height range the camera may pan within, so the avatar stays in view.
pub fn pan_limits(
    aabbs: &Aabbs,
    view: FitView,
    avatar: Transform3D,
    camera_size: f32,
) -> (f32, f32) {
    let bounds = aabbs
        .get("overall")
        .or_else(|| aabbs.get("body_base"))
        .copied()
        .unwrap_or(Aabb::new(Vector3::ZERO, Vector3::ONE * 2.0));
    let bottom = world_y(avatar, bounds.position.y);
    let top = world_y(avatar, bounds.position.y + bounds.size.y);
    let vp_h = view.size.y;
    let pan_min = bottom + camera_size * (0.5 - view.bottom / vp_h);
    let pan_max = top - camera_size * (0.5 - view.top / vp_h);
    (pan_min.min(pan_max), pan_min.max(pan_max))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Full-body box of an idle avatar in local space (~1.8 m tall).
    fn body() -> Aabb {
        Aabb::new(Vector3::new(-0.4, 0.0, -0.4), Vector3::new(0.8, 1.8, 0.8))
    }

    /// A giant wearable inflates "overall"; the fit must frame "body_base" instead.
    fn wearable() -> Aabb {
        Aabb::new(Vector3::new(-3.0, 0.0, -3.0), Vector3::new(6.0, 4.0, 6.0))
    }

    fn aabbs() -> Aabbs {
        [
            ("overall".to_string(), wearable()),
            ("body_base".to_string(), body()),
        ]
        .into_iter()
        .collect()
    }

    const MARGIN_TOP: f32 = 64.0;
    const MARGIN_BOTTOM: f32 = 46.0;

    fn view(size: Vector2) -> FitView {
        FitView {
            size,
            margin_left: 0.0,
            margin_right: 0.0,
            top: MARGIN_TOP,
            bottom: MARGIN_BOTTOM,
        }
    }

    fn expected_size(vp: Vector2) -> f32 {
        let inner_h = vp.y - MARGIN_TOP - MARGIN_BOTTOM;
        let inner_w = vp.x;
        (body().size.y * vp.y / inner_h)
            .max(body().size.x * vp.x / inner_w)
            .max(MIN_CAMERA_SIZE_OVERALL)
    }

    fn fit(size: Vector2) -> CameraTarget {
        match fit_to_overall(&aabbs(), view(size), Transform3D::IDENTITY) {
            OverallFit::Fit(target) => target,
            other => panic!("expected a fit, got {other:?}"),
        }
    }

    #[test]
    fn the_fit_follows_the_viewport_size() {
        // Layout settles through a transient short viewport, then the final size arrives: the
        // camera must refit or the avatar reads tiny (#2685).
        let transient = Vector2::new(390.0, 500.0);
        let settled = Vector2::new(390.0, 844.0);
        assert!((fit(transient).size - expected_size(transient)).abs() < 1e-5);
        assert!((fit(settled).size - expected_size(settled)).abs() < 1e-5);
        assert!(fit(transient).size != fit(settled).size);
    }

    #[test]
    fn large_wearables_do_not_shrink_the_avatar() {
        let vp = Vector2::new(390.0, 844.0);
        assert!((fit(vp).size - expected_size(vp)).abs() < 1e-5);
    }

    #[test]
    fn an_unsettled_layout_defers() {
        let tiny = FitView {
            size: Vector2::new(390.0, 100.0),
            ..view(Vector2::new(390.0, 100.0))
        };
        assert_eq!(
            fit_to_overall(&aabbs(), tiny, Transform3D::IDENTITY),
            OverallFit::Deferred
        );
        assert_eq!(
            fit_to_overall(&aabbs(), view(Vector2::ZERO), Transform3D::IDENTITY),
            OverallFit::Deferred
        );
        assert_eq!(
            fit_to_overall(
                &Aabbs::new(),
                view(Vector2::new(390.0, 844.0)),
                Transform3D::IDENTITY
            ),
            OverallFit::NoBody
        );
    }

    #[test]
    fn the_camera_height_stays_inside_the_pan_limits() {
        let vp = Vector2::new(390.0, 844.0);
        let target = fit(vp);
        let (lo, hi) = pan_limits(&aabbs(), view(vp), Transform3D::IDENTITY, target.size);
        assert!(target.center_y >= lo && target.center_y <= hi);
    }

    #[test]
    fn categories_pick_their_region() {
        assert_eq!(focus_for_category("hat"), "head");
        assert_eq!(focus_for_category("eyes"), "head_base");
        assert_eq!(focus_for_category("facial_hair"), "head_base_facial");
        assert_eq!(focus_for_category("hands_wear"), "hands");
        assert_eq!(focus_for_category("body_shape"), "overall");
        assert_eq!(focus_extra_margin("hands"), 80);
        assert_eq!(focus_extra_margin("legs"), 0);
    }

    #[test]
    fn meshes_belong_to_regions_by_name() {
        assert_eq!(mesh_category("hair__hair"), "head");
        assert_eq!(mesh_category("bodyshape_ubody_basemesh"), "torso");
        assert_eq!(mesh_category("bodyshape_lbody_basemesh"), "legs");
        assert_eq!(mesh_category("sneakers__feet"), "feet");
        assert_eq!(mesh_category("random"), "");
    }

    #[test]
    fn symmetrizing_mirrors_the_wider_side() {
        let skewed = Aabb::new(Vector3::new(-0.2, 0.0, 0.0), Vector3::new(0.8, 1.0, 0.5));
        let mirrored = symmetrize_aabb_x(skewed);
        assert_eq!(mirrored.position.x, -0.6);
        assert_eq!(mirrored.size.x, 1.2);
        assert_eq!(mirrored.size.y, 1.0);
    }

    #[test]
    fn endpoints_enumerate_the_corners() {
        let aabb = Aabb::new(Vector3::ZERO, Vector3::ONE);
        let corners: std::collections::HashSet<[i32; 3]> = (0..8)
            .map(|i| {
                let p = aabb_endpoint(aabb, i);
                [p.x as i32, p.y as i32, p.z as i32]
            })
            .collect();
        assert_eq!(corners.len(), 8);
    }

    #[test]
    fn the_stable_box_is_the_body_part() {
        let mut map = aabbs();
        map.insert("head_base".into(), Aabb::new(Vector3::ZERO, Vector3::ONE));
        assert_eq!(stable_aabb("head", &map).size, Vector3::ONE);
        assert_eq!(stable_aabb("overall", &map), body());
    }
}
