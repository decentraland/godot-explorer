use super::{profile::UserProfile, wallet::Wallet};

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
}
