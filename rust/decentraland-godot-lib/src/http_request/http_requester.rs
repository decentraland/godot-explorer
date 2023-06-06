use super::request_response::*;

pub struct HttpRequester {
    sender_to_thread: tokio::sync::mpsc::Sender<RequestOption>,
    receiver_from_thread: tokio::sync::mpsc::Receiver<Result<RequestResponse, String>>,
}

impl HttpRequester {
    pub fn new() -> Self {
        let (sender_to_thread, mut receiver_from_parent) =
            tokio::sync::mpsc::channel::<RequestOption>(100);
        let (sender_to_parent, receiver_from_thread) =
            tokio::sync::mpsc::channel::<Result<RequestResponse, String>>(100);

        std::thread::spawn(move || {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            let client = reqwest::Client::new();

            runtime.block_on(async move {
                while let Some(request_option) = receiver_from_parent.recv().await {
                    let url = request_option.url.clone();
                    let response = Self::do_request(&client, request_option).await;
                    if response.is_err() {
                        println!("Error in request: {:?}", url);
                    } else {
                        // println!("Ok in request: {:?}", url);
                    }
                    match sender_to_parent.send(response).await {
                        Ok(_) => {
                            // println!("Ok sending reqsuest: {:?}", url);
                        }
                        Err(_) => {
                            panic!("Failed to send response");
                        }
                    }
                }
            });
        });

        Self {
            sender_to_thread,
            receiver_from_thread,
        }
    }

    pub fn send_request(&mut self, req: RequestOption) -> bool {
        self.sender_to_thread.try_send(req).is_ok()
    }

    pub fn poll(&mut self) -> Option<Result<RequestResponse, String>> {
        self.receiver_from_thread.try_recv().ok()
    }

    async fn do_request(
        client: &reqwest::Client,
        mut request_option: RequestOption,
    ) -> Result<RequestResponse, String> {
        let mut request = client.request(request_option.method.clone(), request_option.url.clone());

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

        let response = request.send().await.map_err(|e| e.to_string())?;
        let status_code = response.status();

        let response_data = match request_option.response_type.clone() {
            ResponseType::AsString => {
                ResponseEnum::String(response.text().await.map_err(|e| e.to_string())?)
            }
            ResponseType::AsBytes => {
                ResponseEnum::Bytes(response.bytes().await.map_err(|e| e.to_string())?.to_vec())
            }
            ResponseType::ToFile(file_path) => {
                let content = response.bytes().await.map_err(|e| e.to_string())?.to_vec();
                let mut file = std::fs::File::create(file_path).map_err(|e| e.to_string())?;
                let result = std::io::Write::write_all(&mut file, &content);
                ResponseEnum::ToFile(result)
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

    let mut requester = HttpRequester::new();

    // requester.send_request(RequestOption::new(
    //     0,
    //     "https://sdk-test-scenes.decentraland.zone/aboudt".to_string(),
    //     reqwest::Method::GET,
    //     ResponseType::AsString,
    //     None,
    //     None,
    // ));

    // requester.send_request(RequestOption::new(
    //     0,
    //     "https://sdk-test-scenes.decentraland.zone/about".to_string(),
    //     reqwest::Method::GET,
    //     ResponseType::AsString,
    //     None,
    //     None,
    // ));

    // requester.send_request(RequestOption::new(
    //     0,
    //     "https://sdk-test-scenes.decentraland.zone/aboudt".to_string(),
    //     reqwest::Method::GET,
    //     ResponseType::AsBytes,
    //     None,
    //     None,
    // ));

    // requester.send_request(RequestOption::new(
    //     0,
    //     "https://sdk-test-scenes.decentraland.zone/about".to_string(),
    //     reqwest::Method::GET,
    //     ResponseType::AsBytes,
    //     None,
    //     None,
    // ));

    // requester.send_request(RequestOption::new(
    //     0,
    //     "https://sdk-test-scenes.decentraland.zone/aboudt".to_string(),
    //     reqwest::Method::GET,
    //     ResponseType::ToFile("test.txt".to_string()),
    //     None,
    //     None,
    // ));

    // requester.send_request(RequestOption::new(
    //     0,
    //     "https://sdk-test-scenes.decentraland.zone/about".to_string(),
    //     reqwest::Method::GET,
    //     ResponseType::ToFile("test.txt".to_string()),
    //     None,
    //     None,
    // ));

    requester.send_request(RequestOption::new(
        0,
        "https://sdk-test-scenes.decentraland.zone/content/entities/active".to_string(),
        reqwest::Method::POST,
        ResponseType::AsString,
        Some("{\"pointers\":[\"0,0\"]}".as_bytes().to_vec()),
        Some(vec!["Content-Type: application/json".to_string()]),
    ));

    let mut counter = 0;

    loop {
        match requester.poll() {
            Some(response) => {
                println!("{:?}", response);
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
