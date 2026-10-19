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
    fn on_resolved();

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
        data.bind_mut().error_description = GString::from(&reason);

        Gd::from_init_fn(|base| Self {
            resolved: true,
            data: data.to_variant(),
            base,
        })
    }
}

/// Resolve or reject a promise from a thread other than the main thread.
///
/// Never `bind_mut()` a promise off the main thread. GDScript polls promises
/// (`is_resolved()` / `get_data()`) from the main thread, and godot-cell 0.4.5's
/// `MutGuardBlocking::drop` clears the "mutably bound" state and notifies the
/// waiting reader WITHOUT holding the tracker lock: a main-thread `bind()` that
/// checks the state just before that drop misses the wake-up and sleeps forever,
/// which freezes the whole client (observed in the scene-test harness after a
/// few thousand HTTP responses). Deferring the call runs the mutation on the
/// main thread, where no cross-thread borrow can exist.
pub trait PromiseDeferred {
    fn resolve_deferred(&mut self);
    fn resolve_with_data_deferred(&mut self, data: Variant);
    fn reject_deferred(&mut self, reason: GString);
}

impl PromiseDeferred for Gd<Promise> {
    fn resolve_deferred(&mut self) {
        self.call_deferred("resolve", &[]);
    }

    fn resolve_with_data_deferred(&mut self, data: Variant) {
        self.call_deferred("resolve_with_data", &[data]);
    }

    fn reject_deferred(&mut self, reason: GString) {
        self.call_deferred("reject", &[reason.to_variant()]);
    }
}
