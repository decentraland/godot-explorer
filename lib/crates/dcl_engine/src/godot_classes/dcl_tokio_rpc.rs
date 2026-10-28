use dcl_core::OpenUrlRequest;
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=Node)]
pub struct DclTokioRpc {
    sender: tokio::sync::mpsc::Sender<OpenUrlRequest>,
    receiver: tokio::sync::mpsc::Receiver<OpenUrlRequest>,

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
        while let Ok(OpenUrlRequest {
            url,
            description,
            use_webview,
        }) = self.receiver.try_recv()
        {
            self.run_deferred(move |this| {
                this.signals()
                    .need_open_url()
                    .emit(&url, &description, use_webview)
            });
        }
    }
}

#[godot_api]
impl DclTokioRpc {
    #[signal]
    pub fn need_open_url(url: GString, description: GString, use_webview: bool);

    pub fn get_sender(&self) -> tokio::sync::mpsc::Sender<OpenUrlRequest> {
        self.sender.clone()
    }
}
