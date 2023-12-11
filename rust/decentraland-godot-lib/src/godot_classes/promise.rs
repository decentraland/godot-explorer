use godot::engine::RefCounted;
use godot::prelude::*;

#[derive(GodotClass)]
#[class(init, base=RefCounted)]
pub struct PromiseError {
    pub error_description: GString,
    #[base]
    base: Base<RefCounted>,
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
        let mut promise_error = Self::new_gd();
        promise_error.bind_mut().error_description = error_description;
        promise_error
    }
}

#[derive(GodotClass)]
#[class(init, base=RefCounted)]
pub struct Promise {
    resolved: bool,
    data: Variant,

    #[base]
    base: Base<RefCounted>,
}

#[godot_api]
impl Promise {
    #[signal]
    pub fn on_resolved(&self) {}

    #[func]
    pub fn resolve(&mut self) {
        if self.is_resolved() {
            return;
        }
        self.resolved = true;
        self.base
            .call_deferred("emit_signal".into(), &["on_resolved".to_variant()]);
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
}
