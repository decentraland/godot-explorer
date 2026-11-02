//! One attestation attempt per platform: OS proof, then the session exchange with the mobile-bff.

use std::time::Duration;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tokio::time::Instant;

use super::{host::AttestationHost, store::Session, transport::Transport};
use crate::Platform;

/// Per-step durations in milliseconds; only the steps that ran are set.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Timings {
    pub challenge_ms: Option<u64>,
    pub generate_key_ms: Option<u64>,
    pub attest_key_ms: Option<u64>,
    pub play_integrity_ms: Option<u64>,
    pub post_session_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    pub step: &'static str,
    /// The server's `code`, or `client_error`.
    pub code: String,
    pub message: String,
    pub timings: Timings,
}

pub type Attempt = Result<(Session, Timings), Box<Failure>>;

const CLIENT_ERROR: &str = "client_error";

fn ms(since: Instant) -> u64 {
    since.elapsed().as_millis() as u64
}

pub async fn attempt(host: &dyn AttestationHost, transport: &dyn Transport, bff: &str) -> Attempt {
    match host.platform() {
        Platform::Ios => attest_ios(host, transport, bff).await,
        Platform::Android => attest_android(host, transport, bff).await,
        Platform::Desktop => Err(fail(
            "platform",
            CLIENT_ERROR,
            "unsupported platform".into(),
            Timings::default(),
        )),
    }
}

fn fail(step: &'static str, code: &str, message: String, timings: Timings) -> Box<Failure> {
    Box::new(Failure {
        step,
        code: code.to_owned(),
        message: format!("{step}: {message}"),
        timings,
    })
}

/// Fresh App Attest key per session: challenge, generateKey, attestKey(SHA256(challenge)), session.
async fn attest_ios(host: &dyn AttestationHost, transport: &dyn Transport, bff: &str) -> Attempt {
    let mut timings = Timings::default();

    let t0 = Instant::now();
    let challenge_resp = transport
        .post_json(&format!("{bff}/attest/ios/challenge"), String::new(), &[])
        .await;
    timings.challenge_ms = Some(ms(t0));
    let challenge_resp = challenge_resp.map_err(|e| {
        let code = e.code.unwrap_or_else(|| CLIENT_ERROR.into());
        fail("challenge", &code, e.message, timings.clone())
    })?;
    let challenge = challenge_resp
        .get("challenge")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    if challenge.is_empty() {
        return Err(fail(
            "challenge",
            CLIENT_ERROR,
            "challenge field empty".into(),
            timings,
        ));
    }

    let t0 = Instant::now();
    let key_id = host.ios_generate_key().await;
    timings.generate_key_ms = Some(ms(t0));
    let key_id = match key_id {
        Ok(id) if !id.is_empty() => id,
        Ok(_) => {
            return Err(fail(
                "generate_key",
                CLIENT_ERROR,
                "generateKey: empty key id".into(),
                timings,
            ))
        }
        Err(e) => {
            return Err(fail(
                "generate_key",
                CLIENT_ERROR,
                format!("generateKey: {e}"),
                timings,
            ))
        }
    };

    let t0 = Instant::now();
    let challenge_bytes = URL_SAFE_NO_PAD
        .decode(challenge.trim_end_matches('='))
        .unwrap_or_default();
    if challenge_bytes.is_empty() {
        return Err(fail(
            "attest_key",
            CLIENT_ERROR,
            "challenge decode empty".into(),
            timings,
        ));
    }
    let client_data_hash = Sha256::digest(&challenge_bytes).to_vec();
    let attestation = host.ios_attest_key(key_id.clone(), client_data_hash).await;
    timings.attest_key_ms = Some(ms(t0));
    let attestation_object = match attestation {
        Ok(object) if !object.is_empty() => object,
        Ok(_) => {
            return Err(fail(
                "attest_key",
                CLIENT_ERROR,
                "attestKey: empty attestation".into(),
                timings,
            ))
        }
        Err(e) => {
            return Err(fail(
                "attest_key",
                CLIENT_ERROR,
                format!("attestKey: {e}"),
                timings,
            ))
        }
    };

    let t0 = Instant::now();
    let body = json!({
        "key_id": key_id,
        "attestation_object": attestation_object,
        "challenge": challenge,
    })
    .to_string();
    let response = transport
        .post_json(
            &format!("{bff}/attest/session"),
            body,
            &[("x-attest-platform", "ios")],
        )
        .await;
    timings.post_session_ms = Some(ms(t0));
    session_from_response(response, timings)
}

/// The Play Integrity nonce is bound to SHA256 of the (empty) request body, as base64url without padding.
async fn attest_android(
    host: &dyn AttestationHost,
    transport: &dyn Transport,
    bff: &str,
) -> Attempt {
    let mut timings = Timings::default();
    let request_hash = URL_SAFE_NO_PAD.encode(Sha256::digest(b""));

    let t0 = Instant::now();
    let token = host.play_integrity_token(request_hash).await;
    timings.play_integrity_ms = Some(ms(t0));
    let token = match token {
        Ok(token) if !token.is_empty() => token,
        Ok(_) => {
            return Err(fail(
                "play_integrity",
                CLIENT_ERROR,
                "playIntegrity: empty token".into(),
                timings,
            ))
        }
        Err(e) => {
            return Err(fail(
                "play_integrity",
                CLIENT_ERROR,
                format!("playIntegrity: {e}"),
                timings,
            ))
        }
    };

    let t0 = Instant::now();
    let response = transport
        .post_json(
            &format!("{bff}/attest/session"),
            String::new(),
            &[
                ("x-attest-platform", "android"),
                ("x-attest-integrity-token", &token),
            ],
        )
        .await;
    timings.post_session_ms = Some(ms(t0));
    session_from_response(response, timings)
}

fn session_from_response(
    response: Result<Value, super::transport::HttpFailure>,
    timings: Timings,
) -> Attempt {
    let response = response.map_err(|e| {
        let code = e.code.unwrap_or_else(|| CLIENT_ERROR.into());
        fail("post_session", &code, e.message, timings.clone())
    })?;
    let token = response
        .get("token")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if token.is_empty() {
        return Err(fail(
            "post_session",
            CLIENT_ERROR,
            "session: missing token".into(),
            timings,
        ));
    }
    let raw_expiry = response
        .get("expires_at")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let Some(expires_at) = parse_expiry(raw_expiry) else {
        let message = format!("session: bad expires_at (raw={raw_expiry})");
        return Err(fail("post_session", CLIENT_ERROR, message, timings));
    };
    Ok((
        Session {
            token: token.to_owned(),
            expires_at,
        },
        timings,
    ))
}

/// ISO-8601 with a `Z` or offset, e.g. `2026-05-22T15:30:00.000Z`.
fn parse_expiry(raw: &str) -> Option<i64> {
    let unix = chrono::DateTime::parse_from_rfc3339(raw).ok()?.timestamp();
    (unix > 0).then_some(unix)
}

/// First characters of a bearer secret and its length, safe to log.
pub fn mask(secret: &str) -> String {
    match secret.chars().count() {
        0 => "<empty>".into(),
        n if n <= 12 => format!("<short:{n}>"),
        n => format!("{}…(len={n})", secret.chars().take(12).collect::<String>()),
    }
}

pub const RETRY_BACKOFF: [Duration; 5] = [
    Duration::from_secs(1),
    Duration::from_secs(2),
    Duration::from_secs(5),
    Duration::from_secs(10),
    Duration::from_secs(30),
];
