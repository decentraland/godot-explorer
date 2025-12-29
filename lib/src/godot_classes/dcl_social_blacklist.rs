use std::collections::HashSet;

use ethers_core::types::H160;
use godot::{
    builtin::Array,
    obj::{Base, Gd},
    prelude::*,
};

use crate::avatars::dcl_user_profile::DclUserProfile;

/// Manages blocked and muted user lists locally
/// This is used for efficient filtering of incoming messages without
/// requiring heavy profile updates for each block/mute operation
#[derive(GodotClass)]
#[class(base=Node)]
pub struct DclSocialBlacklist {
    base: Base<Node>,
    blocked_addresses: HashSet<H160>,
    muted_addresses: HashSet<H160>,
}

#[godot_api]
impl INode for DclSocialBlacklist {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            blocked_addresses: HashSet::new(),
            muted_addresses: HashSet::new(),
        }
    }
}

#[godot_api]
impl DclSocialBlacklist {
    /// Signal emitted when the blocked or muted lists change
    #[signal]
    fn blacklist_changed();

    /// Add a single address to the blocked list
    #[func]
    pub fn add_blocked(&mut self, address: GString) -> bool {
        if let Ok(addr) = address.to_string().parse::<H160>() {
            let changed = self.blocked_addresses.insert(addr);
            if changed {
                self.base_mut().emit_signal("blacklist_changed", &[]);
            }
            changed
        } else {
            godot_error!("Invalid address format: {}", address);
            false
        }
    }

    /// Remove a single address from the blocked list
    #[func]
    pub fn remove_blocked(&mut self, address: GString) -> bool {
        if let Ok(addr) = address.to_string().parse::<H160>() {
            let changed = self.blocked_addresses.remove(&addr);
            if changed {
                self.base_mut().emit_signal("blacklist_changed", &[]);
            }
            changed
        } else {
            godot_error!("Invalid address format: {}", address);
            false
        }
    }

    /// Check if an address is blocked
    #[func]
    pub fn is_blocked(&self, address: GString) -> bool {
        if let Ok(addr) = address.to_string().parse::<H160>() {
            self.blocked_addresses.contains(&addr)
        } else {
            false
        }
    }

    /// Add a single address to the muted list
    #[func]
    pub fn add_muted(&mut self, address: GString) -> bool {
        if let Ok(addr) = address.to_string().parse::<H160>() {
            let changed = self.muted_addresses.insert(addr);
            if changed {
                self.base_mut().emit_signal("blacklist_changed", &[]);
            }
            changed
        } else {
            godot_error!("Invalid address format: {}", address);
            false
        }
    }

    /// Remove a single address from the muted list
    #[func]
    pub fn remove_muted(&mut self, address: GString) -> bool {
        if let Ok(addr) = address.to_string().parse::<H160>() {
            let changed = self.muted_addresses.remove(&addr);
            if changed {
                self.base_mut().emit_signal("blacklist_changed", &[]);
            }
            changed
        } else {
            godot_error!("Invalid address format: {}", address);
            false
        }
    }

    /// Check if an address is muted
    #[func]
    pub fn is_muted(&self, address: GString) -> bool {
        if let Ok(addr) = address.to_string().parse::<H160>() {
            self.muted_addresses.contains(&addr)
        } else {
            false
        }
    }

    /// Add multiple addresses to the blocked list
    #[func]
    pub fn append_blocked(&mut self, addresses: Array<GString>) {
        let mut changed = false;
        for address in addresses.iter_shared() {
            if let Ok(addr) = address.to_string().parse::<H160>() {
                if self.blocked_addresses.insert(addr) {
                    changed = true;
                }
            } else {
                godot_error!("Invalid address format: {}", address);
            }
        }
        if changed {
            self.base_mut().emit_signal("blacklist_changed", &[]);
        }
    }

    /// Add multiple addresses to the muted list
    #[func]
    pub fn append_muted(&mut self, addresses: Array<GString>) {
        let mut changed = false;
        for address in addresses.iter_shared() {
            if let Ok(addr) = address.to_string().parse::<H160>() {
                if self.muted_addresses.insert(addr) {
                    changed = true;
                }
            } else {
                godot_error!("Invalid address format: {}", address);
            }
        }
        if changed {
            self.base_mut().emit_signal("blacklist_changed", &[]);
        }
    }

    /// Clear all blocked addresses
    #[func]
    pub fn clear_blocked(&mut self) {
        if !self.blocked_addresses.is_empty() {
            self.blocked_addresses.clear();
            self.base_mut().emit_signal("blacklist_changed", &[]);
        }
    }

    /// Clear all muted addresses
    #[func]
    pub fn clear_muted(&mut self) {
        if !self.muted_addresses.is_empty() {
            self.muted_addresses.clear();
            self.base_mut().emit_signal("blacklist_changed", &[]);
        }
    }

    /// Get all blocked addresses as an array
    #[func]
    pub fn get_blocked_list(&self) -> Array<GString> {
        let mut arr = Array::new();
        for addr in &self.blocked_addresses {
            arr.push(&GString::from(format!("{:#x}", addr).as_str()));
        }
        arr
    }

    /// Get all muted addresses as an array
    #[func]
    pub fn get_muted_list(&self) -> Array<GString> {
        let mut arr = Array::new();
        for addr in &self.muted_addresses {
            arr.push(&GString::from(format!("{:#x}", addr).as_str()));
        }
        arr
    }

    /// Initialize from a user profile (to load existing blocked/muted lists)
    #[func]
    pub fn init_from_profile(&mut self, profile: Gd<DclUserProfile>) {
        let profile_bind = profile.bind();
        let inner = &profile_bind.inner;

        // Store current state to check if anything changed
        let old_blocked = self.blocked_addresses.clone();
        let old_muted = self.muted_addresses.clone();

        // Clear existing lists
        self.blocked_addresses.clear();
        self.muted_addresses.clear();

        // Load blocked addresses
        if let Some(blocked_list) = &inner.content.blocked {
            for addr_str in blocked_list {
                if let Ok(addr) = addr_str.parse::<H160>() {
                    self.blocked_addresses.insert(addr);
                }
            }
        }

        // Load muted addresses
        if let Some(muted_list) = &inner.content.muted {
            for addr_str in muted_list {
                if let Ok(addr) = addr_str.parse::<H160>() {
                    self.muted_addresses.insert(addr);
                }
            }
        }

        // Only emit signal if the lists actually changed
        if old_blocked != self.blocked_addresses || old_muted != self.muted_addresses {
            self.base_mut().emit_signal("blacklist_changed", &[]);
        }
    }

    /// Internal method to check if an address is blocked (using H160 directly)
    pub fn is_blocked_h160(&self, address: &H160) -> bool {
        self.blocked_addresses.contains(address)
    }

    /// Internal method to check if an address is muted (using H160 directly)
    pub fn is_muted_h160(&self, address: &H160) -> bool {
        self.muted_addresses.contains(address)
    }

    /// Get the blocked addresses as a HashSet<String> for profile serialization
    pub fn get_blocked_as_strings(&self) -> HashSet<String> {
        self.blocked_addresses
            .iter()
            .map(|addr| format!("{:#x}", addr))
            .collect()
    }

    /// Get the muted addresses as a HashSet<String> for profile serialization
    pub fn get_muted_as_strings(&self) -> HashSet<String> {
        self.muted_addresses
            .iter()
            .map(|addr| format!("{:#x}", addr))
            .collect()
    }

    /// Get a reference to the blocked addresses HashSet (for performance)
    pub fn get_blocked_set(&self) -> &HashSet<H160> {
        &self.blocked_addresses
    }

    /// Get a reference to the muted addresses HashSet (for performance)
    pub fn get_muted_set(&self) -> &HashSet<H160> {
        &self.muted_addresses
    }
}
