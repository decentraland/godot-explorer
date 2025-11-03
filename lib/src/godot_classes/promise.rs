use godot::prelude::*;

#[derive(GodotClass)]
#[class(init, base=Object)]
pub struct PromiseError {
    pub error_description: GString,
    base: Base<Object>,
}

#[godot_api]
impl PromiseError {
    #[func]
    fn get_error(&self) -> GString {
        self.error_description.clone()
    }
}

impl PromiseError {
    fn new(error_description: GString) -> Gd<Self> {
        let mut promise_error = Self::new_alloc();
        promise_error.bind_mut().error_description = error_description;
        promise_error
    }
}

#[derive(GodotClass)]
#[class(init, base=Object)]
pub struct Promise {
    resolved: bool,
    data: Variant,

    base: Base<Object>,
}

#[godot_api]
impl Promise {
    #[signal]
    pub fn on_resolved();

    #[func]
    pub fn resolve(&mut self) {
        if self.is_resolved() {
            return;
        }
        self.resolved = true;
        self.base_mut()
            .call_deferred("emit_signal", &["on_resolved".to_variant()]);
    }

    #[func]
    pub fn resolve_with_data(&mut self, data: Variant) {
        if self.is_resolved() {
            return;
        }
        self.data = data;
        self.resolve();
    }

    #[func]
    pub fn reject(&mut self, reason: GString) {
        if self.is_resolved() {
            return;
        }
        self.data = PromiseError::new(reason).to_variant();
        self.resolve();
    }

    #[func]
    pub fn get_data(&self) -> Variant {
        self.data.clone()
    }

    #[func]
    pub fn is_resolved(&self) -> bool {
        self.resolved
    }

    #[func]
    pub fn is_rejected(&self) -> bool {
        self.data.try_to::<Gd<PromiseError>>().is_ok()
    }

    pub fn preset_data(&mut self, data: Variant) {
        self.data = data;
    }

    pub fn make_to_async() -> (Gd<Promise>, impl Fn() -> Option<Gd<Promise>>) {
        let this_promise = Promise::new_alloc();
        let promise_instance_id = this_promise.instance_id();
        let get_promise = move || Gd::<Promise>::try_from_instance_id(promise_instance_id).ok();
        (this_promise, get_promise)
    }

    pub fn from_resolved(data: Variant) -> Gd<Self> {
        Gd::from_init_fn(|base| Self {
            resolved: true,
            data,
            base,
        })
    }

    pub fn from_rejected(reason: String) -> Gd<Self> {
        let mut data = PromiseError::new_alloc();
        data.bind_mut().error_description = GString::from(reason);

        Gd::from_init_fn(|base| Self {
            resolved: true,
            data: data.to_variant(),
            base,
        })
    }
}
