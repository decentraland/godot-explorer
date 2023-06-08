use godot::prelude::{GodotString, Variant};

#[derive(Debug)]
pub enum ResponseEnum {
    String(String),
    Bytes(Vec<u8>),
    ToFile(Result<(), std::io::Error>),
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
    pub method: reqwest::Method,
    pub body: Option<Vec<u8>>,
    pub headers: Option<Vec<String>>,
    pub response_type: ResponseType,
}

impl RequestOption {
    pub fn new(
        reference_id: u32,
        url: String,
        method: reqwest::Method,
        response_type: ResponseType,
        body: Option<Vec<u8>>,
        headers: Option<Vec<String>>,
    ) -> Self {
        Self {
            id: REQUEST_ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
            reference_id,
            url,
            method,
            body,
            headers,
            response_type,
        }
    }
}

#[derive(Debug, godot::prelude::GodotClass)]
pub struct RequestResponse {
    pub request_option: RequestOption,
    pub status_code: hyper::StatusCode,
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
            ResponseEnum::String(string) => {
                godot::engine::Json::parse_string(GodotString::from(string))
            }
            _ => Variant::default(),
        }
    }
}
