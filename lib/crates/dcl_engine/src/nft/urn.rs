//! `urn:decentraland:<chain>:<standard>:<contract>:<token>`.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NftUrn {
    pub urn: String,
    pub chain: String,
    pub contract_address: String,
    pub token_id: String,
}

impl NftUrn {
    /// None when the text is not a decentraland NFT urn. The contract standard (part 3) is not
    /// used.
    pub fn parse(urn: &str) -> Option<NftUrn> {
        let parts: Vec<&str> = urn.split(':').collect();
        if parts.len() < 6 || parts[0] != "urn" || parts[1] != "decentraland" {
            return None;
        }
        Some(NftUrn {
            urn: urn.to_string(),
            chain: parts[2].to_string(),
            contract_address: parts[4].to_string(),
            token_id: parts[5].to_string(),
        })
    }

    /// The key requests are cached by.
    pub fn hash(&self) -> String {
        format!("{}:{}", self.contract_address, self.token_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_decentraland_nft_urn() {
        let urn = NftUrn::parse("urn:decentraland:ethereum:erc721:0xabc:42").unwrap();
        assert_eq!(urn.chain, "ethereum");
        assert_eq!(urn.contract_address, "0xabc");
        assert_eq!(urn.token_id, "42");
        assert_eq!(urn.hash(), "0xabc:42");
    }

    #[test]
    fn rejects_other_urns() {
        assert!(NftUrn::parse("urn:other:ethereum:erc721:0xabc:42").is_none());
        assert!(NftUrn::parse("urn:decentraland:ethereum").is_none());
        assert!(NftUrn::parse("not a urn").is_none());
        assert!(NftUrn::parse("").is_none());
    }
}
