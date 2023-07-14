use std::{sync::Arc, time::Instant};

pub mod wallet;

use crate::dcl::components::proto_components::kernel::comms::rfc5::{ws_packet, WsIdentification};
use godot::{
    engine::{TlsOptions, WebSocketPeer},
    prelude::*,
};

use self::wallet::Wallet;

struct WebSocketRoom {
    url: GodotString,
    ws_peer: Gd<WebSocketPeer>,
    last_state: godot::engine::web_socket_peer::State,
    last_try_time: Instant,
    tls_client: Gd<TlsOptions>,
    wallet: Arc<Wallet>,
}

impl WebSocketRoom {
    fn new(ws_url: &str, tls_client: Gd<TlsOptions>, wallet: Arc<Wallet>) -> Self {
        Self {
            ws_peer: WebSocketPeer::new(),
            url: GodotString::from(ws_url),
            last_state: godot::engine::web_socket_peer::State::STATE_CLOSED,
            last_try_time: Instant::now(),
            tls_client,
            wallet,
        }
    }

    fn poll(&mut self) {
        let mut peer = self.ws_peer.share();
        peer.poll();

        let current_state = peer.get_ready_state();
        if current_state != self.last_state {
            match peer.get_ready_state() {
                godot::engine::web_socket_peer::State::STATE_CONNECTING => {
                    godot_print!("comms > connecting to {}", self.url);
                    self.last_try_time = Instant::now();
                }
                godot::engine::web_socket_peer::State::STATE_CLOSING => {
                    godot_print!("comms > closing to {}", self.url);
                }
                godot::engine::web_socket_peer::State::STATE_CLOSED => {
                    godot_print!("comms > closed to {}", self.url);
                }
                godot::engine::web_socket_peer::State::STATE_OPEN => {
                    godot_print!("comms > connected to {}", self.url);

                    let ident = ws_packet::Message::PeerIdentification(WsIdentification {
                        address: format!("{:#x}", self.wallet.address()),
                    });

                    let mut buf = Vec::new();
                    ident.encode(&mut buf);

                    let buf = PackedByteArray::from_iter(buf.into_iter());
                    peer.send(buf);
                }
                _ => {}
            }
            self.last_state = current_state;
        }

        match peer.get_ready_state() {
            godot::engine::web_socket_peer::State::STATE_CLOSED => {
                if (Instant::now() - self.last_try_time).as_secs() > 1 {
                    // TODO: see if the tls client is really required for now
                    let _tls_client = self.tls_client.share();

                    peer.call("connect_to_url".into(), &[self.url.clone().to_variant()]);
                }
            }
            godot::engine::web_socket_peer::State::STATE_OPEN => {
                while peer.get_available_packet_count() > 0 {
                    let packet = peer.get_packet();
                    godot_print!("comms > packet {:?}", packet);
                }
            }
            _ => {}
        }
    }
}

enum Adapter {
    None,
    WsRoom(WebSocketRoom),
}

#[derive(GodotClass)]
#[class(base=Node)]
pub struct Comms {
    #[base]
    base: Base<Node>,
    current_adapter: Adapter,
    tls_client: Option<Gd<TlsOptions>>,
    wallet: Arc<Wallet>,
}

#[godot_api]
impl NodeVirtual for Comms {
    fn init(base: Base<Node>) -> Self {
        Comms {
            base,
            current_adapter: Adapter::None,
            tls_client: None,
            wallet: Arc::new(Wallet::new_local_wallet()),
        }
    }

    fn ready(&mut self) {
        self.call_deferred("init_rs".into(), &[]);
    }

    fn process(&mut self, _dt: f64) {
        match &mut self.current_adapter {
            Adapter::None => {}
            Adapter::WsRoom(ws_room) => {
                ws_room.poll();
            }
        }
    }
}

impl Comms {}

#[godot_api]
impl Comms {
    fn realm(&self) -> Gd<Node> {
        self.base.get_node("/root/realm".into()).unwrap()
    }

    #[func]
    fn init_rs(&mut self) {
        let mut realm = self.realm();
        let on_realm_changed =
            Callable::from_object_method(self.base.share(), StringName::from("_on_realm_changed"));

        realm.connect("realm_changed".into(), on_realm_changed);

        if self.tls_client.is_none() {
            let tls_client = self
                .get_node("/root/Global".into())
                .unwrap()
                .call("get_tls_client".into(), &[]);
            let tls_client: Gd<TlsOptions> = Gd::from_variant(&tls_client);
            self.tls_client = Some(tls_client);
        }
    }

    #[func]
    fn _on_realm_changed(&mut self) {
        self.call_deferred("_on_realm_changed_deferred".into(), &[]);
    }

    fn _internal_get_comms_from_real(&self) -> Option<(String, Option<GodotString>)> {
        let realm = self.realm();
        let realm_about = Dictionary::from_variant(&realm.get("realm_about".into()));
        let comms = Dictionary::from_variant(&realm_about.get(StringName::from("comms"))?);
        let comms_protocol = String::from_variant(&comms.get(StringName::from("protocol"))?);
        let comms_fixed_adapter = comms
            .get(StringName::from("fixedAdapter"))
            .map(|v| GodotString::from_variant(&v));

        Some((comms_protocol, comms_fixed_adapter))
    }

    #[func]
    fn _on_realm_changed_deferred(&mut self) {
        self.clean();

        let comms = self._internal_get_comms_from_real();
        if comms.is_none() {
            godot_print!("comms > invalid comms from realm.");
            return;
        }

        let (comms_protocol, comms_fixed_adapter) = comms.unwrap();
        if comms_protocol != "v3" {
            godot_print!("comms > Only protocol 'v3' is supported.");
            return;
        }

        if comms_fixed_adapter.is_none() {
            godot_print!("comms > As far, only fixedAdapter is supported.");
            return;
        }

        let comms_fixed_adapter_str = comms_fixed_adapter.unwrap().to_string();
        let fixed_adapter: Vec<&str> = comms_fixed_adapter_str.splitn(2, ':').collect();
        let adapter_protocol = *fixed_adapter.first().unwrap();

        match adapter_protocol {
            "ws-room" => {
                if let Some(ws_url) = fixed_adapter.get(1) {
                    godot_print!("comms > websocket to {}", ws_url);
                    self.current_adapter = Adapter::WsRoom(WebSocketRoom::new(
                        ws_url,
                        self.tls_client.as_ref().unwrap().share(),
                        self.wallet.clone(),
                    ));
                }
            }
            "offline" => {
                godot_print!("comms > set offline");
            }
            _ => {
                godot_print!("comms > unknown adapter {:?}", adapter_protocol);
            }
        }
    }

    fn clean(&mut self) {
        match &self.current_adapter {
            Adapter::None => {}
            Adapter::WsRoom(ws_room) => {
                let mut peer = ws_room.ws_peer.share();
                peer.close();
                match peer.get_ready_state() {
                    godot::engine::web_socket_peer::State::STATE_OPEN
                    | godot::engine::web_socket_peer::State::STATE_CONNECTING => {
                        peer.close();
                    }
                    _ => {}
                }
            }
        }

        self.current_adapter = Adapter::None;
    }
}
