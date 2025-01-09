use std::collections::{HashMap, VecDeque};

use godot::{engine::file_access::ModeFlags, prelude::*};
use serde_json::json;

use crate::content::packed_array::PackedByteArrayFromVec;

use super::{
    http_queue_requester::QueueRequest,
    request_response::{RequestResponse, RequestResponseError, ResponseEnum, ResponseType},
};

const MAX_PARALLEL_REQUESTS: usize = 8;

#[derive(Debug)]
struct OngoingRequest {
    client: Gd<godot::engine::HttpClient>,
    queue_request: QueueRequest,
    body_ready: bool,
    file_handle: Option<Gd<godot::engine::FileAccess>>,
    accumulated_bytes: Vec<u8>,
}

#[derive(GodotClass)]
#[class(base=Node, init)]
pub struct DclWebHttpRequester {
    _base: Base<Node>,

    pub web_pending_queue_requests: VecDeque<QueueRequest>,
    pub web_ongoing_requests: HashMap<u32, OngoingRequest>,

    pub tls_options: Option<Gd<godot::engine::TlsOptions>>,
}

#[godot_api]
impl INode for DclWebHttpRequester {
    fn process(&mut self, _delta: f64) {
        self.poll_web_fetch();
    }
}

#[godot_api]
impl DclWebHttpRequester {
    #[func]
    fn set_tls_options(&mut self, tls_options: Gd<godot::engine::TlsOptions>) {
        self.tls_options = Some(tls_options);
    }
}

impl DclWebHttpRequester {
    pub fn add_requests(&mut self, http_request: QueueRequest) {
        self.web_pending_queue_requests.push_back(http_request);
    }

    pub fn poll_web_fetch(&mut self) {
        // Process ongoing requests
        let mut completed_requests = Vec::new();

        for (id, ongoing) in self.web_ongoing_requests.iter_mut() {
            let mut client = ongoing.client.clone();
            let poll_err = client.poll();

            if poll_err != godot::engine::global::Error::OK {
                completed_requests.push((*id, Err(RequestResponseError {
                    id: ongoing.queue_request.id,
                    error_message: format!("Poll error: {:?} - {:?}", poll_err, client.get_status()),
                })));
                continue;
            }

            match client.get_status() {
                godot::engine::http_client::Status::BODY => {
                    ongoing.body_ready = true;
                    let chunk = client.read_response_body_chunk();
                    
                    if !chunk.is_empty() {
                        // Handle chunk based on response type
                        if let Some(file_handle) = &mut ongoing.file_handle {
                            // Write directly to file
                            file_handle.store_buffer(chunk);
                        } else {
                            // Accumulate in buffer
                            ongoing.accumulated_bytes.extend_from_slice(&chunk.as_slice());
                        }
                    }

                    if client.get_status() == godot::engine::http_client::Status::DISCONNECTED {
                        completed_requests.push((*id, Ok(())));
                    }
                }
                godot::engine::http_client::Status::CONNECTED
                | godot::engine::http_client::Status::REQUESTING => {
                    continue;
                }
                godot::engine::http_client::Status::DISCONNECTED
                | godot::engine::http_client::Status::CONNECTION_ERROR
                | godot::engine::http_client::Status::CANT_CONNECT
                | godot::engine::http_client::Status::CANT_RESOLVE => {
                    if ongoing.body_ready {
                        completed_requests.push((*id, Ok(())));
                    } else {
                        completed_requests.push((*id, Err(RequestResponseError {
                            id: ongoing.queue_request.id,
                            error_message: format!("Connection error: {:?}", client.get_status()),
                        })));
                    }
                }
                _ => {
                    completed_requests.push((*id, Err(RequestResponseError {
                        id: ongoing.queue_request.id,
                        error_message: format!("Connection error: {:?}", client.get_status()),
                    })));
                }
            }
        }

        // Remove completed requests
        for (id, result) in completed_requests {
            let Some(mut item) = self.web_ongoing_requests.remove(&id) else {
                continue;
            };

            match result {
                Ok(()) => {
                    let accumulated_bytes = std::mem::take(&mut item.accumulated_bytes);
                    let response = self.read_response(item.client, &mut item.queue_request, accumulated_bytes);
                    let _ = item.queue_request.response_sender.send(response);
                }
                Err(err) => {
                    let _ = item.queue_request.response_sender.send(Err(err));
                }
            }
        }

        // Start new requests if slots are available
        while self.web_ongoing_requests.len() < MAX_PARALLEL_REQUESTS {
            if let Some(queue_request) = self.web_pending_queue_requests.pop_front() {
                if !queue_request.request_option.is_some() {
                    continue;
                }

                match self.start_request(queue_request) {
                    Ok(()) => {}
                    Err(err) => {
                        godot_warn!("Failed to start request: {:?}", err);
                    }
                }
            } else {
                break; // No more pending requests
            }
        }
    }

    fn start_request(
        &mut self,
        mut queue_request: QueueRequest,
    ) -> Result<(), RequestResponseError> {
        let mut client = godot::engine::HttpClient::new_gd();
        let mut request_option = queue_request.request_option.take().unwrap();
        // Parse URL to get host and path
        let url = url::Url::parse(&request_option.url).map_err(|e| RequestResponseError {
            id: queue_request.id,
            error_message: format!("Invalid URL: {}", e),
        })?;

        // Connect to host
        let port = url
            .port()
            .unwrap_or(if url.scheme() == "https" { 443 } else { 80 });
        let host = url.host_str().ok_or_else(|| RequestResponseError {
            id: queue_request.id,
            error_message: "No host in URL".to_string(),
        })?;

        let connect_err = {
            let result = client.call(
                "connect_to_host".into(),
                &[
                    host.to_variant(),
                    port.to_variant(),
                    self.tls_options.clone().unwrap().to_variant(),
                ],
            );
            result.to::<godot::engine::global::Error>()
        };
        if connect_err != godot::engine::global::Error::OK {
            return Err(RequestResponseError {
                id: queue_request.id,
                error_message: format!("Connection error: {:?}", connect_err),
            });
        }

        if client.get_status() == godot::engine::http_client::Status::RESOLVING {
            client.poll();
        }

        if client.get_status() == godot::engine::http_client::Status::CONNECTING {
            client.poll();
        }

        // Convert headers to PackedStringArray
        let headers = request_option
            .headers
            .as_ref()
            .and_then(|headers| {
                Some(
                    headers
                        .iter()
                        .map(|(k, v)| format!("{}: {}", k, v))
                        .collect::<Vec<String>>(),
                )
            })
            .unwrap_or_default();
        let headers = headers
            .iter()
            .map(|s| s.into())
            .collect::<PackedStringArray>();

        // Start request
        let method = match request_option.method.as_str() {
            "GET" => godot::engine::http_client::Method::GET,
            "POST" => godot::engine::http_client::Method::POST,
            "PUT" => godot::engine::http_client::Method::PUT,
            "DELETE" => godot::engine::http_client::Method::DELETE,
            _ => godot::engine::http_client::Method::GET,
        };

        // TODO: implement send body
        let path = url.path();
        let request_err = if let Some(body) = request_option.body.take() {
            let bytes = PackedByteArray::from_vec(body.as_slice());
            client.request_raw(method, path.into(), headers, bytes)
        } else {
            client.request(method, path.into(), headers)
        };

        if request_err != godot::engine::global::Error::OK {
            return Err(RequestResponseError {
                id: queue_request.id,
                error_message: format!("Request error: {:?}", request_err),
            });
        }

        queue_request.request_option = Some(request_option);
        // Initialize file handle if needed
        let file_handle = if let Some(request_option) = &queue_request.request_option {
            if let ResponseType::ToFile(path) = &request_option.response_type {
                let fs = godot::engine::FileAccess::open(path.as_str().into(), ModeFlags::WRITE);
                if fs.is_none() {
                    return Err(RequestResponseError {
                        id: queue_request.id,
                        error_message: "Failed to open file for writing".to_string(),
                    });
                }
                fs
            } else {
                None
            }
        } else {
            None
        };

        // Store ongoing request
        self.web_ongoing_requests.insert(
            queue_request.id,
            OngoingRequest {
                client,
                queue_request,
                body_ready: false,
                file_handle,
                accumulated_bytes: Vec::new(),
            },
        );

        Ok(())
    }

    fn read_response(
        &self,
        mut client: Gd<godot::engine::HttpClient>,
        queue_request: &mut QueueRequest,
        accumulated_bytes: Vec<u8>,
    ) -> Result<RequestResponse, RequestResponseError> {
        let response_code = client.get_response_code();
        let headers = client.get_response_headers_as_dictionary();
        let headers = headers
            .iter_shared()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<HashMap<String, String>>();
        let status_code = http::StatusCode::from_u16(response_code as u16)
            .unwrap_or(http::StatusCode::INTERNAL_SERVER_ERROR);
        let request_option = queue_request.request_option.take().unwrap();

        tracing::info!("Finished request - response: {:?} bytes", accumulated_bytes.len());

        let response_data = match &request_option.response_type {
            ResponseType::AsString => {
                ResponseEnum::String(String::from_utf8(accumulated_bytes.clone()).unwrap())
            }
            ResponseType::AsBytes => {
                ResponseEnum::Bytes(accumulated_bytes.clone())
            }
            ResponseType::AsJson => {
                let text = String::from_utf8_lossy(&accumulated_bytes).to_string();
                let json_result = serde_json::from_str(&text);
                ResponseEnum::Json(json_result)
            }
            ResponseType::ToFile(path) => {
                ResponseEnum::ToFile(Ok(path.clone()))
            }
        };

        Ok(RequestResponse {
            headers: Some(headers),
            status_code,
            request_option,
            response_data: Ok(response_data),
        })
    }
}
