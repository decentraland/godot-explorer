//! Cached registry of Scene-UI Controls that block pointer input.
//!
//! A `DclUiControl` registers itself here whenever its `mouse_filter` becomes
//! `STOP` (either via `pointerFilter: 'block'` or because it has an
//! `onMouseDown`/`onMouseUp` listener attached) and unregisters on the reverse
//! transition. We deliberately do NOT unregister on `exit_tree`: `scene_ui`
//! reparents Controls during tree reorganization, and `exit_tree` fires on
//! every reparent even though the Control is still alive. Freed Controls are
//! pruned lazily in `blocks_point` via `is_instance_valid()`.
//!
//! `SceneManager::get_current_mouse_entity` consults the registry once per
//! frame to gate the 3D raycast: if any visible blocker contains the screen
//! point, no entity is reported as under the cursor. O(blockers), not O(all
//! Scene UI controls).
//!
//! Thread-local because Godot's main loop is single-threaded; this avoids
//! plumbing an `Rc<RefCell<…>>` through every `DclUiControl` creation site.

use std::cell::RefCell;
use std::collections::HashMap;

use godot::builtin::Vector2;
use godot::classes::Control;
use godot::obj::{Gd, InstanceId};

#[derive(Default)]
pub struct UiBlockerRegistry {
    blockers: HashMap<InstanceId, Gd<Control>>,
}

impl UiBlockerRegistry {
    pub fn register(&mut self, id: InstanceId, ctrl: Gd<Control>) {
        self.blockers.insert(id, ctrl);
    }

    pub fn unregister(&mut self, id: InstanceId) {
        self.blockers.remove(&id);
    }

    /// Returns true if any registered, visible blocker's global rect contains
    /// `point`. Prunes freed entries before iterating.
    pub fn blocks_point(&mut self, point: Vector2) -> bool {
        self.blockers.retain(|_, c| c.is_instance_valid());
        self.blockers
            .values()
            .any(|c| c.is_visible_in_tree() && c.get_global_rect().contains_point(point))
    }
}

thread_local! {
    static REGISTRY: RefCell<UiBlockerRegistry> = RefCell::new(UiBlockerRegistry::default());
}

pub fn register_blocker(id: InstanceId, ctrl: Gd<Control>) {
    REGISTRY.with(|r| r.borrow_mut().register(id, ctrl));
}

pub fn unregister_blocker(id: InstanceId) {
    REGISTRY.with(|r| r.borrow_mut().unregister(id));
}

pub fn blocks_point(point: Vector2) -> bool {
    REGISTRY.with(|r| r.borrow_mut().blocks_point(point))
}
