use godot::prelude::*;

#[derive(GodotClass)]
#[class(init, base=Object)]
pub struct DclRequestState {
    #[base]
    _base: Base<Object>,
}

#[godot_api]
impl DclRequestState {
    #[signal]
    fn on_finish() {}
}
