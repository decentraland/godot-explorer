use godot::engine::Node3D;
use godot::prelude::*;

#[derive(GodotClass)]
#[class(init, base=Node3D)]
pub struct DclSceneNode {
    scene_id: i32,

    is_global: bool,

    pub last_tick_number: i32,

    #[base]
    _base: Base<Node3D>,
}

#[godot_api]
impl DclSceneNode {
    pub fn new_alloc(scene_id: i32, is_global: bool) -> Gd<Self> {
        let mut obj = Gd::with_base(|_base| {
            // accepts the base and returns a constructed object containing it
            DclSceneNode {
                _base,
                scene_id,
                is_global,
                last_tick_number: -1,
            }
        });
        obj.set_name(GodotString::from(format!(
            "scene_id_{:?}",
            scene_id.clone()
        )));
        obj
    }

    #[func]
    fn get_scene_id(&self) -> i32 {
        self.scene_id
    }

    #[func]
    fn is_global(&self) -> bool {
        self.is_global
    }
}
