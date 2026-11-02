use godot::classes::{
    BoxMesh, BoxShape3D, CollisionShape3D, IStaticBody3D, MeshInstance3D, StandardMaterial3D,
    StaticBody3D,
};
use godot::prelude::*;

/// Layer 2 is what the player collides with.
const WALL_LAYER: u32 = 2;

#[derive(GodotClass)]
#[class(base=StaticBody3D)]
pub struct InvisibleWall {
    base: Base<StaticBody3D>,
    wall_size: Vector3,
    box_shape: Option<Gd<BoxShape3D>>,
}

#[godot_api]
impl IStaticBody3D for InvisibleWall {
    fn init(base: Base<StaticBody3D>) -> Self {
        Self {
            base,
            wall_size: Vector3::new(1.0, 10.0, 1.0),
            box_shape: None,
        }
    }

    fn ready(&mut self) {
        self.ensure_collision();
    }
}

impl InvisibleWall {
    fn ensure_collision(&mut self) {
        if self.box_shape.is_some() {
            return;
        }
        self.base_mut().set_collision_layer(WALL_LAYER);
        self.base_mut().set_collision_mask(0);

        let mut shape = BoxShape3D::new_gd();
        shape.set_size(self.wall_size);
        let mut collision = CollisionShape3D::new_alloc();
        collision.set_name("WallCollision");
        collision.set_shape(&shape);
        self.base_mut().add_child(&collision);
        self.box_shape = Some(shape);
    }

    pub fn configure(&mut self, size: Vector3, position: Vector3) {
        self.wall_size = size;
        self.ensure_collision();
        if let Some(shape) = &mut self.box_shape {
            shape.set_size(size);
        }
        self.base_mut().set_global_position(position);
    }

    pub fn show_debug_mesh(&mut self) {
        self.remove_debug_mesh();
        let mut mesh = BoxMesh::new_gd();
        mesh.set_size(self.wall_size);
        let mut material = StandardMaterial3D::new_gd();
        material.set_albedo(Color::from_rgba(1.0, 0.0, 0.0, 0.3));
        material.set_transparency(godot::classes::base_material_3d::Transparency::ALPHA);
        material.set_flag(
            godot::classes::base_material_3d::Flags::DISABLE_DEPTH_TEST,
            true,
        );
        let mut instance = MeshInstance3D::new_alloc();
        instance.set_name("DebugMesh");
        instance.set_mesh(&mesh);
        instance.set_material_override(&material);
        self.base_mut().add_child(&instance);
    }

    pub fn remove_debug_mesh(&mut self) {
        if let Some(mut mesh) = self.base().get_node_or_null("DebugMesh") {
            mesh.queue_free();
        }
    }
}

/// The four invisible walls around the floating islands.
#[derive(GodotClass)]
#[class(init, base=Node3D)]
pub struct FloatingIslandWalls {
    base: Base<Node3D>,
    active_walls: Vec<Gd<InvisibleWall>>,
    debug_mode: bool,
}

const WALL_HEIGHT: f32 = 120.0;
const WALL_THICKNESS: f32 = 2.0;
const PARCEL: f32 = 16.0;

#[godot_api]
impl INode3D for FloatingIslandWalls {
    fn exit_tree(&mut self) {
        self.clear_walls();
    }
}

#[godot_api]
impl FloatingIslandWalls {
    #[func]
    pub fn create_walls_for_bounds(
        &mut self,
        scene_min_x: i32,
        scene_max_x: i32,
        scene_min_z: i32,
        scene_max_z: i32,
        padding: i32,
    ) {
        self.clear_walls();

        let min_x = scene_min_x - padding;
        let max_x = scene_max_x + padding;
        let min_z = scene_min_z - padding;
        let max_z = scene_max_z + padding;

        let width = (max_x - min_x + 1) as f32 * PARCEL;
        let depth = (max_z - min_z + 1) as f32 * PARCEL;
        // Parcels span coord*16..coord*16+16 and Godot's z is flipped.
        let center_x = (min_x + max_x + 1) as f32 * 8.0;
        let center_z = -((min_z + max_z + 1) as f32) * 8.0;
        let y = -WALL_HEIGHT / 2.0 + 10.0;

        let walls = [
            (
                "NorthWall",
                Vector3::new(center_x, y, -(min_z as f32 * PARCEL) + WALL_THICKNESS / 2.0),
                Vector3::new(width, WALL_HEIGHT, WALL_THICKNESS),
            ),
            (
                "SouthWall",
                Vector3::new(
                    center_x,
                    y,
                    -((max_z + 1) as f32 * PARCEL) - WALL_THICKNESS / 2.0,
                ),
                Vector3::new(width, WALL_HEIGHT, WALL_THICKNESS),
            ),
            (
                "WestWall",
                Vector3::new(min_x as f32 * PARCEL - WALL_THICKNESS / 2.0, y, center_z),
                Vector3::new(WALL_THICKNESS, WALL_HEIGHT, depth),
            ),
            (
                "EastWall",
                Vector3::new(
                    (max_x + 1) as f32 * PARCEL + WALL_THICKNESS / 2.0,
                    y,
                    center_z,
                ),
                Vector3::new(WALL_THICKNESS, WALL_HEIGHT, depth),
            ),
        ];
        for (name, position, size) in walls {
            let mut wall = InvisibleWall::new_alloc();
            wall.set_name(name);
            self.base_mut().add_child(&wall);
            wall.bind_mut().configure(size, position);
            if self.debug_mode {
                wall.bind_mut().show_debug_mesh();
            }
            self.active_walls.push(wall);
        }
    }

    #[func]
    pub fn clear_walls(&mut self) {
        for mut wall in std::mem::take(&mut self.active_walls) {
            if wall.is_instance_valid() {
                if let Some(mut parent) = wall.get_parent() {
                    parent.remove_child(&wall);
                }
                wall.queue_free();
            }
        }
    }

    #[func]
    pub fn set_debug_mode(&mut self, enabled: bool) {
        self.debug_mode = enabled;
        for wall in &mut self.active_walls {
            if enabled {
                wall.bind_mut().show_debug_mesh();
            } else {
                wall.bind_mut().remove_debug_mesh();
            }
        }
    }

    #[func]
    pub fn get_wall_count(&self) -> i64 {
        self.active_walls.len() as i64
    }
}
