use std::collections::HashMap;

use godot::{
    obj::Gd,
    prelude::{GString, Variant},
};

use crate::godot_classes::promise::{Promise, PromiseDeferred};

#[derive(Debug)]
pub enum ResponseEnum {
    String(String),
    Bytes(Vec<u8>),
    ToFile(Result<String, std::io::Error>),
    Json(Result<serde_json::Value, serde_json::Error>),
}

#[derive(Debug, Clone)]
pub enum ResponseType {
    AsString,
    #[allow(dead_code)]
    AsBytes,
    AsJson,
    ToFile(String),
}

static REQUEST_ID_COUNTER: once_cell::sync::Lazy<std::sync::atomic::AtomicU32> =
    once_cell::sync::Lazy::new(Default::default);

#[derive(Debug)]
pub struct RequestOption {
    pub id: u32,
    pub reference_id: u32,
    pub url: String,
    pub method: http::Method,
    pub body: Option<Vec<u8>>,
    pub headers: Option<HashMap<String, String>>,
    pub response_type: ResponseType,
    pub timeout: Option<std::time::Duration>,
}

impl RequestOption {
    pub fn new(
        reference_id: u32,
        url: String,
        method: http::Method,
        response_type: ResponseType,
        body: Option<Vec<u8>>,
        headers: Option<HashMap<String, String>>,
        timeout: Option<std::time::Duration>,
    ) -> Self {
        Self {
            id: REQUEST_ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
            reference_id,
            url,
            method,
            body,
            headers,
            response_type,
            timeout,
        }
    }
}

#[derive(Debug, godot::prelude::GodotClass)]
#[class(no_init)]
pub struct RequestResponse {
    pub request_option: RequestOption,
    pub status_code: http::StatusCode,
    pub headers: Option<HashMap<String, String>>,
    pub response_data: Result<ResponseEnum, String>,
}

#[godot::prelude::godot_api]
impl RequestResponse {
    #[func]
    pub fn status_code(&self) -> i32 {
        self.status_code.as_u16() as i32
    }

    #[func]
    pub fn is_error(&self) -> bool {
        self.response_data.is_err()
    }

    #[func]
    pub fn get_error(&self) -> GString {
        GString::from(self.response_data.as_ref().unwrap_err())
    }

    #[func]
    pub fn id(&self) -> u32 {
        self.request_option.id
    }

    #[func]
    pub fn reference_id(&self) -> u32 {
        self.request_option.reference_id
    }

    #[func]
    pub fn get_string_response_as_json(&mut self) -> Variant {
        let response = self.response_data.as_ref().unwrap();

        match response {
            ResponseEnum::String(string) => godot::classes::Json::parse_string(string),
            _ => Variant::default(),
        }
    }

    #[func]
    pub fn get_response_as_string(&self) -> Variant {
        let response = self.response_data.as_ref().unwrap();

        match response {
            ResponseEnum::String(string) => Variant::from(string.as_str()),
            ResponseEnum::Json(json) => {
                if let Ok(result) = json {
                    Variant::from(result.to_string().as_str())
                } else {
                    Variant::nil()
                }
            }
            ResponseEnum::ToFile(path) => {
                if let Ok(result) = path {
                    Variant::from(result.as_str())
                } else {
                    Variant::nil()
                }
            }
            ResponseEnum::Bytes(bytes) => {
                if let Ok(result) = String::from_utf8(bytes.to_vec()) {
                    Variant::from(result.as_str())
                } else {
                    Variant::nil()
                }
            }
        }
    }
}

#[derive(Debug, Default, godot::prelude::GodotClass)]
#[class(init)]
pub struct RequestResponseError {
    pub id: u32,
    pub error_message: String,
}

#[godot::prelude::godot_api]
impl RequestResponseError {
    #[func]
    pub fn id(&self) -> u32 {
        self.id
    }

    #[func]
    pub fn get_error_message(&self) -> GString {
        self.error_message.clone().as_str().into()
    }
}

/// Runs on a tokio task: the promise is resolved through the main thread
/// (`PromiseDeferred`), never by binding it mutably from here.
pub fn send_result_to_promise(
    result: Result<RequestResponse, RequestResponseError>,
    mut promise: Gd<Promise>,
) {
    match result {
        Ok(response) => {
            let status_code = response.status_code();
            if response.is_error() {
                promise.reject_deferred(response.get_error());
            } else if !(200..=299).contains(&status_code) {
                let payload = response.get_response_as_string();
                if payload.is_nil() {
                    let reason = http::StatusCode::from_u16(status_code as u16)
                        .ok()
                        .and_then(|code| code.canonical_reason())
                        .unwrap_or("unknown reason");
                    promise.reject_deferred(GString::from(reason));
                } else {
                    promise.reject_deferred(payload.to_string().as_str().into());
                }
            } else {
                promise.resolve_with_data_deferred(Variant::from(Gd::from_object(response)));
            }
        }
        Err(error) => {
            promise.reject_deferred(error.get_error_message());
        }
    }
}
