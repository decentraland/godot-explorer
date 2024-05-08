use godot::{engine::node::InternalMode, prelude::*};

use crate::godot_classes::promise::Promise;

#[derive(GodotClass)]
#[class(base=Node)]
pub struct ResourceOwner {
    promise: Option<Gd<Promise>>,

    #[base]
    _base: Base<Node>,
}

#[godot_api]
impl ResourceOwner {
    fn init(base: Base<Node>) -> Self {
        Self {
            _base: base,
            promise: None,
        }
    }

    fn get_node_name(promise: &Gd<Promise>) -> GString {
        GString::from(format!(
            "ResourceOwner{}",
            promise.instance_id()
        ))
    }

    pub fn new_alloc(promise: Gd<Promise>) -> Gd<Self> {
        let name = ResourceOwner::get_node_name(&promise);
        let mut obj = Gd::from_init_fn(|_base| ResourceOwner {
            _base,
            promise: Some(promise),
        });
        obj.set_name(name);
        obj
    }

    pub fn add_to(mut owner: Gd<Node>, promise: Gd<Promise>) {
        let name = NodePath::from(ResourceOwner::get_node_name(&promise));
        if !owner.has_node(name) {
            let resource_owner = ResourceOwner::new_alloc(promise);
            owner.add_child_ex(resource_owner.upcast()).internal(InternalMode::INTERNAL_MODE_FRONT).done();
        }
    }

    pub fn has_promise(&self, promise: &Gd<Promise>) -> bool {
        if let Some(self_promise) = &self.promise {
            promise.instance_id() == self_promise.instance_id()
        } else {
            false
        }
    }

    #[func]
    pub fn release_ownership(base_node: Gd<Node>, promise: Gd<Promise>) {
        for child in base_node.get_children_ex().include_internal(true).done().iter_shared() {
            if let Ok(mut child) = child.try_cast::<ResourceOwner>() {
                if child.bind().has_promise(&promise) {
                    child.queue_free();
                }
            }
        }
    }
}
