use godot::prelude::*;

pub enum GodotTokioCall {
    OpenUrl {
        url: String,
        description: String,
        use_webview: bool, // use webview
    },
}

#[derive(GodotClass)]
#[class(base=Node)]
pub struct DclTokioRpc {
    sender: tokio::sync::mpsc::Sender<GodotTokioCall>,
    receiver: tokio::sync::mpsc::Receiver<GodotTokioCall>,

    base: Base<Node>,
}

#[godot_api]
impl INode for DclTokioRpc {
    fn init(base: Base<Node>) -> Self {
        let (sender, receiver) = tokio::sync::mpsc::channel(100);

        Self {
            sender,
            receiver,
            base,
        }
    }

    fn process(&mut self, _dt: f64) {
        while let Ok(state) = self.receiver.try_recv() {
            match state {
                GodotTokioCall::OpenUrl {
                    url,
                    description,
                    use_webview,
                } => {
                    self.base_mut().call_deferred(
                        "emit_signal",
                        &[
                            "need_open_url".to_variant(),
                            url.to_variant(),
                            description.to_variant(),
                            use_webview.to_variant(),
                        ],
                    );
                }
            }
        }
    }
}

#[godot_api]
impl DclTokioRpc {
    #[signal]
    fn need_open_url(url: GString, description: GString, use_webview: bool);

    pub fn get_sender(&self) -> tokio::sync::mpsc::Sender<GodotTokioCall> {
        self.sender.clone()
    }
}
