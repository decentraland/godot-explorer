use std::sync::Arc;

use godot::prelude::*;

use crate::{godot_classes::promise::Promise, scene_runner::tokio_runtime::TokioRuntime};

use super::request_response::send_result_to_promise;

// Deriving GodotClass makes the class available to Godot
#[derive(GodotClass)]
#[class(base=RefCounted)]
pub struct RustHttpQueueRequester {
    http_queue_requester: Arc<super::http_queue_requester::HttpQueueRequester>,
}

#[godot_api]
impl IRefCounted for RustHttpQueueRequester {
    fn init(_base: Base<RefCounted>) -> Self {
        Self {
            http_queue_requester: Arc::new(super::http_queue_requester::HttpQueueRequester::new(
                10,
            )),
        }
    }
}

#[godot_api]
impl RustHttpQueueRequester {
    #[func]
    fn request_file(&self, url: GString, absolute_path: GString) -> Gd<Promise> {
        let request_option = crate::http_request::request_response::RequestOption::new(
            0,
            url.to_string(),
            http::Method::GET,
            crate::http_request::request_response::ResponseType::ToFile(absolute_path.to_string()),
            None,
            None,
            None,
        );
        let http_requester = self.http_queue_requester.clone();
        let (ret_promise, get_promise) = Promise::make_to_async();
        TokioRuntime::spawn(async move {
            let result = http_requester.request(request_option, 0).await;
            let Some(promise) = get_promise() else {
                return;
            };
            send_result_to_promise(result, promise);
        });
        ret_promise
    }

    #[func]
    fn request_json(
        &self,
        url: GString,
        method: godot::engine::http_client::Method,
        body: GString,
        headers: VariantArray,
    ) -> Gd<Promise> {
        let body = match body.to_string().as_str() {
            "" => None,
            _ => Some(body.to_string().into_bytes()),
        };
        self._request_json(url, method, body, headers)
    }

    #[func]
    fn request_json_bin(
        &self,
        url: GString,
        method: godot::engine::http_client::Method,
        body: PackedByteArray,
        headers: VariantArray,
    ) -> Gd<Promise> {
        self._request_json(url, method, Some(body.to_vec()), headers)
    }
}

impl RustHttpQueueRequester {
    fn _request_json(
        &self,
        url: GString,
        method: godot::engine::http_client::Method,
        body: Option<Vec<u8>>,
        headers: VariantArray,
    ) -> Gd<Promise> {
        // tracing::info!("Requesting json: {:?}", url.to_string());

        let method = match method {
            godot::engine::http_client::Method::METHOD_POST => http::Method::POST,
            _ => http::Method::GET,
        };

        let headers = match headers.len() {
            0 => None,
            _ => {
                let mut headers_vec = Vec::new();
                for i in 0..headers.len() {
                    let header = headers.get(i).to_string();
                    headers_vec.push(header);
                }
                Some(headers_vec)
            }
        };

        let request_option = crate::http_request::request_response::RequestOption::new(
            0,
            url.to_string(),
            method,
            crate::http_request::request_response::ResponseType::AsString,
            body,
            headers,
            None,
        );
        let http_requester = self.http_queue_requester.clone();
        let (ret_promise, get_promise) = Promise::make_to_async();
        TokioRuntime::spawn(async move {
            let result = http_requester.request(request_option, 0).await;
            let Some(promise) = get_promise() else {
                return;
            };
            send_result_to_promise(result, promise);
        });
        ret_promise
    }
}
