use godot::classes::{Button, Control, IControl, Label};
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=Control)]
pub struct DclConfirmDialog {
    base: Base<Control>,

    title_label: Option<Gd<Label>>,
    description_label: Option<Gd<Label>>,
    ok_button: Option<Gd<Button>>,
    reject_button: Option<Gd<Button>>,

    confirm_callback: Option<Box<dyn FnOnce(bool)>>,
}

impl DclConfirmDialog {
    pub fn setup<F>(
        &mut self,
        title: &str,
        description: &str,
        ok_button_text: &str,
        reject_button_text: &str,
        confirm_callback: F,
    ) where
        F: FnOnce(bool) + 'static,
    {
        self.confirm_callback = Some(Box::new(confirm_callback));

        if let Some(title_label) = self.title_label.as_mut() {
            let title = GString::from(title);
            title_label.set_text(title.clone());
            self.base_mut().set_name(title);
        }

        if let Some(description_label) = &mut self.description_label {
            description_label.set_text(GString::from(description));
        }

        if let Some(ok_button) = &mut self.ok_button {
            ok_button.set_text(GString::from(ok_button_text));
        }

        if let Some(reject_button) = &mut self.reject_button {
            reject_button.set_text(GString::from(reject_button_text));
        }

        self.base_mut().show();
    }
}

#[godot_api]
impl DclConfirmDialog {
    #[func]
    fn _on_ok_pressed(&mut self) {
        if let Some(confirm_callback) = self.confirm_callback.take() {
            confirm_callback(true);
        }
        self.base_mut().queue_free();
    }

    #[func]
    fn _on_reject_pressed(&mut self) {
        if let Some(confirm_callback) = self.confirm_callback.take() {
            confirm_callback(false);
        }
        self.base_mut().queue_free();
    }
}

#[godot_api]
impl IControl for DclConfirmDialog {
    fn init(base: Base<Control>) -> Self {
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
            .base()
            .get_node_or_null("%OkButton".into())
            .expect("Missing %OkButton")
            .cast::<Button>();
        let mut reject_button = self
            .base()
            .get_node_or_null("%RejectButton".into())
            .expect("Missing %RejectButton")
            .cast::<Button>();

        ok_button.connect("pressed".into(), self.base().callable("_on_ok_pressed"));
        reject_button.connect("pressed".into(), self.base().callable("_on_reject_pressed"));

        self.title_label = Some(
            self.base()
                .get_node_or_null("%Title".into())
                .expect("Missing %Title")
                .cast::<Label>(),
        );
        self.description_label = Some(
            self.base()
                .get_node_or_null("%Description".into())
                .expect("Missing %Description")
                .cast::<Label>(),
        );
        self.ok_button = Some(ok_button);
        self.reject_button = Some(reject_button);
    }
}
