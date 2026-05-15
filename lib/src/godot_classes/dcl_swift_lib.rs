use godot::prelude::*;

/// Typed wrapper around the Swift `DclSwiftLib` GDExtension class.
///
/// Mirrors the pattern used by [`DclIosPlugin`](super::dcl_ios_plugin::DclIosPlugin)
/// for the `DclGodotiOS` singleton: callers (Rust or GDScript) should go through
/// this wrapper instead of touching `ClassDB.instantiate("DclSwiftLib")` directly,
/// so the iOS / non-iOS branching lives in one place.
#[derive(GodotClass)]
#[class(init, base=RefCounted)]
pub struct DclSwiftLibPlugin {
    _base: Base<RefCounted>,
}

#[godot_api]
impl DclSwiftLibPlugin {
    /// Instantiate `DclSwiftLib` via ClassDB, returning `None` if the class is
    /// not registered (non-iOS desktop builds load the no-op stub instead of the
    /// real framework).
    fn try_instantiate() -> Option<Gd<Object>> {
        let class_name = StringName::from("DclSwiftLib");
        let class_db = godot::classes::ClassDb::singleton();
        if !class_db.class_exists(&class_name) {
            return None;
        }
        class_db
            .instantiate(&class_name)
            .try_to::<Gd<Object>>()
            .ok()
    }

    /// True only on iOS builds where the Swift framework is loaded and the
    /// `DclSwiftLib` class is registered in ClassDB.
    #[func]
    pub fn is_available() -> bool {
        #[cfg(target_os = "ios")]
        {
            Self::try_instantiate().is_some()
        }
        #[cfg(not(target_os = "ios"))]
        {
            false
        }
    }

    /// Round-trip smoke call. Returns `"ok"` on iOS, empty string elsewhere.
    #[func]
    pub fn ping() -> GString {
        let Some(mut instance) = Self::try_instantiate() else {
            return GString::new();
        };
        instance
            .call("ping", &[])
            .try_to::<GString>()
            .unwrap_or_default()
    }

    /// Returns the Swift library version string (e.g. `"0.1.0"`), empty on
    /// non-iOS or if the class is missing.
    #[func]
    pub fn version() -> GString {
        let Some(mut instance) = Self::try_instantiate() else {
            return GString::new();
        };
        instance
            .call("version", &[])
            .try_to::<GString>()
            .unwrap_or_default()
    }
}

/// Typed wrapper around the Swift `DclStoreKit` GDExtension class.
///
/// Sibling to [`DclSwiftLibPlugin`] but instance-based: StoreKit usage is
/// stateful (loaded-product cache + `Transaction.updates` listener task), so
/// one wrapper owns one long-lived Swift instance whose state must outlive
/// any single call. Re-emits the Swift class's 7 signals as typed Rust
/// signals so GDScript callers never touch `ClassDB.instantiate("DclStoreKit")`
/// directly. On non-iOS builds every method is a no-op and `is_available()`
/// returns false.
#[derive(GodotClass)]
#[class(init, base=RefCounted)]
pub struct DclStoreKitPlugin {
    inner: Option<Gd<Object>>,
    wired: bool,
    base: Base<RefCounted>,
}

#[godot_api]
impl DclStoreKitPlugin {
    #[signal]
    fn products_loaded(json: GString);

    #[signal]
    fn products_load_failed(error: GString);

    #[signal]
    fn purchase_completed(json: GString);

    #[signal]
    fn purchase_failed(product_id: GString, reason: GString);

    #[signal]
    fn purchase_cancelled(product_id: GString);

    #[signal]
    fn purchase_pending(product_id: GString);

    #[signal]
    fn transaction_updated(json: GString);

    fn ensure_inner(&mut self) -> bool {
        if self.inner.is_some() {
            return true;
        }
        let class_name = StringName::from("DclStoreKit");
        let class_db = godot::classes::ClassDb::singleton();
        if !class_db.class_exists(&class_name) {
            return false;
        }
        let Ok(instance) = class_db.instantiate(&class_name).try_to::<Gd<Object>>() else {
            godot_error!("[DclStoreKitPlugin] failed to instantiate DclStoreKit");
            return false;
        };
        self.inner = Some(instance);
        self.wire_signals();
        true
    }

    fn wire_signals(&mut self) {
        if self.wired || self.inner.is_none() {
            return;
        }
        let cb_products_loaded = self.base().callable("_on_products_loaded");
        let cb_products_load_failed = self.base().callable("_on_products_load_failed");
        let cb_purchase_completed = self.base().callable("_on_purchase_completed");
        let cb_purchase_failed = self.base().callable("_on_purchase_failed");
        let cb_purchase_cancelled = self.base().callable("_on_purchase_cancelled");
        let cb_purchase_pending = self.base().callable("_on_purchase_pending");
        let cb_transaction_updated = self.base().callable("_on_transaction_updated");

        let inner = self.inner.as_mut().unwrap();
        inner.connect("products_loaded", &cb_products_loaded);
        inner.connect("products_load_failed", &cb_products_load_failed);
        inner.connect("purchase_completed", &cb_purchase_completed);
        inner.connect("purchase_failed", &cb_purchase_failed);
        inner.connect("purchase_cancelled", &cb_purchase_cancelled);
        inner.connect("purchase_pending", &cb_purchase_pending);
        inner.connect("transaction_updated", &cb_transaction_updated);
        self.wired = true;
    }

    /// True only when the Swift `DclStoreKit` class is registered (iOS).
    /// First call also instantiates and wires signal forwarding.
    #[func]
    fn is_available(&mut self) -> bool {
        self.ensure_inner()
    }

    #[func]
    fn can_make_payments(&mut self) -> bool {
        if !self.ensure_inner() {
            return false;
        }
        self.inner
            .as_mut()
            .unwrap()
            .call("can_make_payments", &[])
            .try_to::<bool>()
            .unwrap_or(false)
    }

    #[func]
    fn start_listening(&mut self) {
        if !self.ensure_inner() {
            return;
        }
        self.inner.as_mut().unwrap().call("start_listening", &[]);
    }

    #[func]
    fn load_products(&mut self, product_ids: PackedStringArray) {
        if !self.ensure_inner() {
            return;
        }
        self.inner
            .as_mut()
            .unwrap()
            .call("load_products", &[product_ids.to_variant()]);
    }

    #[func]
    fn purchase(&mut self, product_id: GString, wallet_address: GString) {
        if !self.ensure_inner() {
            return;
        }
        self.inner.as_mut().unwrap().call(
            "purchase",
            &[product_id.to_variant(), wallet_address.to_variant()],
        );
    }

    #[func]
    fn finish_transaction(&mut self, transaction_id: GString) {
        if !self.ensure_inner() {
            return;
        }
        self.inner
            .as_mut()
            .unwrap()
            .call("finish_transaction", &[transaction_id.to_variant()]);
    }

    // Signal forwarders: must be `#[func]` so they're addressable via Callable.

    #[func]
    fn _on_products_loaded(&mut self, json: GString) {
        self.base_mut()
            .emit_signal("products_loaded", &[json.to_variant()]);
    }

    #[func]
    fn _on_products_load_failed(&mut self, error: GString) {
        self.base_mut()
            .emit_signal("products_load_failed", &[error.to_variant()]);
    }

    #[func]
    fn _on_purchase_completed(&mut self, json: GString) {
        self.base_mut()
            .emit_signal("purchase_completed", &[json.to_variant()]);
    }

    #[func]
    fn _on_purchase_failed(&mut self, product_id: GString, reason: GString) {
        self.base_mut().emit_signal(
            "purchase_failed",
            &[product_id.to_variant(), reason.to_variant()],
        );
    }

    #[func]
    fn _on_purchase_cancelled(&mut self, product_id: GString) {
        self.base_mut()
            .emit_signal("purchase_cancelled", &[product_id.to_variant()]);
    }

    #[func]
    fn _on_purchase_pending(&mut self, product_id: GString) {
        self.base_mut()
            .emit_signal("purchase_pending", &[product_id.to_variant()]);
    }

    #[func]
    fn _on_transaction_updated(&mut self, json: GString) {
        self.base_mut()
            .emit_signal("transaction_updated", &[json.to_variant()]);
    }
}
