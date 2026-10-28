//! The debug overlay of a light: range cones or sphere, status dot and a ring of info labels.

use godot::classes::base_material_3d::{BillboardMode, ShadingMode, Transparency};
use godot::classes::mesh::PrimitiveType;
use godot::classes::{ImmediateMesh, Label3D, MeshInstance3D, StandardMaterial3D};
use godot::prelude::*;

use super::logic::{debug_cone_dimensions, fmtf};

const CONE_SEGMENTS: i32 = 32;
const SPHERE_SEGMENTS: i32 = 32;
const SPHERE_MIN_RADIUS: f32 = 1.0;
const INFO_LABELS: usize = 6;
const INFO_PIXEL_SIZE: f32 = 0.000625;
const STATUS_PIXEL_SIZE: f32 = 0.0011;
const INTENSITY_POS: Vector3 = Vector3::new(0.0, 0.62, 0.0);
const RANGE_POS: Vector3 = Vector3::new(-0.95, 0.15, 0.0);
const TEXTURE_POS: Vector3 = Vector3::new(1.05, 0.15, 0.0);
const BUDGET_POS: Vector3 = Vector3::new(0.0, -0.42, 0.0);

/// What the gizmo draws; the component fills it from its state.
pub struct GizmoView {
    pub spot: bool,
    pub intensity: f32,
    pub range: f32,
    pub inner_angle: f32,
    pub outer_angle: f32,
    pub activation_range: f32,
    pub in_range: bool,
    pub budget_blocked: bool,
    pub texture_text: String,
    pub budget_text: String,
}

pub struct Gizmo {
    root: Gd<Node3D>,
    status: Gd<Label3D>,
    outer_cone: Gd<MeshInstance3D>,
    inner_cone: Gd<MeshInstance3D>,
    sphere: Gd<MeshInstance3D>,
    info_lines: Gd<MeshInstance3D>,
    labels: Vec<Gd<Label3D>>,
}

fn named_mesh(name: &str) -> Gd<MeshInstance3D> {
    let mut node = MeshInstance3D::new_alloc();
    node.set_name(name);
    node
}

fn billboard_label(name: &str, pixel_size: f32) -> Gd<Label3D> {
    let mut label = Label3D::new_alloc();
    label.set_name(name);
    label.set_draw_flag(godot::classes::label_3d::DrawFlags::FIXED_SIZE, true);
    label.set_pixel_size(pixel_size);
    label.set_draw_flag(
        godot::classes::label_3d::DrawFlags::DISABLE_DEPTH_TEST,
        true,
    );
    label.set_billboard_mode(BillboardMode::ENABLED);
    label
}

fn line_material(color: Color) -> Gd<StandardMaterial3D> {
    let mut material = StandardMaterial3D::new_gd();
    material.set_shading_mode(ShadingMode::UNSHADED);
    material.set_albedo(color);
    material.set_feature(godot::classes::base_material_3d::Feature::EMISSION, true);
    material.set_emission(color);
    material.set_flag(
        godot::classes::base_material_3d::Flags::DISABLE_DEPTH_TEST,
        true,
    );
    material.set_transparency(Transparency::ALPHA);
    material
}

fn draw_lines(target: &mut Gd<MeshInstance3D>, color: Color, vertices: &[Vector3]) {
    let mut mesh = ImmediateMesh::new_gd();
    mesh.surface_begin_ex(PrimitiveType::LINES)
        .material(&line_material(color))
        .done();
    for vertex in vertices {
        mesh.surface_add_vertex(*vertex);
    }
    mesh.surface_end();
    target.set_mesh(&mesh);
}

fn cone_vertices(length: f32, radius: f32, rays: bool) -> Vec<Vector3> {
    let mut out = Vec::new();
    for i in 0..CONE_SEGMENTS {
        let a0 = std::f32::consts::TAU * i as f32 / CONE_SEGMENTS as f32;
        let a1 = std::f32::consts::TAU * (i + 1) as f32 / CONE_SEGMENTS as f32;
        let p0 = Vector3::new(a0.cos() * radius, a0.sin() * radius, -length);
        let p1 = Vector3::new(a1.cos() * radius, a1.sin() * radius, -length);
        out.extend([p0, p1]);
        if rays {
            out.extend([Vector3::ZERO, p0]);
        }
    }
    out
}

fn sphere_vertices(radius: f32) -> Vec<Vector3> {
    let mut out = Vec::new();
    for i in 0..SPHERE_SEGMENTS {
        let a0 = std::f32::consts::TAU * i as f32 / SPHERE_SEGMENTS as f32;
        let a1 = std::f32::consts::TAU * (i + 1) as f32 / SPHERE_SEGMENTS as f32;
        let (x0, y0) = (a0.cos() * radius, a0.sin() * radius);
        let (x1, y1) = (a1.cos() * radius, a1.sin() * radius);
        out.extend([
            Vector3::new(x0, y0, 0.0),
            Vector3::new(x1, y1, 0.0),
            Vector3::new(x0, 0.0, y0),
            Vector3::new(x1, 0.0, y1),
            Vector3::new(0.0, x0, y0),
            Vector3::new(0.0, x1, y1),
        ]);
    }
    out
}

fn circle_anchor(light_range: f32, angle: f32, side: f32) -> Vector3 {
    let (length, radius) = debug_cone_dimensions(light_range, angle);
    Vector3::new(side * radius, 0.0, -length)
}

impl Gizmo {
    pub fn new(parent: &mut Gd<Node3D>, visible: bool) -> Self {
        let mut root = Node3D::new_alloc();
        root.set_name("DCL_Light_DebugGizmo");
        parent.add_child(&root);

        let mut status = billboard_label("StatusLabel", STATUS_PIXEL_SIZE);
        status.set_text("🟢");
        root.add_child(&status);

        let mut outer_cone = named_mesh("SpotOuterCone");
        let mut inner_cone = named_mesh("SpotInnerCone");
        let mut sphere = named_mesh("PointSphere");
        let mut info_lines = named_mesh("InfoLines");
        root.add_child(&outer_cone);
        root.add_child(&inner_cone);
        root.add_child(&sphere);
        root.add_child(&info_lines);
        outer_cone.set_visible(false);
        inner_cone.set_visible(false);
        sphere.set_visible(false);
        info_lines.set_visible(false);

        let labels = (0..INFO_LABELS)
            .map(|i| {
                let mut label = billboard_label(&format!("DebugInfoLabel{i}"), INFO_PIXEL_SIZE);
                label.set_outline_size(2);
                root.add_child(&label);
                label
            })
            .collect();
        root.set_visible(visible);
        Self {
            root,
            status,
            outer_cone,
            inner_cone,
            sphere,
            info_lines,
            labels,
        }
    }

    pub fn set_visible(&mut self, visible: bool) {
        self.root.set_visible(visible);
    }

    pub fn update(&mut self, view: &GizmoView) {
        self.root.set_visible(true);
        self.status.set_text(if !view.in_range {
            "🔴"
        } else if view.budget_blocked {
            "🟡"
        } else {
            "🟢"
        });

        self.outer_cone.set_visible(view.spot);
        self.inner_cone.set_visible(view.spot);
        self.sphere.set_visible(!view.spot);
        if view.spot {
            let (length, radius) = debug_cone_dimensions(view.range, view.outer_angle);
            draw_lines(
                &mut self.outer_cone,
                Color::from_rgba(1.0, 0.55, 0.10, 0.95),
                &cone_vertices(length, radius, true),
            );
            let (length, radius) = debug_cone_dimensions(view.range, view.inner_angle);
            draw_lines(
                &mut self.inner_cone,
                Color::from_rgba(1.0, 1.0, 0.15, 0.95),
                &cone_vertices(length, radius, false),
            );
        } else {
            draw_lines(
                &mut self.sphere,
                Color::from_rgba(0.15, 0.85, 1.0, 0.95),
                &sphere_vertices(view.range.max(SPHERE_MIN_RADIUS)),
            );
        }
        self.update_labels(view);
    }

    fn set_label(&mut self, index: usize, position: Vector3, text: &str) {
        if let Some(label) = self.labels.get_mut(index) {
            label.set_position(position);
            label.set_text(text);
            label.set_visible(!text.is_empty());
        }
    }

    fn update_labels(&mut self, view: &GizmoView) {
        self.set_label(0, INTENSITY_POS, &format!("i:{}", fmtf(view.intensity)));
        self.set_label(
            1,
            RANGE_POS,
            &format!("r:{} ar:{}", fmtf(view.range), fmtf(view.activation_range)),
        );
        self.set_label(2, TEXTURE_POS, &view.texture_text);
        self.set_label(3, BUDGET_POS, &view.budget_text);

        let mut anchors: [Option<(Vector3, Vector3)>; INFO_LABELS] = [None; INFO_LABELS];
        if view.spot {
            let inner = circle_anchor(view.range, view.inner_angle, -1.0);
            let outer = circle_anchor(view.range, view.outer_angle, 1.0);
            let inner_label = inner + Vector3::new(-0.36, -0.14, 0.0);
            let outer_label = outer + Vector3::new(0.36, 0.14, 0.0);
            self.set_label(4, inner_label, &format!("inner:{}", fmtf(view.inner_angle)));
            self.set_label(5, outer_label, &format!("outer:{}", fmtf(view.outer_angle)));
            anchors[4] = Some((inner, inner_label));
            anchors[5] = Some((outer, outer_label));
        } else {
            self.set_label(4, Vector3::ZERO, "");
            self.set_label(5, Vector3::ZERO, "");
        }

        let mut lines = Vec::new();
        for (i, label) in self.labels.iter().enumerate() {
            if !label.is_visible() {
                continue;
            }
            let (start, end) = anchors[i].unwrap_or((Vector3::ZERO, label.get_position() * 0.86));
            lines.extend([start, end]);
        }
        self.info_lines.set_visible(true);
        draw_lines(
            &mut self.info_lines,
            Color::from_rgba(1.0, 1.0, 1.0, 0.85),
            &lines,
        );
    }
}
