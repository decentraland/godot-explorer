use godot::engine::Node3D;
use godot::prelude::*;

use crate::dcl::components::SceneEntityId;

#[derive(GodotClass)]
#[class(init, base=Node3D)]
pub struct DclNodeEntity3d {
    entity_id: i32,
    #[base]
    _base: Base<Node3D>,
}

#[godot_api]
impl DclNodeEntity3d {
    pub fn new_alloc(entity_id: SceneEntityId) -> Gd<Self> {
        let entity_id = entity_id.as_i32();
        let mut obj = Gd::from_init_fn(|_base| DclNodeEntity3d { _base, entity_id });
        obj.set_name(GString::from(format!("e{:x}", entity_id)));
        obj
    }

    #[func]
    fn e_id(&self) -> i32 {
        self.entity_id
    }
}
