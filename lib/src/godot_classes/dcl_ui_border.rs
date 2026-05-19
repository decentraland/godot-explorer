use godot::{
    builtin::Corner,
    classes::{
        control::{LayoutPreset, MouseFilter},
        Control, IControl, StyleBoxFlat,
    },
    prelude::*,
};

#[derive(GodotClass)]
#[class(base=Control)]
pub struct DclUiBorder {
    base: Base<Control>,
    // [left, right, top, bottom]
    widths: [f32; 4],
    // [top_left, top_right, bottom_right, bottom_left]
    radii: [f32; 4],
    // [top, right, bottom, left]
    colors: [Color; 4],
    // Built lazily in set_border when all four colors match. When present, the
    // draw path is a single FFI call (draw_style_box) instead of rebuilding the
    // StyleBoxFlat on every redraw. None ⇒ slow path (per-side colors differ).
    cached_stylebox: Option<Gd<StyleBoxFlat>>,
}

#[godot_api]
impl IControl for DclUiBorder {
    fn init(base: Base<Control>) -> Self {
        let transparent = Color::from_rgba(0.0, 0.0, 0.0, 0.0);
        Self {
            base,
            widths: [0.0; 4],
            radii: [0.0; 4],
            colors: [transparent; 4],
            cached_stylebox: None,
        }
    }

    fn ready(&mut self) {
        self.base_mut().set_mouse_filter(MouseFilter::IGNORE);
        self.base_mut().set_anchors_preset(LayoutPreset::FULL_RECT);

        let mut parent = self
            .base()
            .get_parent()
            .expect("ui_border must have a parent");
        parent.connect("resized", &self.base().callable("_on_parent_size"));
    }

    fn draw(&mut self) {
        let size = self.base().get_size();
        if size.x <= 0.0 || size.y <= 0.0 {
            return;
        }

        // Fast path: stylebox was prebuilt in set_border. One FFI call, no rebuild.
        if let Some(sb) = self.cached_stylebox.clone() {
            let rect = Rect2::new(Vector2::ZERO, size);
            self.base_mut().draw_style_box(&sb, rect);
            return;
        }

        let [bw_l, bw_r, bw_t, bw_b] = self.widths;
        let [c_t, c_r, c_b, c_l] = self.colors;

        // Slow path: per-side colors differ.
        // For each corner: if it has a radius, two annular sectors split at the bisector
        // fill it; the adjacent side's inner endpoint is aligned to the arc's inner endpoint
        // (br_*, side_thickness). If it has no radius, the side trapezoid covers the corner
        // with a CSS miter — inner endpoint at the adjacent border-width (bw_*, side_thickness).
        let w = size.x;
        let h = size.y;
        let [br_tl, br_tr, br_br, br_bl] = self.radii;

        // Inner-endpoint coordinates per side, chosen to match the arc when present.
        let top_in_l_x = if br_tl > 0.0 { br_tl } else { bw_l };
        let top_in_r_x = if br_tr > 0.0 { w - br_tr } else { w - bw_r };
        let right_in_t_y = if br_tr > 0.0 { br_tr } else { bw_t };
        let right_in_b_y = if br_br > 0.0 { h - br_br } else { h - bw_b };
        let bottom_in_r_x = if br_br > 0.0 { w - br_br } else { w - bw_r };
        let bottom_in_l_x = if br_bl > 0.0 { br_bl } else { bw_l };
        let left_in_b_y = if br_bl > 0.0 { h - br_bl } else { h - bw_b };
        let left_in_t_y = if br_tl > 0.0 { br_tl } else { bw_t };

        // Top
        if bw_t > 0.0 && c_t.a > 0.0 {
            let pts = packed_v2(&[
                Vector2::new(br_tl, 0.0),
                Vector2::new(w - br_tr, 0.0),
                Vector2::new(top_in_r_x, bw_t),
                Vector2::new(top_in_l_x, bw_t),
            ]);
            self.base_mut().draw_colored_polygon(&pts, c_t);
        }
        // Right
        if bw_r > 0.0 && c_r.a > 0.0 {
            let pts = packed_v2(&[
                Vector2::new(w, br_tr),
                Vector2::new(w, h - br_br),
                Vector2::new(w - bw_r, right_in_b_y),
                Vector2::new(w - bw_r, right_in_t_y),
            ]);
            self.base_mut().draw_colored_polygon(&pts, c_r);
        }
        // Bottom
        if bw_b > 0.0 && c_b.a > 0.0 {
            let pts = packed_v2(&[
                Vector2::new(w - br_br, h),
                Vector2::new(br_bl, h),
                Vector2::new(bottom_in_l_x, h - bw_b),
                Vector2::new(bottom_in_r_x, h - bw_b),
            ]);
            self.base_mut().draw_colored_polygon(&pts, c_b);
        }
        // Left
        if bw_l > 0.0 && c_l.a > 0.0 {
            let pts = packed_v2(&[
                Vector2::new(0.0, h - br_bl),
                Vector2::new(0.0, br_tl),
                Vector2::new(bw_l, left_in_t_y),
                Vector2::new(bw_l, left_in_b_y),
            ]);
            self.base_mut().draw_colored_polygon(&pts, c_l);
        }

        // Corner arcs (annular sectors). Each half draws over the gap left by the
        // trapezoid outer-corner cut, in its adjacent side's color.
        const SEGMENTS: u32 = 12;
        // Top-left: arc angle PI .. 3*PI/2 (left .. up). Bisector at 5*PI/4.
        if br_tl > 0.0 {
            let center = Vector2::new(br_tl, br_tl);
            if bw_l > 0.0 && c_l.a > 0.0 {
                let pts = annular_sector(
                    center,
                    br_tl,
                    (br_tl - bw_l).max(0.0),
                    std::f32::consts::PI,
                    1.25 * std::f32::consts::PI,
                    SEGMENTS,
                );
                self.base_mut().draw_colored_polygon(&pts, c_l);
            }
            if bw_t > 0.0 && c_t.a > 0.0 {
                let pts = annular_sector(
                    center,
                    br_tl,
                    (br_tl - bw_t).max(0.0),
                    1.25 * std::f32::consts::PI,
                    1.5 * std::f32::consts::PI,
                    SEGMENTS,
                );
                self.base_mut().draw_colored_polygon(&pts, c_t);
            }
        }
        // Top-right: arc angle 3*PI/2 .. 2*PI (up .. right). Bisector at 7*PI/4.
        if br_tr > 0.0 {
            let center = Vector2::new(w - br_tr, br_tr);
            if bw_t > 0.0 && c_t.a > 0.0 {
                let pts = annular_sector(
                    center,
                    br_tr,
                    (br_tr - bw_t).max(0.0),
                    1.5 * std::f32::consts::PI,
                    1.75 * std::f32::consts::PI,
                    SEGMENTS,
                );
                self.base_mut().draw_colored_polygon(&pts, c_t);
            }
            if bw_r > 0.0 && c_r.a > 0.0 {
                let pts = annular_sector(
                    center,
                    br_tr,
                    (br_tr - bw_r).max(0.0),
                    1.75 * std::f32::consts::PI,
                    2.0 * std::f32::consts::PI,
                    SEGMENTS,
                );
                self.base_mut().draw_colored_polygon(&pts, c_r);
            }
        }
        // Bottom-right: arc angle 0 .. PI/2 (right .. down). Bisector at PI/4.
        if br_br > 0.0 {
            let center = Vector2::new(w - br_br, h - br_br);
            if bw_r > 0.0 && c_r.a > 0.0 {
                let pts = annular_sector(
                    center,
                    br_br,
                    (br_br - bw_r).max(0.0),
                    0.0,
                    0.25 * std::f32::consts::PI,
                    SEGMENTS,
                );
                self.base_mut().draw_colored_polygon(&pts, c_r);
            }
            if bw_b > 0.0 && c_b.a > 0.0 {
                let pts = annular_sector(
                    center,
                    br_br,
                    (br_br - bw_b).max(0.0),
                    0.25 * std::f32::consts::PI,
                    0.5 * std::f32::consts::PI,
                    SEGMENTS,
                );
                self.base_mut().draw_colored_polygon(&pts, c_b);
            }
        }
        // Bottom-left: arc angle PI/2 .. PI (down .. left). Bisector at 3*PI/4.
        if br_bl > 0.0 {
            let center = Vector2::new(br_bl, h - br_bl);
            if bw_b > 0.0 && c_b.a > 0.0 {
                let pts = annular_sector(
                    center,
                    br_bl,
                    (br_bl - bw_b).max(0.0),
                    0.5 * std::f32::consts::PI,
                    0.75 * std::f32::consts::PI,
                    SEGMENTS,
                );
                self.base_mut().draw_colored_polygon(&pts, c_b);
            }
            if bw_l > 0.0 && c_l.a > 0.0 {
                let pts = annular_sector(
                    center,
                    br_bl,
                    (br_bl - bw_l).max(0.0),
                    0.75 * std::f32::consts::PI,
                    std::f32::consts::PI,
                    SEGMENTS,
                );
                self.base_mut().draw_colored_polygon(&pts, c_l);
            }
        }
    }
}

fn packed_v2(points: &[Vector2]) -> PackedVector2Array {
    PackedVector2Array::from(points)
}

fn annular_sector(
    center: Vector2,
    radius_outer: f32,
    radius_inner: f32,
    start_angle: f32,
    end_angle: f32,
    segments: u32,
) -> PackedVector2Array {
    let mut pts = PackedVector2Array::new();
    let step = (end_angle - start_angle) / segments as f32;
    for i in 0..=segments {
        let a = start_angle + i as f32 * step;
        pts.push(center + Vector2::new(a.cos(), a.sin()) * radius_outer);
    }
    if radius_inner > 0.0 {
        for i in (0..=segments).rev() {
            let a = start_angle + i as f32 * step;
            pts.push(center + Vector2::new(a.cos(), a.sin()) * radius_inner);
        }
    } else {
        pts.push(center);
    }
    pts
}

#[godot_api]
impl DclUiBorder {
    #[func]
    fn _on_parent_size(&mut self) {
        self.base_mut().queue_redraw();
    }

    pub fn set_border(&mut self, widths: [f32; 4], radii: [f32; 4], colors: [Color; 4]) {
        self.widths = widths;
        self.radii = radii;
        self.colors = colors;

        let [c_t, c_r, c_b, c_l] = colors;
        let all_colors_equal = c_t == c_r && c_r == c_b && c_b == c_l;

        if all_colors_equal && c_t.a > 0.0 {
            // Build the fast-path StyleBoxFlat once here. Reuse it on every draw.
            let mut sb = self
                .cached_stylebox
                .clone()
                .unwrap_or_else(StyleBoxFlat::new_gd);
            sb.set_bg_color(Color::from_rgba(0.0, 0.0, 0.0, 0.0));
            sb.set_border_color(c_t);
            sb.set_border_width(Side::LEFT, widths[0].round() as i32);
            sb.set_border_width(Side::RIGHT, widths[1].round() as i32);
            sb.set_border_width(Side::TOP, widths[2].round() as i32);
            sb.set_border_width(Side::BOTTOM, widths[3].round() as i32);
            sb.set_corner_radius(Corner::TOP_LEFT, radii[0].round() as i32);
            sb.set_corner_radius(Corner::TOP_RIGHT, radii[1].round() as i32);
            sb.set_corner_radius(Corner::BOTTOM_RIGHT, radii[2].round() as i32);
            sb.set_corner_radius(Corner::BOTTOM_LEFT, radii[3].round() as i32);
            sb.set_anti_aliased(true);
            self.cached_stylebox = Some(sb);
        } else {
            self.cached_stylebox = None;
        }

        self.base_mut().queue_redraw();
    }
}
