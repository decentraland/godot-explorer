use godot::prelude::*;

use crate::scene_runner::tokio_runtime::TokioRuntime;

// Deriving GodotClass makes the class available to Godot
#[derive(GodotClass)]
#[class(base=Node)]
pub struct RustHttpRequester {
    http_requester: super::http_requester::HttpRequester,
}

#[godot_api]
impl RustHttpRequester {
    #[func]
    fn poll(&mut self) -> Variant {
        match self.http_requester.poll() {
            Some(response) => {
                match response {
                    Ok(response) => {
                        // tracing::info!(
                        //     "response {:?} ok? {:?}",
                        //     response.request_option.url.clone(),
                        //     !response.is_error()
                        // );
                        Variant::from(Gd::new(response))
                    }
                    Err(error) => {
                        tracing::info!("error polling http_requester id={} msg={}", error.id, error.error_message);

                        Variant::from(Gd::new(error))
                    }
                }
            }
            _ => Variant::nil(),
        }
    }

    #[func]
    fn request_file(
        &mut self,
        reference_id: u32,
        url: GodotString,
        absolute_path: GodotString,
    ) -> u32 {
        // tracing::info!(
        //     "Requesting file: {:?} in {absolute_path}  ",
        //     url.to_string()
        // );

        let request_option = crate::http_request::request_response::RequestOption::new(
            reference_id,
            url.to_string(),
            http::Method::GET,
            crate::http_request::request_response::ResponseType::ToFile(absolute_path.to_string()),
            None,
            None,
        );
        let id = request_option.id;
        self.http_requester.send_request(request_option);
        id
    }

    #[func]
    fn request_json(
        &mut self,
        reference_id: u32,
        url: GodotString,
        method: godot::engine::http_client::Method,
        body: GodotString,
        headers: VariantArray,
    ) -> u32 {
        tracing::info!("Requesting json: {:?}", url.to_string());

        let method = match method {
            godot::engine::http_client::Method::METHOD_POST => http::Method::POST,
            _ => http::Method::GET,
        };

        let body = match body.to_string().as_str() {
            "" => None,
            _ => Some(body.to_string().into_bytes()),
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
            reference_id,
            url.to_string(),
            method,
            crate::http_request::request_response::ResponseType::AsString,
            body,
            headers,
        );
        let id = request_option.id;
        self.http_requester.send_request(request_option);
        id
    }
}

#[godot_api]
impl NodeVirtual for RustHttpRequester {
    fn init(base: Base<Node>) -> Self {
        RustHttpRequester {
            http_requester: super::http_requester::HttpRequester::new(
                TokioRuntime::static_clone_handle(),
            ),
        }
    }
}
