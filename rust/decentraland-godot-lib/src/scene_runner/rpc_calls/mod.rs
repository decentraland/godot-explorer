mod handle_restricted_actions;

use crate::common::rpc::{RpcCall, RpcCalls};

use self::handle_restricted_actions::change_realm;

use super::scene::Scene;

pub fn process_rpcs(scene: &Scene, rpc_calls: &RpcCalls) {
    for rpc_call in rpc_calls {
        match rpc_call {
            RpcCall::ChangeRealm { .. } => {
                change_realm(scene, rpc_call);
            }
        }
    }
}
