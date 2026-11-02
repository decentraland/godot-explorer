//! The engine's HTTP requester as plain futures over `serde_json`.

use super::globals;
use godot::classes::http_client::Method;
use godot::prelude::*;
use serde_json::Value;

use crate::{
    engine::local_task::await_promise, godot_classes::promise::PromiseError,
    http_request::request_response::RequestResponse,
};

/// The response body as text, or why there is none (rejections carry only a message).
pub async fn request_text(
    url: &str,
    method: Method,
    body: &str,
    headers: VarDictionary,
) -> Result<String, String> {
    let requester = globals::http_requester();
    let promise =
        requester
            .bind()
            .request_json(GString::from(url), method, GString::from(body), headers);
    let result = await_promise(promise).await;
    if let Ok(error) = result.try_to::<Gd<PromiseError>>() {
        return Err(error.bind().get_error().to_string());
    }
    let response = result
        .try_to::<Gd<RequestResponse>>()
        .map_err(|_| "unexpected response".to_string())?;
    let text = response.bind().get_response_as_string();
    text.try_to::<GString>()
        .map(|text| text.to_string())
        .map_err(|_| "response is not text".to_string())
}

pub async fn get_json(url: &str) -> Option<Value> {
    let text = request_text(url, Method::GET, "", VarDictionary::new())
        .await
        .ok()?;
    serde_json::from_str(&text).ok()
}
