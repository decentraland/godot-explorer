//! The account state of the running app: which wallet is active, its ephemeral auth chain and the
//! sign-in bookkeeping around it. Pure state; the host turns the returned events into signals.

use ethers_core::types::H160;
use ethers_signers::LocalWallet;

use super::{
    ephemeral_auth_chain::EphemeralAuthChain, remote_wallet::RemoteWallet, thirdweb_guest,
    wallet::Wallet,
};

enum CurrentWallet {
    Remote(RemoteWallet),
    Local { wallet: Wallet, keys: Vec<u8> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionEvent {
    WalletConnected {
        address: String,
        chain_id: u64,
        is_guest: bool,
    },
    Logout,
}

/// Everything needed to restore an account on the next launch.
#[derive(Clone)]
pub struct RecoverableAccount {
    pub account_address: H160,
    pub chain_id: u64,
    pub ephemeral_auth_chain: EphemeralAuthChain,
    /// Private key bytes of a disposable local wallet; empty for remote wallets.
    pub local_wallet: Vec<u8>,
}

#[derive(Default)]
pub struct Session {
    wallet: Option<CurrentWallet>,
    ephemeral_auth_chain: Option<EphemeralAuthChain>,
    /// `true` only when the active wallet is a silent persistent thirdweb guest.
    pub is_thirdweb_guest: bool,
    /// The thirdweb guest has a linked auth method beyond the silent login. Only meaningful while
    /// `is_thirdweb_guest` is set.
    pub is_thirdweb_guest_upgraded: bool,
    /// Set between `start_mobile_connect_account` and the returning deep link. Process-local: a
    /// cold start during the browser hop legitimately loses it.
    pending_mobile_auth: Option<()>,
    mobile_auth_cancelled: bool,
    mobile_auth_started: bool,
    mobile_auth_attempt: u32,
}

impl Session {
    pub fn install_remote(
        &mut self,
        address: H160,
        chain_id: u64,
        ephemeral_auth_chain: EphemeralAuthChain,
    ) -> SessionEvent {
        self.wallet = Some(CurrentWallet::Remote(RemoteWallet::new(address, chain_id)));
        self.ephemeral_auth_chain = Some(ephemeral_auth_chain);
        self.is_thirdweb_guest = false;
        self.is_thirdweb_guest_upgraded = false;
        SessionEvent::WalletConnected {
            address: format!("{address:#x}"),
            chain_id,
            is_guest: false,
        }
    }

    /// WalletConnect keeps the thirdweb markers as they were.
    pub fn install_walletconnect(
        &mut self,
        address: H160,
        ephemeral_auth_chain: EphemeralAuthChain,
    ) -> SessionEvent {
        self.wallet = Some(CurrentWallet::Remote(RemoteWallet::new(address, 1)));
        self.ephemeral_auth_chain = Some(ephemeral_auth_chain);
        SessionEvent::WalletConnected {
            address: format!("{address:#x}"),
            chain_id: 1,
            is_guest: false,
        }
    }

    pub fn install_local(
        &mut self,
        local_wallet_bytes: &[u8],
        ephemeral_auth_chain: EphemeralAuthChain,
    ) -> Result<SessionEvent, anyhow::Error> {
        let local_wallet =
            Wallet::new_from_inner(Box::new(LocalWallet::from_bytes(local_wallet_bytes)?));
        let address = local_wallet.address();
        self.wallet = Some(CurrentWallet::Local {
            wallet: local_wallet,
            keys: local_wallet_bytes.to_vec(),
        });
        self.ephemeral_auth_chain = Some(ephemeral_auth_chain);
        self.is_thirdweb_guest = false;
        self.is_thirdweb_guest_upgraded = false;
        Ok(SessionEvent::WalletConnected {
            address: format!("{address:#x}"),
            chain_id: 1,
            is_guest: true,
        })
    }

    /// A throwaway local wallet (dev builds): it has no server account to delete.
    pub fn is_disposable_guest(&self) -> bool {
        matches!(self.wallet, Some(CurrentWallet::Local { .. }))
    }

    /// Whether analytics counts the player as a guest: a disposable wallet, or a thirdweb guest
    /// that has not linked an email.
    pub fn counts_as_guest(&self) -> bool {
        self.is_disposable_guest() || (self.is_thirdweb_guest && !self.is_thirdweb_guest_upgraded)
    }

    /// Restores a saved account. `None` when the saved chain has expired or the key is unusable.
    pub fn recover(&mut self, account: RecoverableAccount) -> Option<SessionEvent> {
        if account.ephemeral_auth_chain.expired() {
            return None;
        }
        if !account.local_wallet.is_empty() {
            return self
                .install_local(&account.local_wallet, account.ephemeral_auth_chain)
                .ok();
        }
        let event = self.install_remote(
            account.account_address,
            account.chain_id,
            account.ephemeral_auth_chain,
        );
        // A recovered remote wallet does not record how it was minted: it is the thirdweb guest
        // only if its address matches the persisted guest session.
        if let Some(session) = thirdweb_guest::load_session_from_disk() {
            if session.wallet_address == account.account_address {
                self.is_thirdweb_guest = true;
            }
        }
        Some(event)
    }

    pub fn recoverable_account(&self) -> Option<RecoverableAccount> {
        let chain = self.ephemeral_auth_chain.clone()?;
        let (chain_id, local_wallet) = match self.wallet.as_ref()? {
            CurrentWallet::Remote(wallet) => (wallet.chain_id(), Vec::new()),
            CurrentWallet::Local { keys, .. } => (1, keys.clone()),
        };
        Some(RecoverableAccount {
            account_address: self.try_address()?,
            chain_id,
            ephemeral_auth_chain: chain,
            local_wallet,
        })
    }

    pub fn try_address(&self) -> Option<H160> {
        match self.wallet.as_ref()? {
            CurrentWallet::Remote(wallet) => Some(wallet.address()),
            CurrentWallet::Local { wallet, .. } => Some(wallet.address()),
        }
    }

    pub fn address_string(&self) -> String {
        self.try_address()
            .map(|address| format!("{address:#x}"))
            .unwrap_or_default()
    }

    pub fn ephemeral_auth_chain(&self) -> Option<&EphemeralAuthChain> {
        self.ephemeral_auth_chain.as_ref()
    }

    /// Forgets the account. `None` when nobody was signed in.
    pub fn logout(&mut self) -> Option<SessionEvent> {
        self.try_address()?;
        self.wallet = None;
        self.ephemeral_auth_chain = None;
        self.is_thirdweb_guest = false;
        self.is_thirdweb_guest_upgraded = false;
        self.pending_mobile_auth = None;
        self.mobile_auth_cancelled = false;
        self.mobile_auth_started = false;
        Some(SessionEvent::Logout)
    }

    pub fn begin_mobile_auth(&mut self) -> u32 {
        self.mobile_auth_started = true;
        self.mobile_auth_cancelled = false;
        self.mobile_auth_attempt = self.mobile_auth_attempt.wrapping_add(1);
        self.mobile_auth_attempt
    }

    pub fn set_pending_mobile_auth(&mut self) {
        self.pending_mobile_auth = Some(());
    }

    /// Consumes the pending flag; its absence is not an error (the app may have been killed).
    pub fn take_pending_mobile_auth(&mut self) {
        self.pending_mobile_auth.take();
    }

    pub fn has_pending_mobile_auth(&self) -> bool {
        self.pending_mobile_auth.is_some()
    }

    pub fn was_mobile_auth_cancelled(&self) -> bool {
        self.mobile_auth_cancelled
    }

    pub fn mobile_auth_attempt(&self) -> u32 {
        self.mobile_auth_attempt
    }

    pub fn is_current_mobile_attempt(&self, attempt: u32) -> bool {
        attempt == self.mobile_auth_attempt
    }

    /// Cancels the sign-in in flight. The cancel only sticks if this process started the flow,
    /// otherwise the returning deep link would look like a cold start and complete anyway.
    pub fn abort_connect(&mut self) {
        self.pending_mobile_auth = None;
        self.mobile_auth_cancelled = self.mobile_auth_started;
        self.mobile_auth_attempt = self.mobile_auth_attempt.wrapping_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::auth_identity::create_local_ephemeral;
    use rand::thread_rng;

    fn local() -> (LocalWallet, EphemeralAuthChain) {
        let wallet = LocalWallet::new(&mut thread_rng());
        let chain = create_local_ephemeral(&wallet);
        (wallet, chain)
    }

    #[test]
    fn guests_count_until_they_link_an_email() {
        let mut session = Session::default();
        assert!(!session.counts_as_guest());
        session.is_thirdweb_guest = true;
        assert!(session.counts_as_guest());
        session.is_thirdweb_guest_upgraded = true;
        assert!(!session.counts_as_guest());

        let (wallet, chain) = local();
        let mut local_session = Session::default();
        local_session
            .install_local(&wallet.signer().to_bytes(), chain)
            .unwrap();
        assert!(local_session.counts_as_guest());
    }

    #[test]
    fn local_account_round_trips_through_recovery() {
        let (wallet, chain) = local();
        let mut session = Session::default();
        let event = session
            .install_local(&wallet.signer().to_bytes(), chain)
            .unwrap();
        assert!(matches!(
            event,
            SessionEvent::WalletConnected {
                is_guest: true,
                chain_id: 1,
                ..
            }
        ));

        let saved = session.recoverable_account().unwrap();
        assert!(!saved.local_wallet.is_empty());

        let mut restored = Session::default();
        restored.recover(saved).unwrap();
        assert_eq!(restored.address_string(), session.address_string());
    }

    #[test]
    fn logout_clears_and_reports_once() {
        let (wallet, chain) = local();
        let mut session = Session::default();
        assert_eq!(session.logout(), None);
        session
            .install_local(&wallet.signer().to_bytes(), chain)
            .unwrap();
        assert_eq!(session.logout(), Some(SessionEvent::Logout));
        assert!(session.try_address().is_none());
        assert_eq!(session.logout(), None);
    }

    #[test]
    fn remote_install_resets_thirdweb_markers() {
        let (wallet, chain) = local();
        let mut session = Session {
            is_thirdweb_guest: true,
            is_thirdweb_guest_upgraded: true,
            ..Default::default()
        };
        use ethers_signers::Signer;
        session.install_remote(wallet.address(), 1, chain);
        assert!(!session.is_thirdweb_guest && !session.is_thirdweb_guest_upgraded);
    }

    #[test]
    fn abort_only_marks_cancelled_when_this_process_started_the_flow() {
        let mut session = Session::default();
        session.abort_connect();
        assert!(!session.was_mobile_auth_cancelled());

        let attempt = session.begin_mobile_auth();
        session.set_pending_mobile_auth();
        session.abort_connect();
        assert!(session.was_mobile_auth_cancelled());
        assert!(!session.has_pending_mobile_auth());
        assert!(!session.is_current_mobile_attempt(attempt));
    }
}
