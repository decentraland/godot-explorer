//! The saved account of a profile slot. The core writes JSON; 1.x wrote the account as Godot
//! `var_to_str` text, which is read here so an upgraded install keeps its session (R-6).

use base64::Engine as _;
use serde_json::Value;

use super::{
    ephemeral_auth_chain::EphemeralAuthChain, identity::RecoverableAccount, wallet::AsH160,
};

pub fn encode(account: &RecoverableAccount) -> String {
    let chain = serde_json::to_string(&account.ephemeral_auth_chain)
        .expect("serialize ephemeral auth chain");
    let mut object = serde_json::json!({
        "account_address": format!("{:#x}", account.account_address),
        "chain_id": account.chain_id,
        "ephemeral_auth_chain": chain,
    });
    if !account.local_wallet.is_empty() {
        object["local_wallet"] = account.local_wallet.clone().into();
    }
    object.to_string()
}

/// `None` for an empty slot, a malformed value or a chain that cannot be parsed.
pub fn decode(text: &str) -> Option<RecoverableAccount> {
    let value: Value = serde_json::from_str(text)
        .ok()
        .or_else(|| LegacyParser::new(text).dictionary())?;
    let object = value.as_object()?;
    let account_address = object.get("account_address")?.as_str()?.as_h160()?;
    let chain_id = object.get("chain_id")?.as_u64()?;
    let ephemeral_auth_chain: EphemeralAuthChain =
        serde_json::from_str(object.get("ephemeral_auth_chain")?.as_str()?).ok()?;
    let local_wallet = match object.get("local_wallet") {
        Some(Value::Array(bytes)) => bytes
            .iter()
            .map(|b| b.as_u64().and_then(|b| u8::try_from(b).ok()))
            .collect::<Option<Vec<u8>>>()?,
        _ => Vec::new(),
    };
    Some(RecoverableAccount {
        account_address,
        chain_id,
        ephemeral_auth_chain,
        local_wallet,
    })
}

/// Reads the part of Godot's text format a saved account uses: a dictionary of strings, integers
/// and byte arrays (`PackedByteArray(1, 2)` or `PackedByteArray("base64")`). Byte arrays become
/// JSON arrays of numbers.
struct LegacyParser<'a> {
    rest: &'a str,
}

impl<'a> LegacyParser<'a> {
    fn new(text: &'a str) -> Self {
        Self { rest: text }
    }

    fn dictionary(mut self) -> Option<Value> {
        self.expect('{')?;
        let mut map = serde_json::Map::new();
        loop {
            self.skip_space();
            if self.eat('}') {
                return Some(Value::Object(map));
            }
            let key = self.string()?;
            self.expect(':')?;
            let value = self.value()?;
            map.insert(key, value);
            self.skip_space();
            if !self.eat(',') && !self.rest.starts_with('}') {
                return None;
            }
        }
    }

    fn value(&mut self) -> Option<Value> {
        self.skip_space();
        match self.rest.chars().next()? {
            '"' => self.string().map(Value::String),
            c if c == '-' || c.is_ascii_digit() => self.number(),
            c if c.is_ascii_alphabetic() => self.constructor(),
            _ => None,
        }
    }

    fn number(&mut self) -> Option<Value> {
        let end = self
            .rest
            .find(|c: char| !(c == '-' || c == '.' || c.is_ascii_digit()))
            .unwrap_or(self.rest.len());
        let (number, rest) = self.rest.split_at(end);
        self.rest = rest;
        number
            .parse::<i64>()
            .map(Value::from)
            .ok()
            .or_else(|| number.parse::<f64>().ok().map(Value::from))
    }

    /// `Name(...)`: only `PackedByteArray` carries data the core needs.
    fn constructor(&mut self) -> Option<Value> {
        let open = self.rest.find('(')?;
        let name = self.rest[..open].to_string();
        self.rest = &self.rest[open + 1..];
        self.skip_space();
        let bytes = if self.rest.starts_with('"') {
            let encoded = self.string()?;
            base64::engine::general_purpose::STANDARD
                .decode(encoded)
                .ok()?
        } else {
            let close = self.rest.find(')')?;
            let inner = &self.rest[..close];
            self.rest = &self.rest[close..];
            inner
                .split(',')
                .map(str::trim)
                .filter(|part| !part.is_empty())
                .map(|part| part.parse::<u8>().ok())
                .collect::<Option<Vec<u8>>>()?
        };
        self.expect(')')?;
        (name == "PackedByteArray").then(|| Value::from(bytes))
    }

    fn string(&mut self) -> Option<String> {
        self.skip_space();
        self.expect('"')?;
        let mut out = String::new();
        let mut chars = self.rest.char_indices();
        while let Some((i, c)) = chars.next() {
            match c {
                '"' => {
                    self.rest = &self.rest[i + 1..];
                    return Some(out);
                }
                '\\' => match chars.next()?.1 {
                    'n' => out.push('\n'),
                    't' => out.push('\t'),
                    'r' => out.push('\r'),
                    other => out.push(other),
                },
                other => out.push(other),
            }
        }
        None
    }

    fn skip_space(&mut self) {
        self.rest = self.rest.trim_start();
    }

    fn eat(&mut self, c: char) -> bool {
        self.skip_space();
        match self.rest.strip_prefix(c) {
            Some(rest) => {
                self.rest = rest;
                true
            }
            None => false,
        }
    }

    fn expect(&mut self, c: char) -> Option<()> {
        self.eat(c).then_some(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::auth_identity::create_local_ephemeral;
    use ethers_signers::{LocalWallet, Signer};
    use rand::thread_rng;

    fn account(local_wallet: Vec<u8>) -> RecoverableAccount {
        let wallet = LocalWallet::new(&mut thread_rng());
        RecoverableAccount {
            account_address: wallet.address(),
            chain_id: 1,
            ephemeral_auth_chain: create_local_ephemeral(&wallet),
            local_wallet,
        }
    }

    fn same(a: &RecoverableAccount, b: &RecoverableAccount) {
        assert_eq!(a.account_address, b.account_address);
        assert_eq!(a.chain_id, b.chain_id);
        assert_eq!(a.local_wallet, b.local_wallet);
        assert_eq!(
            serde_json::to_string(&a.ephemeral_auth_chain).unwrap(),
            serde_json::to_string(&b.ephemeral_auth_chain).unwrap()
        );
    }

    #[test]
    fn json_round_trips_remote_and_local_accounts() {
        for local in [Vec::new(), vec![1, 2, 255]] {
            let original = account(local);
            same(&decode(&encode(&original)).unwrap(), &original);
        }
    }

    /// What 1.x saved: `var_to_str` of the dictionary `get_recover_account_to` filled.
    fn legacy_text(original: &RecoverableAccount, bytes: Option<&str>) -> String {
        let chain = serde_json::to_string(&original.ephemeral_auth_chain)
            .unwrap()
            .replace('\\', "\\\\")
            .replace('"', "\\\"");
        let local = bytes
            .map(|b| format!("\"local_wallet\": {b},\n"))
            .unwrap_or_default();
        format!(
            "{{\n{local}\"account_address\": \"{:#x}\",\n\"chain_id\": 1,\n\"ephemeral_auth_chain\": \"{chain}\"\n}}",
            original.account_address
        )
    }

    #[test]
    fn reads_the_account_1x_saved_as_godot_text() {
        let original = account(Vec::new());
        same(&decode(&legacy_text(&original, None)).unwrap(), &original);
    }

    #[test]
    fn reads_legacy_byte_arrays_in_both_godot_forms() {
        let original = account(vec![7, 8, 9]);
        for form in ["PackedByteArray(7, 8, 9)", "PackedByteArray(\"BwgJ\")"] {
            same(
                &decode(&legacy_text(&original, Some(form))).unwrap(),
                &original,
            );
        }
    }

    #[test]
    fn empty_or_broken_slots_yield_nothing() {
        for text in [
            "",
            "{}",
            "{\n}",
            "not an account",
            "{ \"chain_id\": 1 }",
            "{\"a\": ",
        ] {
            assert!(decode(text).is_none(), "{text:?}");
        }
        let mut broken = account(Vec::new());
        broken.chain_id = 1;
        let mut json: Value = serde_json::from_str(&encode(&broken)).unwrap();
        json["ephemeral_auth_chain"] = "{}".into();
        assert!(decode(&json.to_string()).is_none());
    }
}
