use crate::dcl::scene_apis::RpcResultSender;
use godot::prelude::*;

pub enum GodotTokioCall {
    MagicSignMessage {
        message: String,
        response: RpcResultSender<(String, String)>,
    },
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

    waiting_signature_response: Option<RpcResultSender<(String, String)>>,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for DclTokioRpc {
    fn init(base: Base<Node>) -> Self {
        let (sender, receiver) = tokio::sync::mpsc::channel(100);

        Self {
            sender,
            receiver,
            waiting_signature_response: None,
            base,
        }
    }

    fn process(&mut self, _dt: f64) {
        while let Ok(state) = self.receiver.try_recv() {
            match state {
                GodotTokioCall::OpenUrl { url, description, use_webview } => {
                    self.base.call_deferred(
                        "emit_signal".into(),
                        &[
                            "need_open_url".to_variant(),
                            url.to_variant(),
                            description.to_variant(),
                            use_webview.to_variant(),
                        ],
                    );
                }
                GodotTokioCall::MagicSignMessage { message, response } => {
                    self.waiting_signature_response = Some(response);

                    self.base.call_deferred(
                        "emit_signal".into(),
                        &["magic_sign".to_variant(), message.to_variant()],
                    );
                }
            }
        }
    }
}

#[godot_api]
impl DclTokioRpc {
    #[signal]
    fn need_open_url(&self, url: GString, description: GString, use_webview: bool);

    #[signal]
    fn magic_sign(&self, message: GString);

    #[func]
    fn magic_signed_message(&mut self, signer: GString, signature: GString) {
        if let Some(response) = &self.waiting_signature_response {
            response.send((signer.into(), signature.into()));
            self.waiting_signature_response = None;
        }
    }

    pub fn get_sender(&self) -> tokio::sync::mpsc::Sender<GodotTokioCall> {
        self.sender.clone()
    }
}
