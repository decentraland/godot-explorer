//! Remote feature flags served by the mobile-bff, fetched once at boot.
//!
//! Fail-open: on timeout, network error or a malformed payload every flag keeps its default.
//! Callers choose the default per flag (`pulse` and `sentry-error-events` default to off).

use std::{
    sync::{LazyLock, RwLock},
    time::Duration,
};

use serde_json::{Map, Value};

use crate::http::{
    request_response::{HttpResponse, RequestOption, ResponseEnum, ResponseType},
    HttpQueueRequester,
};

const FETCH_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Default)]
struct State {
    flags: Map<String, Value>,
    loaded: bool,
}

static STATE: LazyLock<RwLock<State>> = LazyLock::new(Default::default);

/// `{"ok": true, "data": {"flags": {...}}}` → the flags object; empty on any shape mismatch.
pub fn parse_response(json: &Value) -> Map<String, Value> {
    if json.get("ok").and_then(Value::as_bool) != Some(true) {
        return Map::new();
    }
    json.pointer("/data/flags")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default()
}

fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|n| n != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

fn read() -> std::sync::RwLockReadGuard<'static, State> {
    STATE.read().unwrap_or_else(|e| e.into_inner())
}

/// `default` is returned while the flags are not loaded or when the flag is absent.
pub fn is_enabled(name: &str, default: bool) -> bool {
    read().flags.get(name).map_or(default, truthy)
}

/// `default` is returned while the flags are not loaded, when the flag is absent or not a number.
pub fn get_number(name: &str, default: f64) -> f64 {
    read()
        .flags
        .get(name)
        .and_then(Value::as_f64)
        .unwrap_or(default)
}

pub fn is_loaded() -> bool {
    read().loaded
}

pub fn all() -> Map<String, Value> {
    read().flags.clone()
}

fn store(flags: Map<String, Value>) {
    let mut state = STATE.write().unwrap_or_else(|e| e.into_inner());
    state.flags = flags;
    state.loaded = true;
    drop(state);
    crate::analytics::push::on_flags_loaded();
}

fn flags_from_response(response: HttpResponse) -> Result<Map<String, Value>, String> {
    let status = response.status_code;
    if !status.is_success() {
        return Err(format!("feature_flags: HTTP {status}"));
    }
    match response.response_data? {
        ResponseEnum::Json(Ok(json)) => Ok(parse_response(&json)),
        ResponseEnum::Json(Err(e)) => Err(format!("feature_flags: invalid JSON: {e}")),
        _ => Err("feature_flags: unexpected response".to_string()),
    }
}

/// Fetches and stores the flags. The flags count as loaded afterwards even on failure, so waiters
/// proceed with the defaults.
pub async fn fetch(requester: &HttpQueueRequester, url: &str) -> Result<(), String> {
    let request = RequestOption::new(
        0,
        url.to_string(),
        http::Method::GET,
        ResponseType::AsJson,
        None,
        None,
        None,
    );
    let result = match tokio::time::timeout(FETCH_TIMEOUT, requester.request(request, 0)).await {
        Err(_) => Err("feature_flags: timeout".to_string()),
        Ok(Err(e)) => Err(e.error_message),
        Ok(Ok(response)) => flags_from_response(response),
    };
    match result {
        Ok(flags) => {
            store(flags);
            Ok(())
        }
        Err(e) => {
            store(Map::new());
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_the_bff_payload() {
        let flags =
            parse_response(&json!({"ok": true, "data": {"flags": {"pulse": true, "rate": 0.5}}}));
        assert_eq!(flags.get("pulse"), Some(&json!(true)));
        assert_eq!(flags.len(), 2);
    }

    #[test]
    fn any_shape_mismatch_is_empty() {
        assert!(parse_response(&json!({"ok": false, "data": {"flags": {"a": true}}})).is_empty());
        assert!(parse_response(&json!({"ok": true, "data": []})).is_empty());
        assert!(parse_response(&json!({"ok": true, "data": {"flags": 3}})).is_empty());
        assert!(parse_response(&json!(null)).is_empty());
    }

    #[test]
    fn truthiness_follows_the_old_gdscript_bool_cast() {
        assert!(truthy(&json!(true)) && truthy(&json!(2)) && truthy(&json!("x")));
        assert!(!truthy(&json!(false)) && !truthy(&json!(0)) && !truthy(&json!("")));
        assert!(!truthy(&Value::Null));
    }

    // One test: the flags are process-wide state.
    #[test]
    fn defaults_apply_until_flags_are_stored() {
        assert!(!is_loaded());
        assert!(is_enabled("archipielago", true));
        assert!(!is_enabled("pulse", false));
        assert_eq!(get_number("rate", 1.0), 1.0);

        store(parse_response(
            &json!({"ok": true, "data": {"flags": {"archipielago": false, "pulse": true, "rate": 0.25, "name": "x"}}}),
        ));
        assert!(is_loaded());
        assert!(!is_enabled("archipielago", true));
        assert!(is_enabled("pulse", false));
        assert!(is_enabled("absent", true));
        assert_eq!(get_number("rate", 1.0), 0.25);
        assert_eq!(
            get_number("name", 1.0),
            1.0,
            "a non-number keeps the default"
        );
    }
}
