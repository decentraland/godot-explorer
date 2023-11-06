use std::fmt::Debug;

use tokio::sync::mpsc::{Receiver, Sender};

use super::request_response::*;

pub struct HttpRequester {
    sender_to_thread: tokio::sync::mpsc::Sender<RequestOption>,
    receiver_from_thread: tokio::sync::mpsc::Receiver<Result<RequestResponse, RequestResponseError>>,
}

impl Debug for HttpRequester {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpRequester").finish()
    }
}

impl Default for HttpRequester {
    fn default() -> Self {
        Self::new(None)
    }
}

async fn request_pool(
    sender_to_parent: Sender<Result<RequestResponse, RequestResponseError>>,
    mut receiver_from_parent: Receiver<RequestOption>,
) {
    while let Some(request_option) = receiver_from_parent.recv().await {
        let sender = sender_to_parent.clone();
        // TODO: limit the concurrent requests
        tokio::spawn(async move {
            let client = reqwest::Client::new();
            let url = request_option.url.clone();
            let response = HttpRequester::do_request(&client, request_option).await;
            if response.is_err() {
                tracing::info!("Error in request: {url:?}");
            } else {
                // tracing::info!("Ok in request: {:?}", url);
            }
            match sender.send(response).await {
                Ok(_) => {
                    // tracing::info!("Ok sending reqsuest: {:?}", url);
                }
                Err(_) => {
                    panic!("Failed to send response");
                }
            }
        });
    }
}

impl HttpRequester {
    pub fn new(runtime: Option<tokio::runtime::Handle>) -> Self {
        let (sender_to_thread, receiver_from_parent) =
            tokio::sync::mpsc::channel::<RequestOption>(100);
        let (sender_to_parent, receiver_from_thread) =
            tokio::sync::mpsc::channel::<Result<RequestResponse, RequestResponseError>>(100);

        if let Some(rt) = runtime {
            rt.spawn(async move {
                request_pool(sender_to_parent, receiver_from_parent).await;
            });
        } else {
            std::thread::spawn(move || {
                let runtime = tokio::runtime::Runtime::new();
                if runtime.is_err() {
                    panic!("Failed to create runtime {:?}", runtime.err());
                }
                let runtime = runtime.unwrap();

                runtime.block_on(async move {
                    request_pool(sender_to_parent, receiver_from_parent).await;
                });
            });
        }

        Self {
            sender_to_thread,
            receiver_from_thread,
        }
    }

    pub fn send_request(&mut self, req: RequestOption) -> bool {
        self.sender_to_thread.try_send(req).is_ok()
    }

    pub fn poll(&mut self) -> Option<Result<RequestResponse, RequestResponseError>> {
        self.receiver_from_thread.try_recv().ok()
    }

    pub async fn do_request(
        client: &reqwest::Client,
        mut request_option: RequestOption,
    ) -> Result<RequestResponse, RequestResponseError> {
        let mut request = client
            .request(request_option.method.clone(), request_option.url.clone())
            .timeout(std::time::Duration::from_secs(10));

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

        let map_err_func = |e: reqwest::Error| RequestResponseError { id: request_option.id, error_message: e.to_string() };

        let response = request.send().await.map_err(map_err_func)?;
        let status_code = response.status();

        let response_data = match request_option.response_type.clone() {
            ResponseType::AsString => {
                ResponseEnum::String(response.text().await.map_err(map_err_func)?)
            }
            ResponseType::AsBytes => {
                ResponseEnum::Bytes(response.bytes().await.map_err(map_err_func)?.to_vec())
            }
            ResponseType::ToFile(file_path) => {
                let content = response.bytes().await.map_err(map_err_func)?.to_vec();
                let mut file = std::fs::File::create(file_path).map_err(|e| RequestResponseError { id: request_option.id, error_message: e.to_string() })?;
                let result = std::io::Write::write_all(&mut file, &content);
                ResponseEnum::ToFile(result)
            }
            ResponseType::AsJson => {
                let json_string = &response.text().await.map_err(map_err_func)?;
                ResponseEnum::Json(serde_json::from_str(json_string))
            }
        };

        Ok(RequestResponse {
            request_option,
            status_code,
            response_data: Ok(response_data),
        })
    }
}

#[test]
fn test() {
    // TODO: add tests

    let mut requester = HttpRequester::new(None);

    // requester.send_request(RequestOption::new(
    //     0,
    //     "https://sdk-test-scenes.decentraland.zone/aboudt".to_string(),
    //     http::Method::GET,
    //     ResponseType::AsString,
    //     None,
    //     None,
    // ));

    // requester.send_request(RequestOption::new(
    //     0,
    //     "https://sdk-test-scenes.decentraland.zone/about".to_string(),
    //     http::Method::GET,
    //     ResponseType::AsString,
    //     None,
    //     None,
    // ));

    // requester.send_request(RequestOption::new(
    //     0,
    //     "https://sdk-test-scenes.decentraland.zone/aboudt".to_string(),
    //     http::Method::GET,
    //     ResponseType::AsBytes,
    //     None,
    //     None,
    // ));

    // requester.send_request(RequestOption::new(
    //     0,
    //     "https://sdk-test-scenes.decentraland.zone/about".to_string(),
    //     http::Method::GET,
    //     ResponseType::AsBytes,
    //     None,
    //     None,
    // ));

    // requester.send_request(RequestOption::new(
    //     0,
    //     "https://sdk-test-scenes.decentraland.zone/aboudt".to_string(),
    //     http::Method::GET,
    //     ResponseType::ToFile("test.txt".to_string()),
    //     None,
    //     None,
    // ));

    // requester.send_request(RequestOption::new(
    //     0,
    //     "https://sdk-test-scenes.decentraland.zone/about".to_string(),
    //     http::Method::GET,
    //     ResponseType::ToFile("test.txt".to_string()),
    //     None,
    //     None,
    // ));

    requester.send_request(RequestOption::new(
        0,
        "https://sdk-test-scenes.decentraland.zone/content/entities/active".to_string(),
        http::Method::POST,
        ResponseType::AsString,
        Some("{\"pointers\":[\"0,0\"]}".as_bytes().to_vec()),
        Some(vec!["Content-Type: application/json".to_string()]),
    ));

    let mut counter = 0;

    loop {
        match requester.poll() {
            Some(response) => {
                tracing::info!("{:?}", response);
                counter += 1;
            }
            None => {
                // Sleep for a while before polling again.
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
        if counter >= 1 {
            break;
        }
    }
}
