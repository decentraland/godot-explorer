use godot::{
    obj::Gd,
    prelude::{GString, Variant},
};

use crate::godot_classes::promise::{Promise, PromiseDeferred};

pub use dcl_core::http::request_response::{
    HttpError, HttpResponse, RequestOption, ResponseEnum, ResponseType,
};

#[derive(Debug, godot::prelude::GodotClass)]
#[class(no_init)]
pub struct RequestResponse {
    inner: HttpResponse,
}

impl RequestResponse {
    pub fn new(inner: HttpResponse) -> Self {
        Self { inner }
    }
}

impl std::ops::Deref for RequestResponse {
    type Target = HttpResponse;

    fn deref(&self) -> &HttpResponse {
        &self.inner
    }
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
    inner: HttpError,
}

impl RequestResponseError {
    pub fn new(inner: HttpError) -> Self {
        Self { inner }
    }
}

#[godot::prelude::godot_api]
impl RequestResponseError {
    #[func]
    pub fn id(&self) -> u32 {
        self.inner.id
    }

    #[func]
    pub fn get_error_message(&self) -> GString {
        self.inner.error_message.as_str().into()
    }
}

/// Runs on a tokio task: the promise is resolved through the main thread
/// (`PromiseDeferred`), never by binding it mutably from here.
pub fn send_result_to_promise(result: Result<HttpResponse, HttpError>, mut promise: Gd<Promise>) {
    match result {
        Ok(response) => {
            let response = RequestResponse::new(response);
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
            promise.reject_deferred(RequestResponseError::new(error).get_error_message());
        }
    }
}
