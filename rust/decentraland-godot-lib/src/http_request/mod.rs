struct RequestOption {
    url: String,
}

pub struct HttpRequester {
    sender_to_thread: tokio::sync::mpsc::Sender<RequestOption>,
    receiver_from_thread: tokio::sync::mpsc::Receiver<String>,
}

impl HttpRequester {
    pub fn new() -> Self {
        let (sender_to_thread, mut receiver_from_parent) =
            tokio::sync::mpsc::channel::<RequestOption>(1);
        let (sender_to_parent, receiver_from_thread) = tokio::sync::mpsc::channel::<String>(1);

        std::thread::spawn(move || {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async move {
                while let Some(request_option) = receiver_from_parent.recv().await {
                    println!("Received: {}", request_option.url.to_string());
                    let response = format!("Response to: {}", request_option.url.to_string());
                    sender_to_parent.send(response);
                    // println!("request_option.url: {}", request_option.url);

                    // let url: hyper::Uri = "https://sdk-test-scenes.decentraland.zone/about"
                    //     .parse()
                    //     .unwrap();
                    // let host = url.host().expect("uri has no host");
                    // let port = url.port_u16().unwrap_or(80);
                    // let addr = format!("{}:{}", host, port);

                    // let stream = tokio::net::TcpStream::connect(addr).await.unwrap();

                    // let (mut sender, conn) =
                    //     hyper::client::conn::http1::handshake(stream).await.unwrap();

                    // tokio::task::spawn(async move {
                    //     if let Err(err) = conn.await {
                    //         println!("Connection failed: {:?}", err);
                    //     }
                    // });

                    // let authority = url.authority().unwrap().clone();
                    // let req = hyper::Request::builder()
                    //     .uri(url)
                    //     .header(hyper::header::HOST, authority.as_str())
                    //     .body(http_body_util::Empty::<hyper::body::Bytes>::new())
                    //     .unwrap();

                    // let res = sender.send_request(req).await.unwrap();

                    // asynchronously aggregate the chunks of the body
                    // let body = res.collect().await.unwrap().aggregate();

                    // while let Ok((uri, res_sender)) = receiver.recv().await {
                    // let res = client
                    //     .get(
                    //         "https://sdk-test-scenes.decentraland.zone/about"
                    //             .parse()
                    //             .unwrap(),
                    //     )
                    //     .await;
                    // let body = hyper::body::(res.unwrap()).await.unwrap();

                    // Send the response body back to the caller.
                    // res_sender
                    //     .send(String::from_utf8_lossy(&body).to_string())
                    //     .unwrap();
                    // }
                }
            });
        });

        Self {
            sender_to_thread,
            receiver_from_thread,
        }
    }

    pub fn send_request(&mut self, url: String) {
        self.sender_to_thread.send(RequestOption { url });
        // let (res_sender, res_receiver) = oneshot::channel::<String>();
        // self.sender.send((uri, res_sender)).unwrap();
        // self.receiver = Some(res_receiver);
    }

    pub fn poll(&mut self) -> Option<String> {
        self.receiver_from_thread.try_recv().ok()
        // match self.receiver.take() {
        //     Some(receiver) => match receiver.try_recv() {
        //         Ok(response) => Some(response),
        //         Err(_) => None,
        //     },
        //     None => None,
    }
}

#[test]
fn test() {
    let mut requester = HttpRequester::new();
    requester.send_request("https://sdk-test-scenes.decentraland.zone/about".to_string());

    loop {
        match requester.poll() {
            Some(response) => {
                println!("{}", response);
                break;
            }
            None => {
                // Sleep for a while before polling again.
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
    }
}
