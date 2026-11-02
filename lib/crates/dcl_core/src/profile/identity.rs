//! Who is signed in, as the profile domain needs it.

use crate::session::ephemeral_auth_chain::EphemeralAuthChain;

pub trait Identity: Send + Sync {
    /// Lowercase `0x` address of the signed-in account.
    fn address(&self) -> Option<String>;
    fn auth_chain(&self) -> Option<EphemeralAuthChain>;
    /// A disposable local wallet: its profile never leaves the device.
    fn is_local_guest(&self) -> bool;
}

/// The process's session in the installed core.
pub struct SessionIdentity;

impl Identity for SessionIdentity {
    fn address(&self) -> Option<String> {
        crate::Core::try_get()?.with_session(|s| s.try_address().map(|a| format!("{a:#x}")))
    }

    fn auth_chain(&self) -> Option<EphemeralAuthChain> {
        crate::Core::try_get()?.with_session(|s| s.ephemeral_auth_chain().cloned())
    }

    fn is_local_guest(&self) -> bool {
        crate::Core::try_get()
            .map(|core| {
                core.with_session(|s| {
                    s.recoverable_account()
                        .is_some_and(|account| !account.local_wallet.is_empty())
                })
            })
            .unwrap_or(false)
    }
}
