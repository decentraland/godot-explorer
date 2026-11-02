//! What the controls ask of the app root (`Global`, still a GDScript autoload): orientation, the
//! device safe area, explorer focus and its signals.

use godot::prelude::*;

use crate::godot_classes::dcl_global::DclGlobal;

fn global() -> Gd<DclGlobal> {
    DclGlobal::singleton()
}

pub fn is_portrait() -> bool {
    DclGlobal::is_orientation_portrait()
}

pub fn is_mobile() -> bool {
    global().bind().is_mobile
}

/// Whether a safe area applies: a phone, or a desktop window emulating one.
pub fn has_safe_area() -> bool {
    is_mobile() || DclGlobal::is_emulating_safe_area()
}

/// The safe area in window pixels.
pub fn safe_area() -> Rect2i {
    DclGlobal::get_safe_area()
}

pub fn explorer_has_focus() -> bool {
    DclGlobal::explorer_has_focus()
}

pub fn explorer_grab_focus() {
    DclGlobal::explorer_grab_focus();
}

/// Connects a `Global` signal to a method of `target`.
pub fn connect(signal: &str, target: &Gd<impl Inherits<Object>>, method: &str) {
    let callable = Callable::from_object_method(target, method);
    global().connect(signal, &callable);
}

/// Connects a `Global` signal to a closure that takes the signal's arguments.
pub fn connect_fn(signal: &str, handler: impl FnMut(&[&Variant]) + 'static) {
    let mut handler = handler;
    let callable = Callable::from_fn(format!("host_{signal}"), move |args| {
        handler(args);
        Variant::nil()
    });
    global().connect(signal, &callable);
}
