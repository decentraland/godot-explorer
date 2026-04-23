use fastnoise_lite::FastNoiseLite;
use godot::builtin::{Color, Vector2, Vector3};

use super::{CornerConfig, ParcelState, GRID_SIZE, PARCEL_HALF_SIZE, PARCEL_SIZE, TERRAIN_HEIGHT};

const CLIFF_HEIGHT: f32 = 30.0;
const CLIFF_SEGMENTS: i32 = 32;
const CLIFF_VERTICAL_SEGMENTS: i32 = 20;
const CLIFF_NOISE_STRENGTH: f32 = 0.8;
const CLIFF_LENGTH: f32 = PARCEL_SIZE;

const OVERHANG_SEGMENTS: i32 = 32;
const OVERHANG_DISTANCE: f32 = 0.3;
const OVERHANG_DROOP: f32 = 0.1;

#[derive(Clone, Copy)]
pub struct CliffSide {
    pub outward_normal: Vector3,
    pub edge_position: Vector3,
}

pub fn nothing_sides(config: &CornerConfig) -> Vec<CliffSide> {
    let mut out = Vec::with_capacity(4);
    if config.north == ParcelState::Nothing {
        out.push(CliffSide {
            outward_normal: Vector3::new(0.0, 0.0, -1.0),
            edge_position: Vector3::new(0.0, 0.0, -PARCEL_HALF_SIZE),
        });
    }
    if config.south == ParcelState::Nothing {
        out.push(CliffSide {
            outward_normal: Vector3::new(0.0, 0.0, 1.0),
            edge_position: Vector3::new(0.0, 0.0, PARCEL_HALF_SIZE),
        });
    }
    if config.east == ParcelState::Nothing {
        out.push(CliffSide {
            outward_normal: Vector3::new(1.0, 0.0, 0.0),
            edge_position: Vector3::new(PARCEL_HALF_SIZE, 0.0, 0.0),
        });
    }
    if config.west == ParcelState::Nothing {
        out.push(CliffSide {
            outward_normal: Vector3::new(-1.0, 0.0, 0.0),
            edge_position: Vector3::new(-PARCEL_HALF_SIZE, 0.0, 0.0),
        });
    }
    out
}

pub struct CliffMeshData {
    pub vertices: Vec<Vector3>,
    pub normals: Vec<Vector3>,
    pub uvs: Vec<Vector2>,
    pub indices: Vec<i32>,
}

pub fn build_cliff_mesh(
    side: &CliffSide,
    coord: (i32, i32),
    config: &CornerConfig,
    terrain_noise: &FastNoiseLite,
    cliff_noise: &FastNoiseLite,
) -> CliffMeshData {
    let world_origin_x = coord.0 as f32 * PARCEL_SIZE + PARCEL_HALF_SIZE;
    let world_origin_z = -(coord.1 as f32 * PARCEL_SIZE + PARCEL_HALF_SIZE);
    let is_horizontal = side.outward_normal.z.abs() > 0.5;

    let row_stride = CLIFF_SEGMENTS + 1;
    let vertex_count = ((CLIFF_VERTICAL_SEGMENTS + 1) * row_stride) as usize;

    let mut positions: Vec<Vector3> = Vec::with_capacity(vertex_count);
    let mut uvs_raw: Vec<Vector2> = Vec::with_capacity(vertex_count);

    for v in 0..=CLIFF_VERTICAL_SEGMENTS {
        let v_ratio = v as f32 / CLIFF_VERTICAL_SEGMENTS as f32;
        let y_pos = -v_ratio * CLIFF_HEIGHT;

        for h in 0..=CLIFF_SEGMENTS {
            let h_ratio = h as f32 / CLIFF_SEGMENTS as f32;
            let horizontal_pos = (h_ratio - 0.5) * CLIFF_LENGTH;

            let (mut vertex_pos, world_x, world_z);
            let mut is_corner_edge = false;
            let mut corner_normal = Vector3::ZERO;

            if is_horizontal {
                vertex_pos = side.edge_position + Vector3::new(horizontal_pos, y_pos, 0.0);
                world_x = world_origin_x + horizontal_pos;
                world_z = world_origin_z + side.edge_position.z;

                if (horizontal_pos - (-PARCEL_HALF_SIZE)).abs() < 0.01
                    && config.west == ParcelState::Nothing
                {
                    is_corner_edge = true;
                    corner_normal = Vector3::new(-1.0, 0.0, side.outward_normal.z).normalized();
                } else if (horizontal_pos - PARCEL_HALF_SIZE).abs() < 0.01
                    && config.east == ParcelState::Nothing
                {
                    is_corner_edge = true;
                    corner_normal = Vector3::new(1.0, 0.0, side.outward_normal.z).normalized();
                }
            } else {
                vertex_pos = side.edge_position + Vector3::new(0.0, y_pos, horizontal_pos);
                world_x = world_origin_x + side.edge_position.x;
                world_z = world_origin_z + horizontal_pos;

                if (horizontal_pos - (-PARCEL_HALF_SIZE)).abs() < 0.01
                    && config.north == ParcelState::Nothing
                {
                    is_corner_edge = true;
                    corner_normal = Vector3::new(side.outward_normal.x, 0.0, -1.0).normalized();
                } else if (horizontal_pos - PARCEL_HALF_SIZE).abs() < 0.01
                    && config.south == ParcelState::Nothing
                {
                    is_corner_edge = true;
                    corner_normal = Vector3::new(side.outward_normal.x, 0.0, 1.0).normalized();
                }
            }

            let skip_displacement = on_empty_boundary_for_cliff(side, horizontal_pos, config);

            if y_pos.abs() < 0.01 {
                let (grid_x, grid_z, local_x, local_z) =
                    floor_edge_grid(is_horizontal, horizontal_pos, side.edge_position);
                vertex_pos = floor_edge_position(
                    grid_x,
                    grid_z,
                    local_x,
                    local_z,
                    world_x,
                    world_z,
                    config,
                    terrain_noise,
                    cliff_noise,
                );
            } else if is_corner_edge && !skip_displacement {
                let noise_value = cliff_noise.get_noise_2d(world_x, world_z);
                let displacement = noise_value * CLIFF_NOISE_STRENGTH;
                vertex_pos -= corner_normal * displacement;
            } else if !skip_displacement {
                let noise_value = cliff_noise.get_noise_2d(world_x, world_z);
                let displacement = noise_value * CLIFF_NOISE_STRENGTH;
                vertex_pos -= side.outward_normal * displacement;
            }

            positions.push(vertex_pos);
            uvs_raw.push(Vector2::new(h_ratio, v_ratio));
        }
    }

    let needs_reversed = side.outward_normal.z > 0.5 || side.outward_normal.x < -0.5;

    let mut indices_raw: Vec<i32> =
        Vec::with_capacity((CLIFF_VERTICAL_SEGMENTS * CLIFF_SEGMENTS * 6) as usize);
    for v in 0..CLIFF_VERTICAL_SEGMENTS {
        for h in 0..CLIFF_SEGMENTS {
            let idx = v * row_stride + h;
            let idx_next = (v + 1) * row_stride + h;

            if needs_reversed {
                indices_raw.extend_from_slice(&[idx, idx_next + 1, idx_next]);
                indices_raw.extend_from_slice(&[idx, idx + 1, idx_next + 1]);
            } else {
                indices_raw.extend_from_slice(&[idx, idx_next, idx_next + 1]);
                indices_raw.extend_from_slice(&[idx, idx_next + 1, idx + 1]);
            }
        }
    }

    let normals_raw = averaged_normals(&positions, &indices_raw);

    CliffMeshData {
        vertices: positions.into_iter().collect(),
        normals: normals_raw.into_iter().collect(),
        uvs: uvs_raw.into_iter().collect(),
        indices: indices_raw.into_iter().collect(),
    }
}

pub struct OverhangMeshData {
    pub vertices: Vec<Vector3>,
    pub normals: Vec<Vector3>,
    pub uvs: Vec<Vector2>,
    pub colors: Vec<Color>,
    pub indices: Vec<i32>,
}

pub fn build_overhang_mesh(
    side: &CliffSide,
    coord: (i32, i32),
    config: &CornerConfig,
    terrain_noise: &FastNoiseLite,
    cliff_noise: &FastNoiseLite,
) -> OverhangMeshData {
    let world_origin_x = coord.0 as f32 * PARCEL_SIZE + PARCEL_HALF_SIZE;
    let world_origin_z = -(coord.1 as f32 * PARCEL_SIZE + PARCEL_HALF_SIZE);
    let is_horizontal = side.outward_normal.z.abs() > 0.5;

    let vertex_count = ((OVERHANG_SEGMENTS + 1) * 2) as usize;
    let mut positions: Vec<Vector3> = Vec::with_capacity(vertex_count);
    let mut uvs_raw: Vec<Vector2> = Vec::with_capacity(vertex_count);
    let mut colors_raw: Vec<Color> = Vec::with_capacity(vertex_count);

    for h in 0..=OVERHANG_SEGMENTS {
        let h_ratio = h as f32 / OVERHANG_SEGMENTS as f32;
        let horizontal_pos = -PARCEL_HALF_SIZE + h_ratio * CLIFF_LENGTH;

        let (world_x, world_z, local_x, local_z, grid_x, grid_z, corner_x_opt, corner_z_opt) =
            overhang_vertex_coords(
                is_horizontal,
                horizontal_pos,
                world_origin_x,
                world_origin_z,
                side.edge_position,
            );

        let mut dir = side.outward_normal;
        if let (Some(corner_x), Some(corner_z)) = (corner_x_opt, corner_z_opt) {
            let corner_checks = [
                (
                    -PARCEL_HALF_SIZE,
                    -PARCEL_HALF_SIZE,
                    Vector3::new(-1.0, 0.0, -1.0).normalized(),
                    config.north,
                    config.west,
                ),
                (
                    PARCEL_HALF_SIZE,
                    -PARCEL_HALF_SIZE,
                    Vector3::new(1.0, 0.0, -1.0).normalized(),
                    config.north,
                    config.east,
                ),
                (
                    -PARCEL_HALF_SIZE,
                    PARCEL_HALF_SIZE,
                    Vector3::new(-1.0, 0.0, 1.0).normalized(),
                    config.south,
                    config.west,
                ),
                (
                    PARCEL_HALF_SIZE,
                    PARCEL_HALF_SIZE,
                    Vector3::new(1.0, 0.0, 1.0).normalized(),
                    config.south,
                    config.east,
                ),
            ];
            for (cx, cz, diag, edge_a, edge_b) in corner_checks {
                if (corner_x - cx).abs() < 0.01 && (corner_z - cz).abs() < 0.01 {
                    let both_cliffs =
                        edge_a == ParcelState::Nothing && edge_b == ParcelState::Nothing;
                    dir = if both_cliffs {
                        diag
                    } else {
                        side.outward_normal
                    };
                    break;
                }
            }
        }

        let inner_pos = floor_edge_position(
            grid_x,
            grid_z,
            local_x,
            local_z,
            world_x,
            world_z,
            config,
            terrain_noise,
            cliff_noise,
        );

        let outer_noise = cliff_noise.get_noise_2d(world_x * 1.5, world_z * 1.5);
        let outer_displacement = outer_noise * 0.3;
        let varied_distance = OVERHANG_DISTANCE + outer_displacement;
        let horizontal_displacement = dir * varied_distance;

        let mut outer_pos = inner_pos
            + Vector3::new(
                horizontal_displacement.x,
                -OVERHANG_DROOP,
                horizontal_displacement.z,
            );
        let droop_variation = cliff_noise.get_noise_2d(world_x * 2.0, world_z * 2.0) * 0.1;
        outer_pos.y -= droop_variation.abs();

        positions.push(inner_pos);
        uvs_raw.push(Vector2::new(0.0, h_ratio));
        colors_raw.push(Color::from_rgba(1.0, 1.0, 1.0, 1.0));

        positions.push(outer_pos);
        uvs_raw.push(Vector2::new(1.0, h_ratio));
        colors_raw.push(Color::from_rgba(0.0, 0.0, 0.0, 1.0));
    }

    let mut indices_raw: Vec<i32> = Vec::with_capacity((OVERHANG_SEGMENTS * 6) as usize);
    for h in 0..OVERHANG_SEGMENTS {
        let idx = h * 2;
        indices_raw.extend_from_slice(&[idx, idx + 2, idx + 3]);
        indices_raw.extend_from_slice(&[idx, idx + 3, idx + 1]);
    }

    let normals_raw = averaged_normals(&positions, &indices_raw);

    OverhangMeshData {
        vertices: positions.into_iter().collect(),
        normals: normals_raw.into_iter().collect(),
        uvs: uvs_raw.into_iter().collect(),
        colors: colors_raw.into_iter().collect(),
        indices: indices_raw.into_iter().collect(),
    }
}

fn on_empty_boundary_for_cliff(
    side: &CliffSide,
    horizontal_pos: f32,
    config: &CornerConfig,
) -> bool {
    let is_horizontal = side.outward_normal.z.abs() > 0.5;
    let epsilon = 0.01;
    let (left_state, right_state) = if is_horizontal {
        (config.west, config.east)
    } else {
        (config.north, config.south)
    };

    ((horizontal_pos - (-PARCEL_HALF_SIZE)).abs() < epsilon && left_state == ParcelState::Empty)
        || ((horizontal_pos - PARCEL_HALF_SIZE).abs() < epsilon
            && right_state == ParcelState::Empty)
}

fn floor_edge_grid(
    is_horizontal: bool,
    horizontal_pos: f32,
    edge_position: Vector3,
) -> (i32, i32, f32, f32) {
    if is_horizontal {
        let grid_x = ((horizontal_pos + PARCEL_HALF_SIZE) * 2.0).round() as i32;
        let grid_x = grid_x.clamp(0, GRID_SIZE);
        let grid_z = if (edge_position.z - (-PARCEL_HALF_SIZE)).abs() < 0.01 {
            0
        } else {
            GRID_SIZE
        };
        (grid_x, grid_z, horizontal_pos, edge_position.z)
    } else {
        let grid_z = ((horizontal_pos + PARCEL_HALF_SIZE) * 2.0).round() as i32;
        let grid_z = grid_z.clamp(0, GRID_SIZE);
        let grid_x = if (edge_position.x - PARCEL_HALF_SIZE).abs() < 0.01 {
            GRID_SIZE
        } else {
            0
        };
        (grid_x, grid_z, edge_position.x, horizontal_pos)
    }
}

type OverhangCoords = (f32, f32, f32, f32, i32, i32, Option<f32>, Option<f32>);

fn overhang_vertex_coords(
    is_horizontal: bool,
    horizontal_pos: f32,
    world_origin_x: f32,
    world_origin_z: f32,
    edge_position: Vector3,
) -> OverhangCoords {
    let epsilon = 0.01;
    let is_corner = (horizontal_pos - (-PARCEL_HALF_SIZE)).abs() < epsilon
        || (horizontal_pos - PARCEL_HALF_SIZE).abs() < epsilon;

    if is_horizontal {
        let (corner_x, corner_z) = if is_corner {
            (Some(horizontal_pos), Some(edge_position.z))
        } else {
            (None, None)
        };
        let world_x = world_origin_x + horizontal_pos;
        let world_z = world_origin_z + edge_position.z;
        let grid_x = ((horizontal_pos + PARCEL_HALF_SIZE) * 2.0).round() as i32;
        let grid_x = grid_x.clamp(0, GRID_SIZE);
        let grid_z = if (edge_position.z - (-PARCEL_HALF_SIZE)).abs() < 0.01 {
            0
        } else {
            GRID_SIZE
        };
        let local_x = horizontal_pos;
        let local_z = edge_position.z;
        (
            world_x, world_z, local_x, local_z, grid_x, grid_z, corner_x, corner_z,
        )
    } else {
        let (corner_x, corner_z) = if is_corner {
            (Some(edge_position.x), Some(horizontal_pos))
        } else {
            (None, None)
        };
        let world_x = world_origin_x + edge_position.x;
        let world_z = world_origin_z + horizontal_pos;
        let grid_z = ((horizontal_pos + PARCEL_HALF_SIZE) * 2.0).round() as i32;
        let grid_z = grid_z.clamp(0, GRID_SIZE);
        let grid_x = if (edge_position.x - PARCEL_HALF_SIZE).abs() < 0.01 {
            GRID_SIZE
        } else {
            0
        };
        let local_x = edge_position.x;
        let local_z = horizontal_pos;
        (
            world_x, world_z, local_x, local_z, grid_x, grid_z, corner_x, corner_z,
        )
    }
}

/// Floor height for a vertex exactly on a parcel edge. Matches what the terrain
/// mesh computes for its boundary row so cliff/overhang tops meet the ground
/// seamlessly.
#[allow(clippy::too_many_arguments)]
fn floor_edge_position(
    grid_x: i32,
    grid_z: i32,
    local_x: f32,
    local_z: f32,
    world_x: f32,
    world_z: f32,
    config: &CornerConfig,
    terrain_noise: &FastNoiseLite,
    cliff_noise: &FastNoiseLite,
) -> Vector3 {
    let mut position = Vector3::new(local_x, 0.0, local_z);

    let falloff = super::terrain::sample_falloff_public(local_x, local_z, config);
    let floor_noise = terrain_noise.get_noise_2d(world_x, world_z);
    let base_displacement = (floor_noise + 1.0) * 0.5 * TERRAIN_HEIGHT;
    position.y = base_displacement * falloff;

    let on_empty_boundary = (grid_z == 0 && config.north == ParcelState::Empty)
        || (grid_z == GRID_SIZE && config.south == ParcelState::Empty)
        || (grid_x == GRID_SIZE && config.east == ParcelState::Empty)
        || (grid_x == 0 && config.west == ParcelState::Empty);

    if on_empty_boundary {
        return position;
    }

    let mut cliff_normal = Vector3::ZERO;
    let mut has_cliff = false;

    let corner_checks = [
        (
            0,
            0,
            config.northwest,
            Vector3::new(-1.0, 0.0, -1.0).normalized(),
        ),
        (
            GRID_SIZE,
            0,
            config.northeast,
            Vector3::new(1.0, 0.0, -1.0).normalized(),
        ),
        (
            0,
            GRID_SIZE,
            config.southwest,
            Vector3::new(-1.0, 0.0, 1.0).normalized(),
        ),
        (
            GRID_SIZE,
            GRID_SIZE,
            config.southeast,
            Vector3::new(1.0, 0.0, 1.0).normalized(),
        ),
    ];

    for (cx, cz, state, normal) in corner_checks {
        if grid_x == cx && grid_z == cz && state == ParcelState::Nothing {
            cliff_normal = normal;
            has_cliff = true;
            break;
        }
    }

    if !has_cliff {
        let edge_checks = [
            (grid_z == 0, config.north, Vector3::new(0.0, 0.0, -1.0)),
            (
                grid_z == GRID_SIZE,
                config.south,
                Vector3::new(0.0, 0.0, 1.0),
            ),
            (
                grid_x == GRID_SIZE,
                config.east,
                Vector3::new(1.0, 0.0, 0.0),
            ),
            (grid_x == 0, config.west, Vector3::new(-1.0, 0.0, 0.0)),
        ];

        for (check, state, normal) in edge_checks {
            if check && state == ParcelState::Nothing {
                cliff_normal = normal;
                has_cliff = true;
                break;
            }
        }
    }

    if has_cliff {
        let cliff_value = cliff_noise.get_noise_2d(world_x, world_z);
        let displacement = cliff_value * CLIFF_NOISE_STRENGTH;
        position -= cliff_normal * displacement;
    }

    position
}

fn averaged_normals(positions: &[Vector3], indices: &[i32]) -> Vec<Vector3> {
    let mut normals = vec![Vector3::ZERO; positions.len()];
    for tri in indices.chunks_exact(3) {
        let (a, b, c) = (tri[0] as usize, tri[1] as usize, tri[2] as usize);
        let edge_ab = positions[b] - positions[a];
        let edge_ac = positions[c] - positions[a];
        let face = edge_ac.cross(edge_ab);
        normals[a] += face;
        normals[b] += face;
        normals[c] += face;
    }
    for n in normals.iter_mut() {
        let len = n.length();
        if len > f32::EPSILON {
            *n /= len;
        } else {
            *n = Vector3::new(0.0, 1.0, 0.0);
        }
    }
    normals
}
