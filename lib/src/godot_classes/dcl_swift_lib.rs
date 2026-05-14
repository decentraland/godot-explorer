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
