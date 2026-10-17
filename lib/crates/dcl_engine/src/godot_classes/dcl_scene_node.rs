use godot::prelude::*;

#[derive(GodotClass)]
#[class(init, base=Node3D)]
pub struct DclSceneNode {
    scene_id: i32,

    is_global: bool,

    pub last_tick_number: i32,

    // two properties to track the loading progress of the gltf
    pub max_gltf_loaded_count: i32,
    pub gltf_loading_count: i32,

    _base: Base<Node3D>,
}

#[godot_api]
impl DclSceneNode {
    #[signal]
    pub fn tree_changed();

    pub fn new_alloc(scene_id: i32, is_global: bool) -> Gd<Self> {
        let mut obj = Gd::from_init_fn(|_base| {
            // accepts the base and returns a constructed object containing it
            DclSceneNode {
                _base,
                scene_id,
                is_global,
                last_tick_number: -1,
                max_gltf_loaded_count: 0,
                gltf_loading_count: 0,
            }
        });
        obj.set_name(&format!("scene_id_{:?}", scene_id.clone()));
        obj
    }

    /// The scene node `target` lives under (itself included).
    pub fn search(target: &Gd<Node3D>) -> Option<Gd<DclSceneNode>> {
        let mut node = Some(target.clone());
        while let Some(current) = node {
            if let Ok(scene) = current.clone().try_cast::<DclSceneNode>() {
                return Some(scene);
            }
            node = current.get_parent_node_3d();
        }
        None
    }

    #[func]
    pub fn get_scene_id(&self) -> i32 {
        self.scene_id
    }

    #[func]
    pub fn is_global(&self) -> bool {
        self.is_global
    }

    #[func]
    fn get_last_tick_number(&self) -> i32 {
        self.last_tick_number
    }

    #[func]
    fn get_gltf_loading_progress(&self) -> f32 {
        if self.max_gltf_loaded_count == 0 {
            return 1.0;
        }
        1.0 - (self.gltf_loading_count as f32 / self.max_gltf_loaded_count as f32)
    }
}
