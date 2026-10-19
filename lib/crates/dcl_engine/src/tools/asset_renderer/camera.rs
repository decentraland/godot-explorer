//! Where the asset renderer puts its camera for a shot.

use godot::prelude::Vector3;

use super::input::ShotCamera;

/// The posed asset: the center of its bounds in world space and the length of their diagonal.
#[derive(Clone, Copy, Debug)]
pub struct FitBounds {
    pub center: Vector3,
    pub diagonal: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Placement {
    pub orthographic: bool,
    pub fov_degrees: f32,
    /// Vertical extent of an orthographic camera.
    pub size: f32,
    pub from: Vector3,
    pub target: Vector3,
}

fn vector(components: [f64; 3]) -> Vector3 {
    Vector3::new(
        components[0] as f32,
        components[1] as f32,
        components[2] as f32,
    )
}

/// `aspect` is width over height. A camera keeps its vertical extent, so a portrait shot needs the
/// fit widened or the asset is cropped left and right.
pub fn place(shot: &ShotCamera, aspect: f32, bounds: FitBounds) -> Placement {
    let orthographic = shot.projection == "ortho";
    let fov_degrees = (shot.fov as f32).clamp(1.0, 179.0);
    if !shot.auto_fit {
        return Placement {
            orthographic,
            fov_degrees,
            size: (shot.ortho_size as f32).max(0.001),
            from: vector(shot.position),
            target: vector(shot.target),
        };
    }

    // The bounding sphere of the posed asset: conservative (the diagonal), so nothing crops
    // whatever the orbit angle.
    let radius = (bounds.diagonal * 0.5).max(0.05) * (shot.fit_margin as f32).max(1.0);
    let yaw = (shot.orbit_yaw_degrees as f32).to_radians();
    let pitch = (shot.orbit_pitch_degrees as f32)
        .clamp(-80.0, 80.0)
        .to_radians();
    // Yaw 0 faces the avatar's front (-Z, as the profile body camera does).
    let direction = Vector3::new(
        yaw.sin() * pitch.cos(),
        pitch.sin(),
        -yaw.cos() * pitch.cos(),
    );
    let aspect = if aspect > 0.0 { aspect } else { 1.0 };

    if orthographic {
        let mut size = radius * 2.0;
        if aspect < 1.0 {
            size /= aspect;
        }
        return Placement {
            orthographic,
            fov_degrees,
            size: size.max(0.1),
            from: bounds.center + direction * (radius * 4.0).max(2.0),
            target: bounds.center,
        };
    }
    let mut half_fov = fov_degrees.to_radians() * 0.5;
    // Narrower than tall: the horizontal half angle is the binding one.
    if aspect < 1.0 {
        half_fov = (half_fov.tan() * aspect).atan();
    }
    let distance = (radius / half_fov.sin()).max(radius + 0.5);
    Placement {
        orthographic,
        fov_degrees,
        size: shot.ortho_size as f32,
        from: bounds.center + direction * distance,
        target: bounds.center,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bounds() -> FitBounds {
        FitBounds {
            center: Vector3::new(0.0, 1.0, 0.0),
            diagonal: 2.0,
        }
    }

    #[test]
    fn an_explicit_camera_is_taken_as_given() {
        let shot = ShotCamera {
            auto_fit: false,
            position: [1.0, 2.0, 3.0],
            target: [0.0, 0.0, 0.0],
            ortho_size: 0.0,
            ..ShotCamera::default()
        };
        let placement = place(&shot, 1.0, bounds());
        assert_eq!(placement.from, Vector3::new(1.0, 2.0, 3.0));
        assert_eq!(placement.target, Vector3::ZERO);
        assert_eq!(placement.size, 0.001);
    }

    #[test]
    fn the_front_view_looks_at_the_asset_from_minus_z() {
        let placement = place(&ShotCamera::default(), 1.0, bounds());
        assert_eq!(placement.target, bounds().center);
        assert!(placement.from.z < 0.0);
        assert!(placement.from.x.abs() < 1e-5);
        assert!(placement.from.y > bounds().center.y);
    }

    #[test]
    fn a_portrait_shot_stands_further_back() {
        let wide = place(&ShotCamera::default(), 1.0, bounds());
        let tall = place(&ShotCamera::default(), 0.5, bounds());
        let away = |p: &Placement| (p.from - p.target).length();
        assert!(away(&tall) > away(&wide));
    }

    #[test]
    fn an_orthographic_fit_scales_with_the_asset() {
        let shot = ShotCamera {
            projection: "ortho".to_string(),
            ..ShotCamera::default()
        };
        let small = place(&shot, 1.0, bounds());
        let big = place(
            &shot,
            1.0,
            FitBounds {
                diagonal: 4.0,
                ..bounds()
            },
        );
        assert!(small.orthographic);
        assert!(big.size > small.size);
    }

    #[test]
    fn the_pitch_is_limited() {
        let shot = ShotCamera {
            orbit_pitch_degrees: 120.0,
            ..ShotCamera::default()
        };
        let placement = place(&shot, 1.0, bounds());
        let direction = (placement.from - placement.target).normalized();
        assert!(direction.y <= 80.0_f32.to_radians().sin() + 1e-5);
    }
}
