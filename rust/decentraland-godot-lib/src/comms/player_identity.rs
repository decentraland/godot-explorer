use godot::prelude::Dictionary;

use crate::wallet::Wallet;

use super::profile::UserProfile;

#[derive(Clone)]
pub struct PlayerIdentity {
    wallet: Wallet,
    profile: UserProfile,
}

impl Default for PlayerIdentity {
    fn default() -> Self {
        let wallet = Wallet::new_local_wallet();
        let mut profile = UserProfile::default();
        profile.content.user_id = Some(format!("{:#x}", wallet.address()));

        Self { wallet, profile }
    }
}

impl PlayerIdentity {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn wallet(&self) -> &Wallet {
        &self.wallet
    }

    pub fn profile(&self) -> &UserProfile {
        &self.profile
    }

    pub fn update_profile_from_dictionary(&mut self, dict: &Dictionary) {
        self.profile.content.copy_from_godot_dictionary(dict);
        self.profile.version += 1;
    }
}
