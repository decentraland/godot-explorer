pub mod cliffs;
pub mod props;
pub mod terrain;

use std::f32::consts::TAU;

use godot::builtin::{Basis, PackedByteArray, Rid, Transform3D, Vector3};
use godot::global::godot_warn;

pub const PARCEL_SIZE: f32 = 16.0;
pub const PARCEL_HALF_SIZE: f32 = 8.0;
pub const PARCEL_HEIGHT_BOUND: f32 = 100.0;
pub const PARCEL_FULL_HEIGHT: f32 = 200.0;
pub const TERRAIN_HEIGHT: f32 = 3.0;
pub const FALLOFF_DISTANCE: f32 = 8.0;
pub const GRID_SIZE: i32 = 32;
pub const CELL_SIZE: f32 = 0.5;

pub const TERRAIN_NOISE_SEED: i32 = 12345;
pub const TERRAIN_NOISE_FREQUENCY: f32 = 0.05;
pub const CLIFF_NOISE_SEED: i32 = 54321;
pub const CLIFF_NOISE_FREQUENCY: f32 = 0.3;

pub const TERRAIN_MATERIAL_PATH: &str = "res://assets/empty-scenes/empty_parcel_material.tres";
pub const CLIFF_MATERIAL_PATH: &str = "res://assets/empty-scenes/cliff_material.tres";
pub const OVERHANG_MATERIAL_PATH: &str =
    "res://assets/empty-scenes/empty_parcel_grass_overhang_material.tres";
pub const GRASS_BLADE_MESH_PATH: &str = "res://assets/empty-scenes/grass_blade.tres";
pub const GRASS_BLADES_MATERIAL_PATH: &str = "res://assets/empty-scenes/grass_blades_material.tres";

pub const GRASS_BASE_SCALE: f32 = 1.5;
pub const GRASS_CULLING_RANGE: i32 = 1;

/// Values MUST stay 0/1/2 to match `CornerConfiguration.ParcelState` in GDScript.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ParcelState {
    Empty = 0,
    Nothing = 1,
    Loaded = 2,
}

impl ParcelState {
    fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Empty),
            1 => Some(Self::Nothing),
            2 => Some(Self::Loaded),
            _ => None,
        }
    }
}

/// Packed layout when received from GDScript: [N, S, E, W, NW, NE, SW, SE].
#[derive(Clone, Copy, Debug)]
pub struct CornerConfig {
    pub north: ParcelState,
    pub south: ParcelState,
    pub east: ParcelState,
    pub west: ParcelState,
    pub northwest: ParcelState,
    pub northeast: ParcelState,
    pub southwest: ParcelState,
    pub southeast: ParcelState,
}

impl Default for CornerConfig {
    fn default() -> Self {
        Self {
            north: ParcelState::Nothing,
            south: ParcelState::Nothing,
            east: ParcelState::Nothing,
            west: ParcelState::Nothing,
            northwest: ParcelState::Nothing,
            northeast: ParcelState::Nothing,
            southwest: ParcelState::Nothing,
            southeast: ParcelState::Nothing,
        }
    }
}

impl CornerConfig {
    pub fn has_any_out_of_bounds_neighbor(&self) -> bool {
        self.north == ParcelState::Nothing
            || self.south == ParcelState::Nothing
            || self.east == ParcelState::Nothing
            || self.west == ParcelState::Nothing
    }

    pub fn from_packed(buffer: &PackedByteArray, offset: usize) -> Option<Self> {
        Some(Self {
            north: ParcelState::from_u8(buffer.get(offset)?)?,
            south: ParcelState::from_u8(buffer.get(offset + 1)?)?,
            east: ParcelState::from_u8(buffer.get(offset + 2)?)?,
            west: ParcelState::from_u8(buffer.get(offset + 3)?)?,
            northwest: ParcelState::from_u8(buffer.get(offset + 4)?)?,
            northeast: ParcelState::from_u8(buffer.get(offset + 5)?)?,
            southwest: ParcelState::from_u8(buffer.get(offset + 6)?)?,
            southeast: ParcelState::from_u8(buffer.get(offset + 7)?)?,
        })
    }
}

/// One is generated per triangle during terrain construction. Consumed by
/// grass/prop spawners to place surface instances on the generated mesh.
#[derive(Clone, Copy, Debug)]
pub struct SpawnLocation {
    pub position: Vector3,
    pub normal: Vector3,
    pub falloff: f32,
}

pub struct PendingPhysicsGeometry {
    pub vertices: Vec<Vector3>,
    pub indices: Vec<i32>,
}

pub struct ParcelData {
    pub terrain_mesh: Rid,
    pub terrain_instance: Rid,
    pub collision_body: Rid,
    pub collision_shape: Rid,

    pub pending_physics_geometry: Option<PendingPhysicsGeometry>,

    pub cliff_side_meshes: Vec<Rid>,
    pub cliff_side_instances: Vec<Rid>,

    pub grass_multimesh: Rid,
    pub grass_instance: Rid,
    pub grass_visible: bool,

    pub prop_instances: Vec<Rid>,
    pub prop_bodies: Vec<Rid>,

    pub config: CornerConfig,

    pub stale_since_msec: Option<u64>,
}

impl Default for ParcelData {
    fn default() -> Self {
        Self {
            terrain_mesh: Rid::Invalid,
            terrain_instance: Rid::Invalid,
            collision_body: Rid::Invalid,
            collision_shape: Rid::Invalid,
            pending_physics_geometry: None,
            cliff_side_meshes: Vec::new(),
            cliff_side_instances: Vec::new(),
            stale_since_msec: None,
            grass_multimesh: Rid::Invalid,
            grass_instance: Rid::Invalid,
            grass_visible: false,
            prop_instances: Vec::new(),
            prop_bodies: Vec::new(),
            config: CornerConfig::default(),
        }
    }
}

/// Port of `ParcelUtils.create_aligned_transform` from GDScript. Builds a
/// transform whose local +Y aligns with `normal`, optionally adding a random
/// yaw around that normal (pass `rng=Some(&mut ...)` for determinism).
pub fn aligned_transform(
    position: Vector3,
    normal: Vector3,
    rng: Option<&mut SimpleRng>,
    scale: f32,
) -> Transform3D {
    let mut basis = Basis::IDENTITY;

    if (normal - Vector3::UP).length_squared() > f32::EPSILON {
        let rotation_axis = Vector3::UP.cross(normal).normalized();
        if rotation_axis.length() > 0.001 {
            let angle = Vector3::UP.angle_to(normal);
            basis = Basis::from_axis_angle(rotation_axis, angle) * basis;
        }
    }

    if let Some(rng) = rng {
        let yaw = rng.next_f32() * TAU;
        basis = Basis::from_axis_angle(normal, yaw) * basis;
    }

    if (scale - 1.0).abs() > f32::EPSILON {
        basis = basis.scaled(Vector3::ONE * scale);
    }

    Transform3D::new(basis, position)
}

/// Deterministic xorshift64 RNG. Seeded from parcel coords so repeated
/// materializations of the same parcel produce identical placements.
pub struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    pub fn new(seed: (u32, u32)) -> Self {
        let s = ((seed.0 as u64) << 32) | (seed.1 as u64);
        let mixed = s
            .wrapping_mul(0x9E37_79B9_7F4A_7C15)
            .wrapping_add(0x1234_5678_9ABC_DEF0);
        Self { state: mixed | 1 }
    }

    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        (x >> 32) as u32
    }

    pub fn next_f32(&mut self) -> f32 {
        (self.next_u32() as f32) / (u32::MAX as f32)
    }
}

pub fn warn_invalid_corner_config(index: usize) {
    godot_warn!(
        "[DclFloatingIslandsManager] invalid ParcelState byte in corner_configs near parcel \
         index {index}, skipping"
    );
}
