use std::collections::HashMap;

use godot::{
    engine::{node::InternalMode, BoxMesh, CylinderMesh, MeshInstance3D, PlaneMesh, SphereMesh},
    prelude::*,
};

use crate::dcl::{
    components::{
        proto_components::sdk::components::pb_mesh_renderer, SceneComponentId, SceneEntityId,
    },
    crdt::{last_write_wins::LastWriteWinsComponentOperation, SceneCrdtStateProtoComponents},
    DclScene, DirtyComponents, DirtyEntities, RendererResponse, SceneDefinition, SceneId,
    SceneResponse,
};

// Deriving GodotClass makes the class available to Godot
#[derive(GodotClass)]
#[class(base=Node)]
pub struct MainTestClass {
    #[base]
    base: Base<Node>,
    next_scene_tick: f64,
    scenes: HashMap<SceneId, GodotDclScene>,

    thread_sender_to_main: std::sync::mpsc::SyncSender<SceneResponse>,
    main_receiver_from_thread: std::sync::mpsc::Receiver<SceneResponse>,
}

pub struct GodotDclScene {
    pub dcl_scene: DclScene,
    pub entities: HashMap<SceneEntityId, Gd<Node3D>>,
    pub root_node: Gd<Node3D>,
    pub in_flight: bool,
}

impl GodotDclScene {
    pub fn new(thread_sender_to_main: std::sync::mpsc::SyncSender<SceneResponse>) -> Self {
        let root_node = Node3D::new_alloc();
        let dcl_scene = DclScene::spawn_new(
            SceneDefinition {
                path: "cube_wave".to_string(),
                offset: godot::prelude::Vector3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                visible: true,
            },
            thread_sender_to_main,
        );

        let entities = HashMap::from([(SceneEntityId::new(0, 0), root_node.share())]);

        GodotDclScene {
            dcl_scene,
            entities,
            root_node,
            in_flight: false,
        }
    }

    pub fn ensure_node(&mut self, entity: &SceneEntityId) -> Gd<Node3D> {
        let maybe_node = self.entities.get(entity);
        if maybe_node.is_some() {
            let value = self.entities.get_mut(entity);
            value.unwrap().share()
        } else {
            let new_node = Node3D::new_alloc();

            self.root_node.add_child(
                new_node.share().upcast(),
                false,
                InternalMode::INTERNAL_MODE_DISABLED,
            );

            self.entities.insert(*entity, new_node.share());

            new_node.share()
        }
    }
}

#[godot_api]
impl MainTestClass {
    #[func]
    fn start_scene(&mut self) {
        let new_scene = GodotDclScene::new(self.thread_sender_to_main.clone());

        self.base.add_child(
            new_scene.root_node.share().upcast(),
            false,
            InternalMode::INTERNAL_MODE_DISABLED,
        );

        self.scenes.insert(new_scene.dcl_scene.scene_id, new_scene);
    }

    fn scene_process(&mut self, delta: f64) {
        self.next_scene_tick -= delta;

        self.process_scenes(delta);

        for (_id, scene) in self.scenes.iter_mut() {
            if !scene.in_flight {
                let crdt = scene.dcl_scene.scene_crdt.clone();
                let mut crdt_state = crdt.lock().unwrap();
                let dirty = crdt_state.take_dirty();
                drop(crdt_state);

                if let Err(e) = scene
                    .dcl_scene
                    .main_sender_to_thread
                    .blocking_send(RendererResponse::Ok(dirty))
                {
                    println!("failed to send updates to scene: {e:?}");
                    // TODO: clean up
                } else {
                    // Something?
                    scene.in_flight = true;
                }
            }
        }
    }

    fn process_scenes(&mut self, delta: f64) {
        // TODO: check infinity loop (loop_end_time)
        loop {
            match self.main_receiver_from_thread.try_recv() {
                Ok(response) => {
                    // println!("received message from scene");
                    match response {
                        SceneResponse::Error(scene_id, msg) => {
                            println!("[{scene_id:?}] error: {msg}");
                        }
                        SceneResponse::Ok(scene_id, (dirty_entities, dirty_components)) => {
                            // println!("scene {:?} OkCrdt", scene_id);

                            if let Some(scene) = self.scenes.get_mut(&scene_id) {
                                update_scene(delta, scene, dirty_entities, dirty_components);
                            }
                        }
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => return,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    panic!("render thread receiver exploded");
                }
            }
        }
    }
}

fn update_scene(
    _dt: f64,
    scene: &mut GodotDclScene,
    _dirty_entities: DirtyEntities,
    dirty_components: DirtyComponents,
) {
    let crdt = scene.dcl_scene.scene_crdt.clone();
    let crdt_state = crdt.lock().unwrap();
    let transform_component = crdt_state.get_transform();

    if let Some(transform_dirty) = dirty_components.get(&SceneComponentId::TRANSFORM) {
        for entity in transform_dirty {
            let value = transform_component.get(*entity);
            let mut node = scene.ensure_node(entity);
            if let Some(entry) = value {
                if let Some(transform) = entry.value.clone() {
                    node.set_rotation(transform.rotation.to_euler(EulerOrder::XYZ));
                    node.set_position(transform.translation);
                    node.set_scale(transform.scale);
                }
            }
        }
    }

    if let Some(mesh_renderer_dirty) = dirty_components.get(&SceneComponentId::MESH_RENDERER) {
        let mesh_renderer_component = SceneCrdtStateProtoComponents::get_mesh_renderer(&crdt_state);

        for entity in mesh_renderer_dirty {
            let new_value = mesh_renderer_component.get(*entity);
            if new_value.is_none() {
                continue;
            }

            let new_value = new_value.unwrap();
            let mut node = scene.ensure_node(entity);

            let new_value = new_value.value.clone();
            let existing = node.try_get_node_as::<MeshInstance3D>(NodePath::from("MeshRenderer"));

            if new_value.is_none() {
                if existing.is_some() {
                    // remove
                }
            } else if let Some(new_value) = new_value {
                if let Some(_existing) = existing {
                    // update
                } else {
                    // create
                    let mut new_mesh_instance_3d = MeshInstance3D::new_alloc();

                    match new_value.mesh {
                        Some(mesh) => match mesh {
                            pb_mesh_renderer::Mesh::Box(_box_mesh) => {
                                let new_box_mesh = BoxMesh::new();
                                new_mesh_instance_3d.set_mesh(new_box_mesh.upcast());

                                // update the material (and with uvs)
                            }
                            pb_mesh_renderer::Mesh::Sphere(_sphere_mesh) => {
                                let new_sphere_mesh = SphereMesh::new();
                                new_mesh_instance_3d.set_mesh(new_sphere_mesh.upcast());

                                // update the material
                            }
                            pb_mesh_renderer::Mesh::Cylinder(cylinder_mesh) => {
                                let mut new_cylinder_mesh = CylinderMesh::new();
                                new_cylinder_mesh
                                    .set_top_radius(cylinder_mesh.radius_top.unwrap_or(0.5) as f64);
                                new_cylinder_mesh.set_bottom_radius(
                                    cylinder_mesh.radius_bottom.unwrap_or(0.5) as f64,
                                );
                                new_cylinder_mesh.set_height(1.0);
                                new_mesh_instance_3d.set_mesh(new_cylinder_mesh.upcast());

                                // update the material
                            }
                            pb_mesh_renderer::Mesh::Plane(_plane_mesh) => {
                                let new_plane_mesh = PlaneMesh::new();
                                new_mesh_instance_3d.set_mesh(new_plane_mesh.upcast());

                                // update the material (and with uvs)
                            }
                        },
                        _ => {
                            let new_box_mesh = BoxMesh::new();
                            new_mesh_instance_3d.set_mesh(new_box_mesh.upcast());
                        }
                    }

                    new_mesh_instance_3d.set_name(GodotString::from("MeshRenderer"));
                    node.add_child(
                        new_mesh_instance_3d.share().upcast(),
                        false,
                        InternalMode::INTERNAL_MODE_DISABLED,
                    );
                }
            }
        }
    }

    scene.in_flight = false;
    drop(crdt_state)
}

#[godot_api]
impl NodeVirtual for MainTestClass {
    fn init(base: Base<Node>) -> Self {
        let (thread_sender_to_main, main_receiver_from_thread) =
            std::sync::mpsc::sync_channel(1000);

        MainTestClass {
            base,
            next_scene_tick: 0.0,
            scenes: HashMap::new(),
            main_receiver_from_thread,
            thread_sender_to_main,
        }
    }

    fn ready(&mut self) {
        // Note: this is downcast during load() -- completely type-safe thanks to type inference!
        // If the resource does not exist or has an incompatible type, this panics.
        // There is also try_load() if you want to check whether loading succeeded.
    }

    fn process(&mut self, delta: f64) {
        self.scene_process(delta);
    }
}
