use godot::prelude::*;

use crate::engine::local_task::defer_from_any_thread;

#[derive(GodotClass)]
#[class(init, base=Object)]
pub struct PromiseError {
    pub error_description: GString,
    base: Base<Object>,
}

#[godot_api]
impl PromiseError {
    #[func]
    pub fn get_error(&self) -> GString {
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
        // Settled from content threads too: the signal goes out on the main thread, where the
        // promise is looked up again by its id.
        let id = self.base().instance_id();
        defer_from_any_thread(&[], move |_| {
            if let Ok(promise) = Gd::<Promise>::try_from_instance_id(id) {
                promise.signals().on_resolved().emit();
            }
        });
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
/// Never `bind_mut()` a promise off the main thread: a main-thread `bind()` racing the guard's
/// drop can miss the wake-up and sleep forever (seen in the scene-test harness after a few
/// thousand HTTP responses). These settle it on the main thread, looked up by id.
pub trait PromiseDeferred {
    fn resolve_deferred(&mut self);
    fn resolve_with_data_deferred(&mut self, data: Variant);
    fn reject_deferred(&mut self, reason: GString);
}

fn settle_on_main(promise: &Gd<Promise>, args: &[Variant], f: fn(&mut Promise, &[&Variant])) {
    let id = promise.instance_id();
    defer_from_any_thread(args, move |args| {
        if let Ok(mut promise) = Gd::<Promise>::try_from_instance_id(id) {
            f(&mut promise.bind_mut(), args);
        }
    });
}

impl PromiseDeferred for Gd<Promise> {
    fn resolve_deferred(&mut self) {
        settle_on_main(self, &[], |promise, _| promise.resolve());
    }

    fn resolve_with_data_deferred(&mut self, data: Variant) {
        settle_on_main(self, &[data], |promise, args| {
            promise.resolve_with_data(args[0].clone())
        });
    }

    fn reject_deferred(&mut self, reason: GString) {
        settle_on_main(self, &[reason.to_variant()], |promise, args| {
            promise.reject(args[0].to::<GString>())
        });
    }
}
