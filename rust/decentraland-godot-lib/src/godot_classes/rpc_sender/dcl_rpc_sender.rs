#[macro_export]
macro_rules! generate_dcl_rpc_sender {
    ($struct_name:ident, $response_type:ty) => {
        #[derive(godot::bind::GodotClass)]
        #[class(init, base=RefCounted)]
        pub struct $struct_name {
            sender: Option<crate::dcl::scene_apis::RpcResultSender<Result<$response_type, String>>>,

            #[base]
            _base: godot::obj::Base<godot::engine::RefCounted>,
        }

        impl $struct_name {
            pub fn set_sender(
                &mut self,
                sender: crate::dcl::scene_apis::RpcResultSender<Result<$response_type, String>>,
            ) {
                self.sender = Some(sender);
            }
        }

        #[godot::bind::godot_api]
        impl $struct_name {
            #[func]
            fn send(&mut self, response: godot::prelude::Variant) {
                if let Some(sender) = self.sender.as_ref() {
                    let response = response.try_to::<godot::prelude::Dictionary>().unwrap();
                    let response = <$response_type>::from_godot(response);
                    let sender: tokio::sync::oneshot::Sender<Result<$response_type, String>> =
                        sender.take();
                    let ret = sender.send(Ok(response));

                    match ret {
                        Ok(_) => {
                            tracing::info!("Response sent");
                        }
                        Err(e) => {
                            tracing::info!("Error sending response {:?}", e);
                        }
                    }
                }
            }
        }
    };
}
