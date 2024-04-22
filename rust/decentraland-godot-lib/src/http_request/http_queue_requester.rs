use reqwest::Client;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::sync::{Arc, Mutex};
use tokio::io::AsyncWriteExt;
use tokio::sync::{oneshot, Semaphore};

use super::request_response::{
    RequestOption, RequestResponse, RequestResponseError, ResponseEnum, ResponseType,
};

#[derive(Debug)]
struct QueueRequest {
    id: u32,
    priority: usize,
    request_option: Option<RequestOption>,
    response_sender: oneshot::Sender<Result<RequestResponse, RequestResponseError>>,
}

impl PartialEq for QueueRequest {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for QueueRequest {
    fn assert_receiver_is_total_eq(&self) {}
}

impl Ord for QueueRequest {
    fn cmp(&self, other: &Self) -> Ordering {
        other.priority.cmp(&self.priority)
    }
}

impl PartialOrd for QueueRequest {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug)]
pub struct HttpQueueRequester {
    client: Arc<Client>,
    queue: Arc<Mutex<BinaryHeap<QueueRequest>>>,
    semaphore: Arc<Semaphore>,
}

async fn process_queue_request(
    queue: Arc<Mutex<BinaryHeap<QueueRequest>>>,
    semaphore: Arc<Semaphore>,
    client: Arc<Client>,
) {
    let _permit = semaphore.acquire_owned().await;
    let request = {
        let mut queue = queue.lock().unwrap();
        queue.pop()
    };

    if let Some(mut queue_request) = request {
        let request_option = queue_request.request_option.take().unwrap();
        let response_result = process_request(client, request_option).await;
        let _ = queue_request.response_sender.send(response_result);
    }
}

async fn process_request(
    client: Arc<Client>,
    mut request_option: RequestOption,
) -> Result<RequestResponse, RequestResponseError> {
    let timeout = request_option
        .timeout
        .unwrap_or(std::time::Duration::from_secs(60));
    let request = client.request(request_option.method.clone(), request_option.url.clone());

    #[cfg(not(target_arch = "wasm32"))]
    let mut request = request.timeout(timeout);

    if let Some(body) = request_option.body.take() {
        request = request.body(body);
    }

    if let Some(headers) = request_option.headers.take() {
        for header in headers {
            let parts: Vec<&str> = header.splitn(2, ':').collect();
            if parts.len() == 2 {
                request = request.header(parts[0], parts[1].trim());
            }
        }
    }

    let map_err_func = |e: reqwest::Error| RequestResponseError {
        id: request_option.id,
        error_message: e.to_string(),
    };

    let response = request.send().await.map_err(map_err_func)?;
    let status_code = response.status();

    let response_data = match request_option.response_type.clone() {
        ResponseType::AsString => {
            ResponseEnum::String(response.text().await.map_err(map_err_func)?)
        }
        ResponseType::AsBytes => {
            ResponseEnum::Bytes(response.bytes().await.map_err(map_err_func)?.to_vec())
        }
        #[cfg(not(target_arch = "wasm32"))]
        ResponseType::ToFile(file_path) => {
            let content = response.bytes().await.map_err(map_err_func)?.to_vec();
            let mut file = tokio::fs::File::create(file_path.clone())
                .await
                .map_err(|e| RequestResponseError {
                    id: request_option.id,
                    error_message: e.to_string(),
                })?;
            let result = file.write_all(&content).await;
            let result = result.map(|_| file_path);
            ResponseEnum::ToFile(result)
        }
        ResponseType::AsJson => {
            let json_string = &response.text().await.map_err(map_err_func)?;
            ResponseEnum::Json(serde_json::from_str(json_string))
        }
        #[cfg(target_arch = "wasm32")]
        _ => {
            return Err(RequestResponseError {
                id: request_option.id,
                error_message: "Response type not supported".to_string(),
            });
        }
    };

    Ok(RequestResponse {
        request_option,
        status_code,
        response_data: Ok(response_data),
    })
}

impl HttpQueueRequester {
    pub fn new(max_parallel_requests: usize) -> Self {
        Self {
            client: Arc::new(Client::new()),
            queue: Arc::new(Mutex::new(BinaryHeap::new())),
            semaphore: Arc::new(Semaphore::new(max_parallel_requests)),
        }
    }
    pub async fn request(
        &self,
        request_option: RequestOption,
        priority: usize,
    ) -> Result<RequestResponse, RequestResponseError> {
        let (response_sender, response_receiver) = oneshot::channel();
        let http_request = QueueRequest {
            id: request_option.id,
            priority,
            request_option: Some(request_option),
            response_sender,
        };
        self.queue.lock().unwrap().push(http_request);
        self.process_queue().await;
        response_receiver.await.unwrap()
    }

    async fn process_queue(&self) {
        let queue: Arc<Mutex<BinaryHeap<QueueRequest>>> = Arc::clone(&self.queue);
        let semaphore: Arc<Semaphore> = Arc::clone(&self.semaphore);
        let client: Arc<Client> = self.client.clone();

        #[cfg(not(target_arch = "wasm32"))]
        process_queue_request(queue, semaphore, client).await;

        #[cfg(target_arch = "wasm32")]
        process_queue_request(queue, semaphore, client);
    }
}
