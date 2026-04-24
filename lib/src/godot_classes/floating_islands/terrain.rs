use fastnoise_lite::{FastNoiseLite, FractalType, NoiseType};
use godot::builtin::{Vector2, Vector3};

use super::{
    CornerConfig, ParcelState, SimpleRng, SpawnLocation, CELL_SIZE, CLIFF_NOISE_FREQUENCY,
    CLIFF_NOISE_SEED, FALLOFF_DISTANCE, GRID_SIZE, PARCEL_HALF_SIZE, PARCEL_SIZE, TERRAIN_HEIGHT,
    TERRAIN_NOISE_FREQUENCY, TERRAIN_NOISE_SEED,
};

pub fn build_terrain_noise() -> FastNoiseLite {
    configure_noise(TERRAIN_NOISE_SEED, TERRAIN_NOISE_FREQUENCY)
}

pub fn build_cliff_noise() -> FastNoiseLite {
    configure_noise(CLIFF_NOISE_SEED, CLIFF_NOISE_FREQUENCY)
}

fn configure_noise(seed: i32, frequency: f32) -> FastNoiseLite {
    let mut noise = FastNoiseLite::with_seed(seed);
    noise.set_noise_type(Some(NoiseType::OpenSimplex2S));
    noise.set_frequency(Some(frequency));
    noise.set_fractal_type(Some(FractalType::FBm));
    noise.set_fractal_octaves(Some(3));
    noise.set_fractal_lacunarity(Some(2.0));
    noise.set_fractal_gain(Some(0.5));
    noise
}

pub struct TerrainMeshData {
    pub vertices: Vec<Vector3>,
    pub normals: Vec<Vector3>,
    pub uvs: Vec<Vector2>,
    pub indices: Vec<i32>,
    pub spawn_locations: Vec<SpawnLocation>,
}

/// Build a parcel's ground mesh in LOCAL space (range `-8..8`). World coords
/// are still used for noise sampling so adjacent parcels line up at their
/// shared boundary.
pub fn build_terrain_mesh(
    coord: (i32, i32),
    config: &CornerConfig,
    terrain_noise: &FastNoiseLite,
    cliff_noise: &FastNoiseLite,
) -> TerrainMeshData {
    let world_origin_x = coord.0 as f32 * PARCEL_SIZE + PARCEL_HALF_SIZE;
    let world_origin_z = -(coord.1 as f32 * PARCEL_SIZE + PARCEL_HALF_SIZE);

    let grid = GRID_SIZE;
    let side = (grid + 1) as usize;
    let grid_vert_count = side * side;
    let quad_count = (grid * grid) as usize;
    let grid_index_count = quad_count * 6;

    let mut vertices: Vec<Vector3> = Vec::with_capacity(grid_vert_count + 16);
    let mut normals: Vec<Vector3> = Vec::with_capacity(grid_vert_count + 16);
    let mut uvs: Vec<Vector2> = Vec::with_capacity(grid_vert_count + 16);
    let mut indices: Vec<i32> = Vec::with_capacity(grid_index_count + 24);

    for z in 0..=grid {
        for x in 0..=grid {
            let x_pos = -PARCEL_HALF_SIZE + x as f32 * CELL_SIZE;
            let z_pos = -PARCEL_HALF_SIZE + z as f32 * CELL_SIZE;
            let v = displaced_vertex(
                x_pos,
                z_pos,
                world_origin_x + x_pos,
                world_origin_z + z_pos,
                x,
                z,
                config,
                terrain_noise,
                cliff_noise,
            );
            vertices.push(v);
            normals.push(Vector3::ZERO);
            uvs.push(Vector2::new(x as f32 / grid as f32, z as f32 / grid as f32));
        }
    }

    let idx_of = |x: i32, z: i32| (z as usize * side + x as usize) as i32;

    let mut spawn_locations = Vec::with_capacity(quad_count * 2);
    let mut rng = SimpleRng::new((coord.0 as u32, coord.1 as u32));

    for z in 0..grid {
        for x in 0..grid {
            let i1 = idx_of(x, z);
            let i2 = idx_of(x + 1, z);
            let i3 = idx_of(x + 1, z + 1);
            let i4 = idx_of(x, z + 1);

            let v1 = vertices[i1 as usize];
            let v2 = vertices[i2 as usize];
            let v3 = vertices[i3 as usize];
            let v4 = vertices[i4 as usize];

            let n1 = face_normal(v1, v2, v3);
            let n2 = face_normal(v1, v3, v4);

            normals[i1 as usize] += n1;
            normals[i2 as usize] += n1;
            normals[i3 as usize] += n1;
            normals[i1 as usize] += n2;
            normals[i3 as usize] += n2;
            normals[i4 as usize] += n2;

            indices.push(i1);
            indices.push(i2);
            indices.push(i3);
            indices.push(i1);
            indices.push(i3);
            indices.push(i4);

            let point1 = random_point_in_triangle(&mut rng, v1, v2, v3);
            let falloff1 = sample_falloff_at(point1.x, point1.z, config);
            spawn_locations.push(SpawnLocation {
                position: point1,
                normal: n1,
                falloff: falloff1,
            });

            let point2 = random_point_in_triangle(&mut rng, v1, v3, v4);
            let falloff2 = sample_falloff_at(point2.x, point2.z, config);
            spawn_locations.push(SpawnLocation {
                position: point2,
                normal: n2,
                falloff: falloff2,
            });
        }
    }

    for n in normals.iter_mut() {
        *n = if n.length_squared() > 0.0 {
            n.normalized()
        } else {
            Vector3::UP
        };
    }

    append_loaded_edge_strips(&mut vertices, &mut normals, &mut uvs, &mut indices, config);

    TerrainMeshData {
        vertices,
        normals,
        uvs,
        indices,
        spawn_locations,
    }
}

fn face_normal(a: Vector3, b: Vector3, c: Vector3) -> Vector3 {
    (c - a).cross(b - a).normalized()
}

/// Regenerates the grass/prop spawn locations from the stored terrain grid
/// using the same RNG seed as `build_terrain_mesh`, so lazy re-promotion
/// produces identical placements. Only iterates the first `GRID_SIZE²` quads —
/// edge strip triangles are appended after and never seed grass/props.
pub fn derive_spawn_locations(
    coord: (i32, i32),
    config: &CornerConfig,
    vertices: &[Vector3],
    indices: &[i32],
) -> Vec<SpawnLocation> {
    let grid = GRID_SIZE as usize;
    let grid_index_count = grid * grid * 6;
    if indices.len() < grid_index_count {
        return Vec::new();
    }

    let mut rng = SimpleRng::new((coord.0 as u32, coord.1 as u32));
    let mut out = Vec::with_capacity(grid * grid * 2);

    for tri in indices[..grid_index_count].chunks_exact(3) {
        let a = vertices[tri[0] as usize];
        let b = vertices[tri[1] as usize];
        let c = vertices[tri[2] as usize];
        let normal = face_normal(a, b, c);
        let position = random_point_in_triangle(&mut rng, a, b, c);
        let falloff = sample_falloff_at(position.x, position.z, config);
        out.push(SpawnLocation {
            position,
            normal,
            falloff,
        });
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn displaced_vertex(
    local_x: f32,
    local_z: f32,
    world_x: f32,
    world_z: f32,
    grid_x: i32,
    grid_z: i32,
    config: &CornerConfig,
    terrain_noise: &FastNoiseLite,
    cliff_noise: &FastNoiseLite,
) -> Vector3 {
    let grid = GRID_SIZE;
    let on_empty_boundary = (grid_z == 0 && config.north == ParcelState::Empty)
        || (grid_z == grid && config.south == ParcelState::Empty)
        || (grid_x == grid && config.east == ParcelState::Empty)
        || (grid_x == 0 && config.west == ParcelState::Empty);

    let mut is_edge_vertex = false;
    let mut cliff_normal = Vector3::ZERO;

    if !on_empty_boundary {
        if grid_z == 0 && config.north == ParcelState::Nothing {
            is_edge_vertex = true;
            cliff_normal = Vector3::new(0.0, 0.0, -1.0);
        }
        if grid_z == grid && config.south == ParcelState::Nothing {
            is_edge_vertex = true;
            cliff_normal = Vector3::new(0.0, 0.0, 1.0);
        }
        if grid_x == grid && config.east == ParcelState::Nothing {
            is_edge_vertex = true;
            cliff_normal = Vector3::new(1.0, 0.0, 0.0);
        }
        if grid_x == 0 && config.west == ParcelState::Nothing {
            is_edge_vertex = true;
            cliff_normal = Vector3::new(-1.0, 0.0, 0.0);
        }

        if grid_x == 0 && grid_z == 0 && config.northwest == ParcelState::Nothing {
            cliff_normal = Vector3::new(-1.0, 0.0, -1.0).normalized();
        } else if grid_x == grid && grid_z == 0 && config.northeast == ParcelState::Nothing {
            cliff_normal = Vector3::new(1.0, 0.0, -1.0).normalized();
        } else if grid_x == 0 && grid_z == grid && config.southwest == ParcelState::Nothing {
            cliff_normal = Vector3::new(-1.0, 0.0, 1.0).normalized();
        } else if grid_x == grid && grid_z == grid && config.southeast == ParcelState::Nothing {
            cliff_normal = Vector3::new(1.0, 0.0, 1.0).normalized();
        }
    }

    if is_edge_vertex && cliff_normal != Vector3::ZERO {
        let noise_value = cliff_noise.get_noise_2d(world_x, world_z);
        let cliff_noise_strength = 0.8_f32;
        let cliff_displacement = noise_value * cliff_noise_strength;
        return Vector3::new(local_x, 0.0, local_z) - cliff_normal * cliff_displacement;
    }

    let noise_value = terrain_noise.get_noise_2d(world_x, world_z);
    let base_displacement = (noise_value + 1.0) * 0.5 * TERRAIN_HEIGHT;
    let falloff = sample_falloff_at(local_x, local_z, config);
    let displacement = base_displacement * falloff;

    Vector3::new(local_x, displacement, local_z)
}

pub(super) fn sample_falloff_public(local_x: f32, local_z: f32, config: &CornerConfig) -> f32 {
    sample_falloff_at(local_x, local_z, config)
}

fn sample_falloff_at(local_x: f32, local_z: f32, config: &CornerConfig) -> f32 {
    let local_pos = Vector2::new(local_x, local_z);
    let mut min_distance = PARCEL_SIZE;

    min_distance = min_distance_for_corners(local_pos, min_distance, config);
    min_distance = min_distance_for_edges(local_pos, min_distance, config);

    let t = (min_distance / FALLOFF_DISTANCE).clamp(0.0, 1.0);
    smoothstep(t)
}

fn min_distance_for_corners(local_pos: Vector2, current_min: f32, config: &CornerConfig) -> f32 {
    let corners = [
        (
            config.northwest,
            Vector2::new(-PARCEL_HALF_SIZE, -PARCEL_HALF_SIZE),
            config.north,
            config.west,
        ),
        (
            config.northeast,
            Vector2::new(PARCEL_HALF_SIZE, -PARCEL_HALF_SIZE),
            config.north,
            config.east,
        ),
        (
            config.southwest,
            Vector2::new(-PARCEL_HALF_SIZE, PARCEL_HALF_SIZE),
            config.south,
            config.west,
        ),
        (
            config.southeast,
            Vector2::new(PARCEL_HALF_SIZE, PARCEL_HALF_SIZE),
            config.south,
            config.east,
        ),
    ];

    let mut result = current_min;
    for (corner_state, corner_pos, edge_a, edge_b) in corners {
        if corner_state == ParcelState::Loaded
            && edge_a == ParcelState::Empty
            && edge_b == ParcelState::Empty
        {
            let dist = (local_pos - corner_pos).length();
            if dist < result {
                result = dist;
            }
        }
    }
    result
}

fn min_distance_for_edges(local_pos: Vector2, current_min: f32, config: &CornerConfig) -> f32 {
    let edges = [
        (config.north, local_pos.y + PARCEL_HALF_SIZE),
        (config.south, PARCEL_HALF_SIZE - local_pos.y),
        (config.east, PARCEL_HALF_SIZE - local_pos.x),
        (config.west, local_pos.x + PARCEL_HALF_SIZE),
    ];

    let mut result = current_min;
    for (edge_state, distance) in edges {
        if edge_state != ParcelState::Empty && distance < result {
            result = distance;
        }
    }
    result
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn append_loaded_edge_strips(
    vertices: &mut Vec<Vector3>,
    normals: &mut Vec<Vector3>,
    uvs: &mut Vec<Vector2>,
    indices: &mut Vec<i32>,
    config: &CornerConfig,
) {
    let base_floor_y = -0.05_f32;
    let terrain_top_y = 0.0_f32;
    let start = -PARCEL_HALF_SIZE;
    let end = PARCEL_HALF_SIZE;

    if config.north == ParcelState::Loaded {
        let z = start;
        let bl = Vector3::new(start, base_floor_y, z);
        let br = Vector3::new(end, base_floor_y, z);
        let tr = Vector3::new(end, terrain_top_y, z);
        let tl = Vector3::new(start, terrain_top_y, z);
        push_quad(vertices, normals, uvs, indices, bl, br, tr, tl);
    }
    if config.south == ParcelState::Loaded {
        let z = end;
        let bl = Vector3::new(start, base_floor_y, z);
        let br = Vector3::new(end, base_floor_y, z);
        let tr = Vector3::new(end, terrain_top_y, z);
        let tl = Vector3::new(start, terrain_top_y, z);
        push_quad(vertices, normals, uvs, indices, tl, tr, br, bl);
    }
    if config.east == ParcelState::Loaded {
        let x = end;
        let near_b = Vector3::new(x, base_floor_y, start);
        let far_b = Vector3::new(x, base_floor_y, end);
        let far_t = Vector3::new(x, terrain_top_y, end);
        let near_t = Vector3::new(x, terrain_top_y, start);
        push_quad(
            vertices, normals, uvs, indices, near_b, far_b, far_t, near_t,
        );
    }
    if config.west == ParcelState::Loaded {
        let x = start;
        let near_b = Vector3::new(x, base_floor_y, start);
        let far_b = Vector3::new(x, base_floor_y, end);
        let far_t = Vector3::new(x, terrain_top_y, end);
        let near_t = Vector3::new(x, terrain_top_y, start);
        push_quad(
            vertices, normals, uvs, indices, near_t, far_t, far_b, near_b,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn push_quad(
    vertices: &mut Vec<Vector3>,
    normals: &mut Vec<Vector3>,
    uvs: &mut Vec<Vector2>,
    indices: &mut Vec<i32>,
    v1: Vector3,
    v2: Vector3,
    v3: Vector3,
    v4: Vector3,
) {
    let uv = |v: Vector3| {
        Vector2::new(
            (v.x + PARCEL_HALF_SIZE) / PARCEL_SIZE,
            (v.z + PARCEL_HALF_SIZE) / PARCEL_SIZE,
        )
    };

    // Flat normals on the vertical edge strips: the two triangles share a
    // plane, so averaging gives the same vector — store the face normal
    // directly on each unique vertex.
    let n = face_normal(v1, v2, v3);
    let base = vertices.len() as i32;

    for vert in [v1, v2, v3, v4] {
        vertices.push(vert);
        normals.push(n);
        uvs.push(uv(vert));
    }

    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

fn random_point_in_triangle(rng: &mut SimpleRng, v1: Vector3, v2: Vector3, v3: Vector3) -> Vector3 {
    let mut r1 = rng.next_f32();
    let mut r2 = rng.next_f32();
    if r1 + r2 > 1.0 {
        r1 = 1.0 - r1;
        r2 = 1.0 - r2;
    }
    v1 + (v2 - v1) * r1 + (v3 - v1) * r2
}
