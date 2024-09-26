use godot::{engine::node::InternalMode, prelude::*};

#[derive(GodotClass)]
#[class(base=Node)]
pub struct ResourceLocker {
    #[export]
    reference: Gd<Resource>,

    base: Base<Node>,
}

#[godot_api]
impl INode for ResourceLocker {
    fn init(mut base: Base<Node>) -> Self {
        let reference = Resource::new_gd();

        // TODO: set meta
        // base.set_name("ResourceLocker".to_godot());
        // base.set_meta(
        //     StringName::from("instance_id"),
        //     reference.instance_id().to_variant(),
        // );
        Self { base, reference }
    }
}

#[godot_api]
impl ResourceLocker {
    #[func]
    pub fn attach_to(mut node: Gd<Node>) {
        if node.has_node(NodePath::from("ResourceLocker")) {
            tracing::error!("You cannot attach two times ResourceLocker to a Node");
            return;
        }

        let resource_locker = ResourceLocker::new_alloc();

        node.add_child_ex(resource_locker.upcast())
            .internal(InternalMode::FRONT)
            .done();
    }

    #[func]
    pub fn get_reference_count(&self) -> i32 {
        self.reference.get_reference_count()
    }

    #[func]
    pub fn get_reference_id(&self) -> i64 {
        self.reference.instance_id().to_i64()
    }
}
