//! The content id of a Decentraland entity: CIDv1, raw codec, sha2-256, base32 lowercase.

use sha2::{Digest, Sha256};

const ALPHABET: &[u8; 32] = b"abcdefghijklmnopqrstuvwxyz234567";
/// CID version 1, raw codec (0x55), sha2-256 (0x12) with a 32 byte digest (0x20).
const HEADER: [u8; 4] = [0x01, 0x55, 0x12, 0x20];

pub fn entity_id(bytes: &[u8]) -> String {
    let mut raw = HEADER.to_vec();
    raw.extend_from_slice(&Sha256::digest(bytes));
    let mut out = String::from("b");
    let (mut buffer, mut bits) = (0u32, 0u32);
    for byte in raw {
        buffer = (buffer << 8) | u32::from(byte);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(ALPHABET[((buffer >> bits) & 31) as usize] as char);
        }
    }
    if bits > 0 {
        out.push(ALPHABET[((buffer << (5 - bits)) & 31) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_the_known_cid_of_empty_input() {
        assert_eq!(
            entity_id(b""),
            "bafkreihdwdcefgh4dqkjv67uzcmw7ojee6xedzdetojuzjevtenxquvyku"
        );
    }

    #[test]
    fn matches_the_known_cid_of_hello_world() {
        assert_eq!(
            entity_id(b"hello world"),
            "bafkreifzjut3te2nhyekklss27nh3k72ysco7y32koao5eei66wof36n5e"
        );
    }
}
