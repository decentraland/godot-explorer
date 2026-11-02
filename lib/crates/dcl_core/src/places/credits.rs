//! The credits server: the spendable balance of the signed-in wallet.

use super::signed::{build_url, SignedClient, SignedError};
use crate::urls;

/// Spendable credits on the USD rail the Shop prices in. An environment without that rail reports
/// 0 rather than the legacy MANA total, which is a different unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct CreditsBalance {
    pub credits: i64,
}

pub async fn credits_balance(client: &SignedClient) -> Result<CreditsBalance, SignedError> {
    let url = build_url(
        &urls::credits_server(),
        &["users", client.address(), "credits"],
        &[],
    )?;
    let body = client.request_json("GET", &url, None).await?;
    let credits = body
        .pointer("/usd/credits")
        .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)))
        .unwrap_or(0);
    Ok(CreditsBalance { credits })
}

#[cfg(test)]
mod tests {
    use super::super::signed::testing::*;
    use super::*;

    #[tokio::test]
    async fn balance_reads_only_the_usd_rail_of_the_wallet_path() {
        let mock = Mock::ok(r#"{"usd":{"credits":42},"totals":{"nonExpiring":900}}"#);
        let balance = credits_balance(&client(&mock)).await.unwrap();
        let sent = mock.only_sent();
        assert_eq!(sent.method, "GET");
        assert_eq!(
            sent.url,
            format!(
                "{}/users/0xabc0000000000000000000000000000000000001/credits",
                urls::credits_server()
            )
        );
        assert_eq!(balance, CreditsBalance { credits: 42 });
    }

    #[tokio::test]
    async fn a_missing_usd_rail_is_zero() {
        let mock = Mock::ok(r#"{"totals":{"nonExpiring":900}}"#);
        assert_eq!(credits_balance(&client(&mock)).await.unwrap().credits, 0);
    }
}
