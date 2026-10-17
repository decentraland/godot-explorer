use std::{
    collections::VecDeque,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

use async_trait::async_trait;
use chrono::TimeZone;
use serde_json::{json, Value};
use sha2::Digest;

use super::*;

const EMPTY_BODY_HASH: &str = "47DEQpj8HBSa-_TImW-5JCeuQeRkm5NMpJWZG3hSuFU";

struct FakeHost {
    platform: Platform,
    supported: AtomicBool,
    integrity: Mutex<VecDeque<Result<String, String>>>,
    keys: Mutex<VecDeque<Result<String, String>>>,
    attestations: Mutex<VecDeque<Result<String, String>>>,
    hashes_seen: Mutex<Vec<String>>,
    cdh_seen: Mutex<Vec<Vec<u8>>>,
    calls: AtomicUsize,
}

impl FakeHost {
    fn new(platform: Platform) -> Arc<Self> {
        Arc::new(Self {
            platform,
            supported: AtomicBool::new(true),
            integrity: Mutex::default(),
            keys: Mutex::default(),
            attestations: Mutex::default(),
            hashes_seen: Mutex::default(),
            cdh_seen: Mutex::default(),
            calls: AtomicUsize::new(0),
        })
    }

    fn android() -> Arc<Self> {
        let host = Self::new(Platform::Android);
        host.integrity
            .lock()
            .unwrap()
            .push_back(Ok("pi-jws".into()));
        host
    }

    fn next(
        queue: &Mutex<VecDeque<Result<String, String>>>,
        default: &str,
    ) -> Result<String, String> {
        queue
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| Ok(default.into()))
    }
}

#[async_trait]
impl AttestationHost for FakeHost {
    fn platform(&self) -> Platform {
        self.platform
    }
    fn is_supported(&self) -> bool {
        self.supported.load(Ordering::SeqCst)
    }
    async fn play_integrity_token(&self, hash: String) -> Result<String, String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.hashes_seen.lock().unwrap().push(hash);
        Self::next(&self.integrity, "pi-jws")
    }
    async fn ios_generate_key(&self) -> Result<String, String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Self::next(&self.keys, "key-1")
    }
    async fn ios_attest_key(&self, _key: String, cdh: Vec<u8>) -> Result<String, String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.cdh_seen.lock().unwrap().push(cdh);
        Self::next(&self.attestations, "attestation-b64u")
    }
}

#[derive(Debug, Clone)]
struct Call {
    url: String,
    body: String,
    headers: Vec<(String, String)>,
    at: Instant,
}

type Handler = Box<dyn FnMut(&Call) -> Result<Value, HttpFailure> + Send>;

struct FakeTransport {
    calls: Mutex<Vec<Call>>,
    handler: Mutex<Handler>,
}

impl FakeTransport {
    fn new(handler: impl FnMut(&Call) -> Result<Value, HttpFailure> + Send + 'static) -> Arc<Self> {
        Arc::new(Self {
            calls: Mutex::default(),
            handler: Mutex::new(Box::new(handler)),
        })
    }

    fn ok_session(token: &'static str, ttl_secs: i64) -> Arc<Self> {
        Self::new(move |_| Ok(session_body(token, ttl_secs)))
    }

    fn calls(&self) -> Vec<Call> {
        self.calls.lock().unwrap().clone()
    }
}

#[async_trait]
impl Transport for FakeTransport {
    async fn post_json(
        &self,
        url: &str,
        body: String,
        headers: &[(&str, &str)],
    ) -> Result<Value, HttpFailure> {
        let call = Call {
            url: url.into(),
            body,
            headers: headers
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            at: Instant::now(),
        };
        self.calls.lock().unwrap().push(call.clone());
        (self.handler.lock().unwrap())(&call)
    }
}

fn session_body(token: &str, ttl_secs: i64) -> Value {
    let expires = chrono::Utc.timestamp_opt(unix_now() + ttl_secs, 0).unwrap();
    json!({"token": token, "expires_at": expires.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()})
}

fn failure(code: Option<&str>) -> HttpFailure {
    HttpFailure {
        code: code.map(str::to_owned),
        message: "boom".into(),
    }
}

fn eula(accepted: bool) -> (Arc<AtomicBool>, EulaCheck) {
    let flag = Arc::new(AtomicBool::new(accepted));
    let seen = flag.clone();
    (flag, Arc::new(move || seen.load(Ordering::SeqCst)))
}

fn build(
    host: Arc<FakeHost>,
    transport: Arc<FakeTransport>,
    dir: &tempfile::TempDir,
) -> Attestation {
    Attestation::with_parts(
        host,
        transport,
        eula(true).1,
        dir.path().join(SESSION_FILE),
        false,
    )
}

fn write_legacy(dir: &tempfile::TempDir, token: &str, expires_at: i64) {
    let legacy = json!({"token": token, "expires_at": expires_at, "platform": "android"});
    std::fs::write(dir.path().join(SESSION_FILE), legacy.to_string()).unwrap();
}

#[tokio::test]
async fn cache_hit_returns_without_touching_host_or_network() {
    let dir = tempfile::tempdir().unwrap();
    write_legacy(&dir, "cached-token", unix_now() + 3600);
    let host = FakeHost::android();
    let transport = FakeTransport::ok_session("new", 3600);
    let attest = build(host.clone(), transport.clone(), &dir);

    assert_eq!(attest.cache_load(), CacheLoad::Hit);
    assert!(attest.has_valid_session());
    assert_eq!(attest.valid_jwt().await.unwrap(), "cached-token");
    assert_eq!(host.calls.load(Ordering::SeqCst), 0);
    assert!(transport.calls().is_empty());
}

#[tokio::test]
async fn cache_hit_is_served_even_when_unsupported() {
    let dir = tempfile::tempdir().unwrap();
    write_legacy(&dir, "cached-token", unix_now() + 3600);
    let host = FakeHost::android();
    host.supported.store(false, Ordering::SeqCst);
    let attest = build(host, FakeTransport::ok_session("new", 3600), &dir);
    assert_eq!(attest.valid_jwt().await.unwrap(), "cached-token");
}

#[test]
fn expiry_margin_is_sixty_seconds() {
    let now = 1_000_000;
    let session = |expires_at| Session {
        token: "t".into(),
        expires_at,
    };
    assert!(session(now + 61).is_valid_at(now));
    assert!(!session(now + 60).is_valid_at(now));
    assert!(!session(now + 30).is_valid_at(now));
    assert!(!session(now - 5).is_valid_at(now));
}

#[tokio::test]
async fn token_inside_margin_is_a_miss_and_reattests() {
    let dir = tempfile::tempdir().unwrap();
    write_legacy(&dir, "stale", unix_now() + 30);
    let attest = build(
        FakeHost::android(),
        FakeTransport::ok_session("fresh", 3600),
        &dir,
    );

    assert_eq!(attest.cache_load(), CacheLoad::MissExpired);
    assert!(!attest.has_valid_session());
    assert_eq!(attest.valid_jwt().await.unwrap(), "fresh");
    assert!(attest.has_valid_session());
}

#[test]
fn legacy_file_formats_load_or_report_why_not() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(SESSION_FILE);
    let now = 1_000_000;

    assert_eq!(store::load(&path, now), (None, CacheLoad::MissNoFile));

    std::fs::write(
        &path,
        r#"{"token":"abc","expires_at":2000000,"platform":"ios"}"#,
    )
    .unwrap();
    let (session, result) = store::load(&path, now);
    assert_eq!(result, CacheLoad::Hit);
    assert_eq!(
        session,
        Some(Session {
            token: "abc".into(),
            expires_at: 2_000_000
        })
    );

    for corrupted in [
        "not json",
        "[]",
        r#"{"token":"","expires_at":2000000}"#,
        r#"{"token":"a","expires_at":0}"#,
        r#"{"expires_at":5}"#,
    ] {
        std::fs::write(&path, corrupted).unwrap();
        assert_eq!(
            store::load(&path, now),
            (None, CacheLoad::MissCorrupted),
            "{corrupted}"
        );
    }
}

#[tokio::test]
async fn android_exchange_binds_empty_body_hash_and_persists_legacy_format() {
    let dir = tempfile::tempdir().unwrap();
    let host = FakeHost::android();
    let transport = FakeTransport::ok_session("session-token", 172_800);
    let attest = build(host.clone(), transport.clone(), &dir);

    assert_eq!(attest.valid_jwt().await.unwrap(), "session-token");

    assert_eq!(
        *host.hashes_seen.lock().unwrap(),
        vec![EMPTY_BODY_HASH.to_string()]
    );
    let calls = transport.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(
        calls[0].url,
        format!("{}/attest/session", urls::attestation_bff())
    );
    assert_eq!(calls[0].body, "");
    assert!(calls[0]
        .headers
        .contains(&("x-attest-platform".into(), "android".into())));
    assert!(calls[0]
        .headers
        .contains(&("x-attest-integrity-token".into(), "pi-jws".into())));

    let saved: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.path().join(SESSION_FILE)).unwrap())
            .unwrap();
    assert_eq!(saved["token"], "session-token");
    assert_eq!(saved["platform"], "android");
    assert!(saved["expires_at"].as_i64().unwrap() > unix_now() + 172_000);
    assert_eq!(attest.state(), State::Idle);
}

#[tokio::test]
async fn ios_exchange_hashes_the_decoded_challenge() {
    let dir = tempfile::tempdir().unwrap();
    let host = FakeHost::new(Platform::Ios);
    // base64url of the bytes [1, 2, 3, 4, 251].
    let transport = FakeTransport::new(|call| {
        if call.url.ends_with("/attest/ios/challenge") {
            Ok(json!({"challenge": "AQIDBPs", "expires_at": "x"}))
        } else {
            Ok(session_body("ios-token", 3600))
        }
    });
    let attest = build(host.clone(), transport.clone(), &dir);

    assert_eq!(attest.valid_jwt().await.unwrap(), "ios-token");

    let expected = sha2::Sha256::digest([1u8, 2, 3, 4, 251]).to_vec();
    assert_eq!(*host.cdh_seen.lock().unwrap(), vec![expected]);
    let calls = transport.calls();
    assert_eq!(calls.len(), 2);
    assert_eq!(
        calls[1].headers,
        vec![("x-attest-platform".to_string(), "ios".to_string())]
    );
    let body: Value = serde_json::from_str(&calls[1].body).unwrap();
    assert_eq!(
        body,
        json!({"key_id": "key-1", "attestation_object": "attestation-b64u", "challenge": "AQIDBPs"})
    );
}

#[tokio::test(start_paused = true)]
async fn retries_back_off_one_two_five_ten_then_thirty_seconds() {
    let dir = tempfile::tempdir().unwrap();
    let mut remaining_failures = 6;
    let transport = FakeTransport::new(move |_| {
        if remaining_failures > 0 {
            remaining_failures -= 1;
            return Err(failure(None));
        }
        Ok(session_body("finally", 3600))
    });
    let attest = build(FakeHost::android(), transport.clone(), &dir);

    assert_eq!(attest.valid_jwt().await.unwrap(), "finally");

    let times: Vec<_> = transport.calls().iter().map(|c| c.at).collect();
    let gaps: Vec<u64> = times.windows(2).map(|w| (w[1] - w[0]).as_secs()).collect();
    assert_eq!(gaps, vec![1, 2, 5, 10, 30, 30]);
    assert!(attest.last_failure().is_none());
}

#[tokio::test(start_paused = true)]
async fn concurrent_callers_share_one_cycle() {
    let dir = tempfile::tempdir().unwrap();
    let transport = FakeTransport::ok_session("shared", 3600);
    let host = FakeHost::android();
    let attest = build(host.clone(), transport.clone(), &dir);

    let (a, b, c) = tokio::join!(attest.valid_jwt(), attest.valid_jwt(), attest.valid_jwt());

    assert_eq!(
        (a.unwrap(), b.unwrap(), c.unwrap()),
        ("shared".into(), "shared".into(), "shared".into())
    );
    assert_eq!(transport.calls().len(), 1);
    assert_eq!(host.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test(start_paused = true)]
async fn boot_warmup_waits_for_the_eula_then_attests() {
    let dir = tempfile::tempdir().unwrap();
    let transport = FakeTransport::ok_session("warm", 3600);
    let (accepted, check) = eula(false);
    let attest = Attestation::with_parts(
        FakeHost::android(),
        transport.clone(),
        check,
        dir.path().join(SESSION_FILE),
        false,
    );

    attest.start();
    tokio::time::sleep(Duration::from_secs(5)).await;
    assert_eq!(attest.state(), State::WaitingEula);
    assert!(transport.calls().is_empty());

    accepted.store(true, Ordering::SeqCst);
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert_eq!(transport.calls().len(), 1);
    assert!(attest.has_valid_session());
    assert_eq!(attest.state(), State::Idle);
}

#[tokio::test(start_paused = true)]
async fn boot_warmup_is_skipped_with_a_valid_session() {
    let dir = tempfile::tempdir().unwrap();
    write_legacy(&dir, "cached", unix_now() + 3600);
    let transport = FakeTransport::ok_session("new", 3600);
    let attest = build(FakeHost::android(), transport.clone(), &dir);
    attest.start();
    tokio::time::sleep(Duration::from_secs(5)).await;
    assert!(transport.calls().is_empty());
}

#[tokio::test]
async fn unsupported_device_errors_instead_of_hanging() {
    let dir = tempfile::tempdir().unwrap();
    let host = FakeHost::android();
    host.supported.store(false, Ordering::SeqCst);
    let attest = build(host, FakeTransport::ok_session("x", 3600), &dir);

    assert!(!attest.is_supported());
    assert_eq!(attest.valid_jwt().await, Err(AttestError::Unsupported));
    assert_eq!(attest.force_reattest().await, Err(AttestError::Unsupported));
}

#[tokio::test(start_paused = true)]
async fn failures_surface_step_and_server_code() {
    let dir = tempfile::tempdir().unwrap();
    let mut first = true;
    let transport = FakeTransport::new(move |_| {
        if std::mem::take(&mut first) {
            return Err(failure(Some("ATTESTATION_ANDROID_BAD_TOKEN")));
        }
        Ok(session_body("ok", 3600))
    });
    let attest = build(FakeHost::android(), transport, &dir);
    let reports = Arc::new(Mutex::new(Vec::new()));
    let sink = reports.clone();
    attest.on_attempt(Arc::new(move |r| sink.lock().unwrap().push(r.clone())));

    attest.valid_jwt().await.unwrap();

    let reports = reports.lock().unwrap();
    assert_eq!(reports.len(), 2);
    let failed = reports[0].outcome.as_ref().unwrap_err();
    assert_eq!(
        (failed.step, failed.code.as_str()),
        ("post_session", "ATTESTATION_ANDROID_BAD_TOKEN")
    );
    assert_eq!((reports[0].attempt, reports[1].attempt), (1, 2));
    assert_eq!(reports[0].cycle_id, reports[1].cycle_id);
    assert_eq!(reports[0].trigger, Trigger::OnDemand);
    assert!(reports[1].outcome.is_ok());
    assert!(reports[1].timings.play_integrity_ms.is_some());
}

#[tokio::test(start_paused = true)]
async fn host_and_response_errors_are_client_errors_with_their_step() {
    let dir = tempfile::tempdir().unwrap();
    let host = FakeHost::new(Platform::Android);
    {
        let mut queue = host.integrity.lock().unwrap();
        queue.push_back(Err("IntegrityErrorCode -13".into()));
        queue.push_back(Ok(String::new()));
        queue.push_back(Ok("pi".into()));
        queue.push_back(Ok("pi".into()));
    }
    let mut replies = VecDeque::from([json!({}), json!({"token": "t", "expires_at": "garbage"})]);
    let transport = FakeTransport::new(move |_| {
        Ok(replies
            .pop_front()
            .unwrap_or_else(|| session_body("ok", 3600)))
    });
    let attest = build(host, transport, &dir);
    let steps = Arc::new(Mutex::new(Vec::new()));
    let sink = steps.clone();
    attest.on_attempt(Arc::new(move |r| {
        if let Err(f) = &r.outcome {
            sink.lock()
                .unwrap()
                .push((f.step, f.code.clone(), f.message.clone()));
        }
    }));

    attest.valid_jwt().await.unwrap();

    let steps = steps.lock().unwrap();
    let summary: Vec<_> = steps.iter().map(|(s, c, _)| (*s, c.as_str())).collect();
    assert_eq!(
        summary,
        vec![
            ("play_integrity", "client_error"),
            ("play_integrity", "client_error"),
            ("post_session", "client_error"),
            ("post_session", "client_error"),
        ]
    );
    assert!(steps[0].2.contains("IntegrityErrorCode -13"));
    assert!(steps[3].2.contains("bad expires_at"));
}

#[tokio::test(start_paused = true)]
async fn ios_challenge_and_key_failures_are_reported_per_step() {
    let dir = tempfile::tempdir().unwrap();
    let host = FakeHost::new(Platform::Ios);
    host.keys
        .lock()
        .unwrap()
        .push_back(Err("not allowed".into()));
    let mut challenges = VecDeque::from([
        json!({"challenge": ""}),
        json!({"challenge": "AQID"}),
        json!({"challenge": "AQID"}),
    ]);
    let transport = FakeTransport::new(move |call| {
        if call.url.ends_with("/challenge") {
            Ok(challenges
                .pop_front()
                .unwrap_or_else(|| json!({"challenge": "AQID"})))
        } else {
            Ok(session_body("ios", 3600))
        }
    });
    let attest = build(host, transport, &dir);
    let steps = Arc::new(Mutex::new(Vec::new()));
    let sink = steps.clone();
    attest.on_attempt(Arc::new(move |r| {
        if let Err(f) = &r.outcome {
            sink.lock().unwrap().push(f.step);
        }
    }));

    attest.valid_jwt().await.unwrap();
    assert_eq!(*steps.lock().unwrap(), vec!["challenge", "generate_key"]);
}

#[tokio::test(start_paused = true)]
async fn force_reattest_drops_cache_and_file_and_fetches_a_new_token() {
    let dir = tempfile::tempdir().unwrap();
    write_legacy(&dir, "rejected", unix_now() + 3600);
    let transport = FakeTransport::ok_session("rotated", 3600);
    let attest = build(FakeHost::android(), transport.clone(), &dir);
    assert_eq!(attest.valid_jwt().await.unwrap(), "rejected");

    assert_eq!(attest.force_reattest().await.unwrap(), "rotated");

    assert_eq!(transport.calls().len(), 1);
    assert_eq!(attest.valid_jwt().await.unwrap(), "rotated");
    let saved = std::fs::read_to_string(dir.path().join(SESSION_FILE)).unwrap();
    assert!(saved.contains("rotated") && !saved.contains("rejected"));
}

#[tokio::test]
async fn production_pins_the_bff_and_other_builds_use_urls() {
    for (production, expected) in [
        (true, PROD_ATTESTATION_BFF.to_string()),
        (false, urls::attestation_bff()),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let transport = FakeTransport::ok_session("t", 3600);
        let attest = Attestation::with_parts(
            FakeHost::android(),
            transport.clone(),
            eula(true).1,
            dir.path().join(SESSION_FILE),
            production,
        );
        attest.valid_jwt().await.unwrap();
        assert_eq!(
            transport.calls()[0].url,
            format!("{expected}/attest/session")
        );
    }
}

#[test]
fn secrets_are_masked_in_logs() {
    assert_eq!(flow::mask(""), "<empty>");
    assert_eq!(flow::mask("short"), "<short:5>");
    assert_eq!(flow::mask("abcdefghijklmnop"), "abcdefghijkl…(len=16)");
}
