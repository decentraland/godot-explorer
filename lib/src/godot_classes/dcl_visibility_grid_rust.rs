use godot::classes::RenderingServer;
use godot::prelude::*;

/// Rust-side hot path for the cell-based visibility-grid PVS culling.
///
/// Build still happens in GDScript (`visibility_grid.gd::build_from_scene_tree`)
/// where Godot's scene-tree introspection lives. After building, the GDScript
/// side calls `set_grid_topology`, `set_cell_*`, `set_mi_*`, `set_pvs_bits`,
/// then `mark_built()`. Per-frame, `update_visibility(cam_pos, frustum)` does
/// the loop natively and flips Godot instance visibility via RenderingServer.
///
/// **Perf wins over the pure-GDScript implementation:**
/// 1. Per-cell loop runs at native speed (avoids GDScript per-op overhead).
/// 2. Visibility flips go through `RenderingServer::instance_set_visible(rid)`,
///    which skips the scene-tree visibility propagation that `Node3D::visible = bool`
///    triggers — important for MIs with children (collision shapes, shadow proxies).
/// 3. Frustum plane test is inlined; no per-cell GDScript `Array[Plane]` walk.
#[derive(GodotClass)]
#[class(base=Node, init)]
pub struct DclVisibilityGridRust {
    base: Base<Node>,

    built: bool,
    cell_origin_x: f32,
    cell_origin_z: f32,
    cell_size: f32,
    cols: i32,
    rows: i32,
    max_distance_m: f32,
    min_size_distance_ratio: f32,
    hide_delay_frames: i32,

    cell_aabb_min: Vec<[f32; 3]>,
    cell_aabb_size: Vec<[f32; 3]>,
    cell_has_content: Vec<u8>,
    cell_last_visible: Vec<u8>,
    cell_hide_streak: Vec<i32>,
    cell_mi_indices: Vec<Vec<i32>>,

    mi_rids: Vec<Rid>,
    mi_visible_count: Vec<i32>,

    pvs_bits: Vec<u8>,

    toggled_on_total: i64,
    toggled_off_total: i64,
}

#[godot_api]
impl DclVisibilityGridRust {
    #[func]
    fn set_grid_topology(
        &mut self,
        origin_x: f32,
        origin_z: f32,
        cell_size: f32,
        cols: i32,
        rows: i32,
    ) {
        self.cell_origin_x = origin_x;
        self.cell_origin_z = origin_z;
        self.cell_size = cell_size;
        self.cols = cols;
        self.rows = rows;
        let n = (cols * rows) as usize;
        self.cell_aabb_min = vec![[0.0; 3]; n];
        self.cell_aabb_size = vec![[0.0; 3]; n];
        self.cell_has_content = vec![0; n];
        self.cell_last_visible = vec![1; n];
        self.cell_hide_streak = vec![0; n];
        self.cell_mi_indices = vec![Vec::new(); n];
    }

    #[func]
    fn set_thresholds(
        &mut self,
        max_distance_m: f32,
        min_size_distance_ratio: f32,
        hide_delay_frames: i32,
    ) {
        self.max_distance_m = max_distance_m;
        self.min_size_distance_ratio = min_size_distance_ratio;
        self.hide_delay_frames = hide_delay_frames;
    }

    #[func]
    fn set_cell_aabb(
        &mut self,
        cell_idx: i32,
        pos: Vector3,
        size: Vector3,
        has_content: bool,
    ) {
        let i = cell_idx as usize;
        if i >= self.cell_has_content.len() {
            return;
        }
        self.cell_aabb_min[i] = [pos.x, pos.y, pos.z];
        self.cell_aabb_size[i] = [size.x, size.y, size.z];
        self.cell_has_content[i] = if has_content { 1 } else { 0 };
    }

    #[func]
    fn set_cell_mi_indices(&mut self, cell_idx: i32, indices: PackedInt32Array) {
        let i = cell_idx as usize;
        if i >= self.cell_mi_indices.len() {
            return;
        }
        self.cell_mi_indices[i] = indices.to_vec();
    }

    /// Register a MeshInstance3D RID with its initial visible-cell count.
    /// `rid` must be the result of `MeshInstance3D::get_instance()` on the GDScript side.
    #[func]
    fn add_mi(&mut self, rid: Rid, initial_visible_count: i32) {
        self.mi_rids.push(rid);
        self.mi_visible_count.push(initial_visible_count);
    }

    #[func]
    fn set_pvs_bits(&mut self, bits: PackedByteArray) {
        self.pvs_bits = bits.to_vec();
    }

    #[func]
    fn mark_built(&mut self) {
        self.built = true;
    }

    /// Per-frame entry. `cam_pos` is the camera world position; `frustum_planes`
    /// is `Camera3D::get_frustum()` (6 planes oriented OUTWARD).
    /// Returns `{toggled_on, toggled_off, cells_visible}`.
    #[func]
    fn update_visibility(
        &mut self,
        cam_pos: Vector3,
        frustum_planes: Array<Plane>,
    ) -> Dictionary {
        let mut out = Dictionary::new();
        out.set("toggled_on", 0);
        out.set("toggled_off", 0);
        out.set("cells_visible", 0);

        if !self.built {
            return out;
        }

        let n = (self.cols * self.rows) as usize;
        if n == 0 {
            return out;
        }

        // Snapshot frustum planes into a fixed-size array for tight inner loop.
        // n-vertex AABB-vs-plane test below requires only the plane normal+d.
        let mut planes: [(f32, f32, f32, f32); 8] = [(0.0, 0.0, 0.0, 0.0); 8];
        let mut plane_count = 0usize;
        for p in frustum_planes.iter_shared() {
            if plane_count >= planes.len() {
                break;
            }
            planes[plane_count] = (p.normal.x, p.normal.y, p.normal.z, p.d);
            plane_count += 1;
        }

        let cam_cx = (((cam_pos.x - self.cell_origin_x) / self.cell_size).floor() as i32)
            .clamp(0, self.cols - 1);
        let cam_cz = (((cam_pos.z - self.cell_origin_z) / self.cell_size).floor() as i32)
            .clamp(0, self.rows - 1);
        let cam_cell = (cam_cz * self.cols + cam_cx) as usize;

        let max_dist_sq = self.max_distance_m * self.max_distance_m;
        let size_ratio_sq = self.min_size_distance_ratio * self.min_size_distance_ratio;

        let mut toggled_on = 0i32;
        let mut toggled_off = 0i32;
        let mut cells_visible = 0i32;

        for idx in 0..n {
            if self.cell_has_content[idx] == 0 {
                continue;
            }
            let amin = self.cell_aabb_min[idx];
            let asz = self.cell_aabb_size[idx];
            let cx = amin[0] + asz[0] * 0.5;
            let cz = amin[2] + asz[2] * 0.5;
            let dx = cx - cam_pos.x;
            let dz = cz - cam_pos.z;
            let dist_sq = dx * dx + dz * dz;
            let mut visible = dist_sq <= max_dist_sq;
            if visible && dist_sq > 4.0 {
                let diag_sq = asz[0] * asz[0] + asz[2] * asz[2];
                if diag_sq < dist_sq * size_ratio_sq {
                    visible = false;
                }
            }
            if visible {
                visible = aabb_in_frustum(amin, asz, &planes[..plane_count]);
            }
            if visible {
                let bit_idx = cam_cell * n + idx;
                let byte = bit_idx >> 3;
                let bit = bit_idx & 7;
                if byte >= self.pvs_bits.len() || (self.pvs_bits[byte] & (1 << bit)) == 0 {
                    visible = false;
                }
            }
            if visible {
                cells_visible += 1;
            }

            let prev = self.cell_last_visible[idx] != 0;
            if visible {
                self.cell_hide_streak[idx] = 0;
            } else {
                self.cell_hide_streak[idx] += 1;
                if prev && self.cell_hide_streak[idx] < self.hide_delay_frames {
                    continue;
                }
            }
            if visible == prev {
                continue;
            }
            self.cell_last_visible[idx] = if visible { 1 } else { 0 };
            if visible {
                toggled_on += 1;
            } else {
                toggled_off += 1;
            }
            self.flip_cell_visibility(idx, visible);
        }

        self.toggled_on_total += toggled_on as i64;
        self.toggled_off_total += toggled_off as i64;
        out.set("toggled_on", toggled_on);
        out.set("toggled_off", toggled_off);
        out.set("cells_visible", cells_visible);
        out
    }

    fn flip_cell_visibility(&mut self, cell_idx: usize, visible: bool) {
        // Multi-cell-aware: bump per-MI count; only call RenderingServer when
        // the count crosses 0 (last cell turned off / first cell turned on).
        let mi_ids = self.cell_mi_indices[cell_idx].clone();
        let mut rs = RenderingServer::singleton();
        if visible {
            for &mid in mi_ids.iter() {
                let mid_u = mid as usize;
                if mid_u >= self.mi_visible_count.len() {
                    continue;
                }
                let prev_cnt = self.mi_visible_count[mid_u];
                self.mi_visible_count[mid_u] = prev_cnt + 1;
                if prev_cnt == 0 {
                    rs.instance_set_visible(self.mi_rids[mid_u], true);
                }
            }
        } else {
            for &mid in mi_ids.iter() {
                let mid_u = mid as usize;
                if mid_u >= self.mi_visible_count.len() {
                    continue;
                }
                let prev_cnt = self.mi_visible_count[mid_u];
                self.mi_visible_count[mid_u] = prev_cnt - 1;
                if prev_cnt == 1 {
                    rs.instance_set_visible(self.mi_rids[mid_u], false);
                }
            }
        }
    }

    #[func]
    fn get_runtime_stats(&self) -> Dictionary {
        let mut d = Dictionary::new();
        d.set("toggled_on_total", self.toggled_on_total);
        d.set("toggled_off_total", self.toggled_off_total);
        d.set("total_mis", self.mi_rids.len() as i64);
        d.set("cols", self.cols);
        d.set("rows", self.rows);
        d.set("built", self.built);
        d
    }
}

#[inline]
fn aabb_in_frustum(
    amin: [f32; 3],
    asz: [f32; 3],
    planes: &[(f32, f32, f32, f32)],
) -> bool {
    // n-vertex AABB test: pick the corner FARTHEST INSIDE the plane (the one
    // whose components align with the OPPOSITE of the plane normal). If even
    // that vertex is outside, the AABB is outside the frustum. Godot's
    // `Camera3D.get_frustum()` returns OUTWARD-facing planes — `distance_to`
    // > 0 means outside.
    for &(nx, ny, nz, d) in planes {
        let vx = amin[0] + if nx > 0.0 { 0.0 } else { asz[0] };
        let vy = amin[1] + if ny > 0.0 { 0.0 } else { asz[1] };
        let vz = amin[2] + if nz > 0.0 { 0.0 } else { asz[2] };
        if nx * vx + ny * vy + nz * vz - d > 0.0 {
            return false;
        }
    }
    true
}
