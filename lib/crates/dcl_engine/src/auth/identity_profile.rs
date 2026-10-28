//! How the identity node follows the account: it fetches the profile of the connected wallet,
//! again on a realm change, and keeps the avatar editor's mutable copies in step.

use godot::prelude::*;

use super::dcl_player_identity::DclPlayerIdentity;
use crate::avatars::avatar_type::DclAvatarWireFormat;
use crate::avatars::dcl_user_profile::DclUserProfile;
use crate::engine::local_task::{await_promise, spawn_local};
use crate::fetcher::globals;
use crate::godot_classes::promise::PromiseError;
use crate::http_request::request_response::RequestResponse;
use crate::profile::profile_service::ProfileService;

impl DclPlayerIdentity {
    pub(super) fn connect_profile_flow(&mut self) {
        let me = self.to_gd();
        self.signals()
            .wallet_connected()
            .connect_self(Self::on_wallet_connected);
        self.signals()
            .profile_changed()
            .connect_self(Self::on_profile_changed);
        globals::realm()
            .signals()
            .realm_changed()
            .connect_other(&me, Self::on_realm_changed);
    }

    fn on_wallet_connected(&mut self, address: GString, _chain_id: i64, is_guest: bool) {
        if is_guest {
            self.set_default_profile();
            return;
        }
        self.lambda_url = globals::realm()
            .bind()
            .get_lambda_server_base_url()
            .to_string();
        spawn_local(fetch_profile(
            self.to_gd(),
            address.to_string(),
            self.lambda_url.clone(),
        ));
    }

    fn on_realm_changed(&mut self) {
        let url = globals::realm()
            .bind()
            .get_lambda_server_base_url()
            .to_string();
        if url == self.lambda_url {
            return;
        }
        self.lambda_url = url;
        let address = self.get_address_str().to_string();
        if !address.is_empty() && !self.is_guest {
            spawn_local(fetch_profile(
                self.to_gd(),
                address,
                self.lambda_url.clone(),
            ));
        }
    }

    fn on_profile_changed(&mut self, new_profile: Gd<DclUserProfile>) {
        self.mutable_profile = new_profile.bind().duplicated();
        self.mutable_avatar = self.mutable_profile.bind().get_avatar();
        // Blocked users come from the social service; only the muted ones live in the profile.
        let muted = new_profile.bind().get_muted();
        if !muted.is_empty() {
            globals::social_blacklist().bind_mut().append_muted(muted);
        }
    }

    /// Forgets the previous account's profile and avatar, so the next account's avatar editor
    /// does not inherit them (issue #1658).
    pub fn reset_identity(&mut self) {
        self.clear_identity();
        self.mutable_profile = DclUserProfile::new_gd();
        self.mutable_avatar = self.mutable_profile.bind().get_avatar();
    }

    pub fn get_mutable_avatar(&self) -> Gd<DclAvatarWireFormat> {
        self.mutable_avatar.clone()
    }

    pub fn get_mutable_profile(&self) -> Gd<DclUserProfile> {
        self.mutable_profile.clone()
    }
}

/// Fetches the profile of `address`; a missing or failed one falls back to the default profile.
async fn fetch_profile(mut me: Gd<DclPlayerIdentity>, address: String, lambda_url: String) {
    let promise =
        ProfileService::async_fetch_profile(address.as_str().into(), lambda_url.as_str().into());
    let response = await_promise(promise).await;
    // Dismissed when the account or the realm moved on while this was in flight.
    if me.bind().get_address_str().to_string() != address || me.bind().lambda_url != lambda_url {
        return;
    }
    if let Ok(error) = response.try_to::<Gd<PromiseError>>() {
        let reason = error.bind().get_error().to_string();
        // A profile deployed moments ago can 404 until the catalyst propagates it; keep it.
        let keeps_local = me.bind().get_profile_or_null().is_some_and(|current| {
            let current = current.bind();
            !current.get_name().is_empty()
                && current
                    .get_ethereum_address()
                    .to_string()
                    .eq_ignore_ascii_case(&address)
        });
        if keeps_local {
            godot_print!("Keeping local profile, fetch failed for {address}: {reason}");
            return;
        }
        me.bind_mut().set_default_profile();
        if reason.contains("404") {
            godot_print!("Profile not found for address {address} - starting with default profile");
        } else {
            godot_error!("Error while fetching profile for {address}, reason: {reason}");
        }
        return;
    }
    let Ok(response) = response.try_to::<Gd<RequestResponse>>() else {
        me.bind_mut().set_default_profile();
        return;
    };
    let updated = me.bind_mut()._update_profile_from_lambda(response);
    if !updated {
        me.bind_mut().set_default_profile();
    }
}
