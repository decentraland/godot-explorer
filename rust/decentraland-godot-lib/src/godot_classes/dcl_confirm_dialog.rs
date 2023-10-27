use godot::engine::{Button, Label, Panel, PanelVirtual};
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=Panel)]
pub struct DclConfirmDialog {
    #[base]
    base: Base<Panel>,

    title_label: Option<Gd<Label>>,
    description_label: Option<Gd<Label>>,
    ok_button: Option<Gd<Button>>,
    reject_button: Option<Gd<Button>>,

    confirm_callback: Option<Box<dyn FnOnce(bool)>>,
}

impl DclConfirmDialog {
    pub fn set_confirm_callback<F>(&mut self, confirm_callback: F)
    where
        F: FnOnce(bool) + 'static,
    {
        self.confirm_callback = Some(Box::new(confirm_callback));
    }

    pub fn set_texts(
        &mut self,
        title: &str,
        description: &str,
        ok_button_text: &str,
        reject_button_text: &str,
    ) {
        if let Some(title_label) = &mut self.title_label {
            title_label.set_text(GodotString::from(title));
        }

        if let Some(description_label) = &mut self.description_label {
            description_label.set_text(GodotString::from(description));
        }

        if let Some(ok_button) = &mut self.ok_button {
            ok_button.set_text(GodotString::from(ok_button_text));
        }

        if let Some(reject_button) = &mut self.reject_button {
            reject_button.set_text(GodotString::from(reject_button_text));
        }
    }
}

#[godot_api]
impl DclConfirmDialog {
    #[func]
    fn _on_ok_pressed(&mut self) {
        if let Some(confirm_callback) = self.confirm_callback.take() {
            confirm_callback(true);
        }
        self.base.hide();
    }

    #[func]
    fn _on_reject_pressed(&mut self) {
        if let Some(confirm_callback) = self.confirm_callback.take() {
            confirm_callback(false);
        }
        self.base.hide();
    }
}

#[godot_api]
impl PanelVirtual for DclConfirmDialog {
    fn init(base: Base<Panel>) -> Self {
        Self {
            base,
            confirm_callback: None,
            title_label: None,
            description_label: None,
            ok_button: None,
            reject_button: None,
        }
    }

    fn ready(&mut self) {
        let mut ok_button = self
            .base
            .get_node("%OkButton".into())
            .expect("Missing %OkButton")
            .cast::<Button>();
        let mut reject_button = self
            .base
            .get_node("%RejectButton".into())
            .expect("Missing %RejectButton")
            .cast::<Button>();

        ok_button.connect("pressed".into(), self.base.callable("_on_ok_pressed"));
        reject_button.connect("pressed".into(), self.base.callable("_on_reject_pressed"));

        self.title_label = Some(
            self.base
                .get_node("%Title".into())
                .expect("Missing %Title")
                .cast::<Label>(),
        );
        self.description_label = Some(
            self.base
                .get_node("%Description".into())
                .expect("Missing %Description")
                .cast::<Label>(),
        );
        self.ok_button = Some(ok_button);
        self.reject_button = Some(reject_button);
    }
}
