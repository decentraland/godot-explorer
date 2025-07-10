use godot::{classes::node::InternalMode, prelude::*};

#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct ResourceLocker {
    #[var]
    reference: Gd<Resource>,

    base: Base<Node>,
}

#[godot_api]
impl ResourceLocker {
    #[func]
    pub fn attach_to(mut node: Gd<Node>) {
        if node.has_node("ResourceLocker") {
            tracing::error!("You cannot attach two times ResourceLocker to a Node");
            return;
        }

        let mut resource_locker = Gd::from_init_fn(|base| {
            let reference = Resource::new_gd();
            ResourceLocker { reference, base }
        });

        let instance_id = resource_locker.bind().reference.instance_id().to_variant();
        resource_locker.set_name("ResourceLocker");
        resource_locker.set_meta("instance_id", &instance_id);

        node.add_child_ex(&resource_locker.upcast::<Node>())
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
