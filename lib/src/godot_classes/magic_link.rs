use godot::{engine::Engine, prelude::*};

use crate::dcl::scene_apis::RpcResultSender;

pub enum MagicLinkRequest {
    SignMessage {
        message: String,
        response: RpcResultSender<String>,
    },
}

#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct MagicLink {
    #[var]
    wallet_connected: bool,

    #[var]
    public_address: GString,

    base: Base<Node>,
}

#[godot_api]
impl MagicLink {
    const PLUGIN_NAME: &'static str = "GodotAndroidPluginMagicLink";

    #[signal]
    fn connection_state(&self, success: bool) {}

    #[signal]
    fn connected(&self, address: GString) {}

    #[signal]
    fn logout(&self) {}

    #[signal]
    fn message_signed(&self, signed_message: GString) {}

    #[func]
    fn get_singleton() -> Option<Gd<Object>> {
        let engine = Engine::singleton();
        if engine.has_singleton(StringName::from(MagicLink::PLUGIN_NAME)) {
            Some(
                engine
                    .get_singleton(StringName::from(MagicLink::PLUGIN_NAME))
                    .unwrap(),
            )
        } else {
            godot_print!("Initialization error: unable to access the java logic");
            None
        }
    }

    #[func]
    fn is_using_magic(&self) -> bool {
        self.get_wallet_connected()
    }

    #[func]
    fn _on_connected(&mut self, address: GString) {
        self.set_public_address(address.clone());

        self.set_wallet_connected(true);

        self.base_mut().call_deferred(
            "emit_signal".into(),
            &["connected".to_variant(), address.clone().to_variant()],
        );
    }

    #[func]
    fn _on_logout(&mut self) {
        self.set_public_address("".into());
        self.set_wallet_connected(false);

        self.base_mut()
            .call_deferred("emit_signal".into(), &["logout".to_variant()]);
    }

    #[func]
    fn _on_connection_state(&mut self, state: GString) {
        let success = state == "true".into();

        self.base_mut().call_deferred(
            "emit_signal".into(),
            &["connection_state".to_variant(), success.to_variant()],
        );
    }

    #[func]
    fn _on_signed_message(&mut self, signature: GString) {
        self.base_mut().call_deferred(
            "emit_signal".into(),
            &["message_signed".to_variant(), signature.to_variant()],
        );
    }

    #[func]
    pub fn setup(&self, magic_key: GString, callback_url: GString, network: GString) {
        if let Some(mut singleton) = MagicLink::get_singleton() {
            singleton.call(
                "setup".into(),
                &[
                    magic_key.to_variant(),
                    callback_url.to_variant(),
                    network.to_variant(),
                ],
            );
            singleton.connect("connected".into(), self.base().callable("_on_connected"));
            singleton.connect(
                "connection_state".into(),
                self.base().callable("_on_connection_state"),
            );
            singleton.connect(
                "signed_message".into(),
                self.base().callable("_on_signed_message"),
            );

            singleton.connect("on_logout".into(), self.base().callable("_on_logout"));
        } else {
            godot_print!("Initialization error");
        }
    }

    #[func]
    pub fn check_connection(&self) {
        if let Some(mut singleton) = MagicLink::get_singleton() {
            singleton.call("checkConnection".into(), &[]);
        } else {
            godot_print!("Initialization error");
        }
    }

    #[func]
    pub fn login_email(&self, email: GString) {
        if let Some(mut singleton) = MagicLink::get_singleton() {
            singleton.call("loginEmailOTP".into(), &[email.to_variant()]);
        } else {
            godot_print!("Initialization error");
        }
    }

    #[func]
    pub fn login_social(&self, oauth_provider: GString) {
        if let Some(mut singleton) = MagicLink::get_singleton() {
            singleton.call("loginSocial".into(), &[oauth_provider.to_variant()]);
        } else {
            godot_print!("Initialization error");
        }
    }

    #[func]
    pub fn open_wallet(&self) {
        if !self.wallet_connected {
            godot_print!("Please, check if you're connected first...");
            return;
        }
        if let Some(mut singleton) = MagicLink::get_singleton() {
            singleton.call("openWallet".into(), &[]);
        } else {
            godot_print!("Initialization error");
        }
    }

    #[func]
    pub fn sign(&self, message: GString) {
        if !self.wallet_connected {
            godot_print!("Please, check if you're connected first...");
            return;
        }
        if let Some(mut singleton) = MagicLink::get_singleton() {
            singleton.call("sign".into(), &[message.to_variant()]);
        } else {
            godot_print!("Initialization error");
        }
    }
}
