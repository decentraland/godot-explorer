use std::collections::HashMap;

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

#[derive(Debug)]
pub struct HttpResponse {
    pub request_option: RequestOption,
    pub status_code: http::StatusCode,
    pub headers: Option<HashMap<String, String>>,
    pub response_data: Result<ResponseEnum, String>,
}

#[derive(Debug, Default)]
pub struct HttpError {
    pub id: u32,
    pub error_message: String,
}
