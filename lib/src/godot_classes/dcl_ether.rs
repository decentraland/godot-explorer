use ethers_core::{types::U256, utils::format_units};
use godot::prelude::*;

#[derive(GodotClass)]
#[class(init, base=RefCounted)]
pub struct DclEther {
    _base: Base<RefCounted>,
}

#[godot_api]
impl DclEther {
    #[func]
    pub fn shorten_eth_address(address: GString) -> GString {
        let address = address.to_string();
        if address.len() == 42 && address.starts_with("0x") {
            let start = &address[0..6]; // First four characters after "0x"
            let end = &address[address.len() - 4..]; // Last four characters
            GString::from(format!("{}...{}", start, end).as_str())
        } else {
            GString::from("Invalid address")
        }
    }

    #[func]
    fn is_valid_ethereum_address(address: GString) -> bool {
        let address = address.to_string();
        // Correct length
        if address.len() != 42 {
            return false;
        }

        // Correct prefix
        if !address.starts_with("0x") {
            return false;
        }

        // Valid hexadecimal characters
        address[2..].chars().all(|c| c.is_ascii_hexdigit())
    }

    #[func]
    fn format_units(amount: GString, units: u32) -> f32 {
        if let Ok(amount) = U256::from_dec_str(amount.to_string().as_str()) {
            let ethers = format_units(amount, units).unwrap_or("0.0".to_string());
            ethers.parse::<f32>().unwrap_or_default()
        } else {
            0.0
        }
    }
}
