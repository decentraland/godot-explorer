use godot::classes::object::ConnectFlags;
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

    /// Logging self-test for the **Swift stack**: invokes `DclSwiftLib.testLogging`
    /// which logs at every level via every Swift form (print / NSLog / os_log /
    /// fputs / GD.print). No-op (empty) on non-iOS. Called from GDScript's
    /// `_run_logging_selftest()`.
    #[func]
    pub fn test_logging() -> GString {
        let Some(mut instance) = Self::try_instantiate() else {
            return GString::new();
        };
        instance
            .call("test_logging", &[])
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
    pub fn products_loaded(json: GString);

    #[signal]
    pub fn products_load_failed(error: GString);

    #[signal]
    pub fn purchase_completed(json: GString);

    #[signal]
    pub fn purchase_failed(product_id: GString, reason: GString);

    #[signal]
    pub fn purchase_cancelled(product_id: GString);

    #[signal]
    pub fn purchase_pending(product_id: GString);

    #[signal]
    pub fn transaction_updated(json: GString);

    /// `(environment, source, resolve_ms)` — emitted exactly once after
    /// `resolve_environment()`, guaranteed even on timeout. `environment` is
    /// `"production"` | `"sandbox"` | `"xcode"` | `"unknown"`; `source` tells how
    /// it was determined; `resolve_ms` is how long `AppTransaction` took.
    #[signal]
    pub fn environment_resolved(environment: GString, source: GString, resolve_ms: f64);

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
        // The Swift side is not a class of this crate, so its signals are connected by name; the
        // handlers are typed closures over this node.
        let me = self.to_gd();
        let forward = |name: &'static str, handler: fn(&mut Self, &[&Variant])| {
            let mut me = me.clone();
            Callable::from_fn(name, move |args| {
                handler(&mut me.bind_mut(), args);
                Variant::nil()
            })
        };
        let handlers = [
            (
                "products_loaded",
                forward("products_loaded", |this, a| {
                    this._on_products_loaded(a[0].to())
                }),
            ),
            (
                "products_load_failed",
                forward("products_load_failed", |this, a| {
                    this._on_products_load_failed(a[0].to())
                }),
            ),
            (
                "purchase_completed",
                forward("purchase_completed", |this, a| {
                    this._on_purchase_completed(a[0].to())
                }),
            ),
            (
                "purchase_failed",
                forward("purchase_failed", |this, a| {
                    this._on_purchase_failed(a[0].to(), a[1].to())
                }),
            ),
            (
                "purchase_cancelled",
                forward("purchase_cancelled", |this, a| {
                    this._on_purchase_cancelled(a[0].to())
                }),
            ),
            (
                "purchase_pending",
                forward("purchase_pending", |this, a| {
                    this._on_purchase_pending(a[0].to())
                }),
            ),
            (
                "transaction_updated",
                forward("transaction_updated", |this, a| {
                    this._on_transaction_updated(a[0].to())
                }),
            ),
            (
                "environment_resolved",
                forward("environment_resolved", |this, a| {
                    this._on_environment_resolved(a[0].to(), a[1].to(), a[2].to())
                }),
            ),
        ];

        // The Swift signals are emitted from background `Task`s (StoreKit runs its async work
        // off the main thread). Connect DEFERRED so the forwarders run on the main thread,
        // where scene-tree mutations are safe.
        let inner = self.inner.as_mut().unwrap();
        for (signal, callable) in &handlers {
            inner.connect_flags(*signal, callable, ConnectFlags::DEFERRED);
        }
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

    /// Synchronous best-effort StoreKit environment: `"sandbox"`,
    /// `"production"` or `"unknown"`, read instantly from the receipt URL.
    /// Safe to call at startup before picking the credits backend. Empty
    /// string on non-iOS. For the authoritative value (and to distinguish the
    /// Xcode StoreKit-config environment) use [`Self::resolve_environment`].
    #[func]
    fn current_environment(&mut self) -> GString {
        if !self.ensure_inner() {
            return GString::new();
        }
        self.inner
            .as_mut()
            .unwrap()
            .call("current_environment", &[])
            .try_to::<GString>()
            .unwrap_or_default()
    }

    /// Authoritative StoreKit environment via `AppTransaction.shared`. Resolves
    /// asynchronously; connect to the `environment_resolved(environment,
    /// source)` signal for the result. No-op on non-iOS.
    #[func]
    fn resolve_environment(&mut self) {
        if !self.ensure_inner() {
            return;
        }
        self.inner
            .as_mut()
            .unwrap()
            .call("resolve_environment", &[]);
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
        self.signals().products_loaded().emit(&json);
    }

    #[func]
    fn _on_products_load_failed(&mut self, error: GString) {
        self.signals().products_load_failed().emit(&error);
    }

    #[func]
    fn _on_purchase_completed(&mut self, json: GString) {
        self.signals().purchase_completed().emit(&json);
    }

    #[func]
    fn _on_purchase_failed(&mut self, product_id: GString, reason: GString) {
        self.signals().purchase_failed().emit(&product_id, &reason);
    }

    #[func]
    fn _on_purchase_cancelled(&mut self, product_id: GString) {
        self.signals().purchase_cancelled().emit(&product_id);
    }

    #[func]
    fn _on_purchase_pending(&mut self, product_id: GString) {
        self.signals().purchase_pending().emit(&product_id);
    }

    #[func]
    fn _on_transaction_updated(&mut self, json: GString) {
        self.signals().transaction_updated().emit(&json);
    }

    #[func]
    fn _on_environment_resolved(&mut self, environment: GString, source: GString, resolve_ms: f64) {
        self.signals()
            .environment_resolved()
            .emit(&environment, &source, resolve_ms);
    }
}
