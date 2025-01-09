pub mod http_queue_requester;
pub mod request_response;
pub mod rust_http_queue_requester;

#[cfg(target_arch = "wasm32")]
pub mod web_http_requester;