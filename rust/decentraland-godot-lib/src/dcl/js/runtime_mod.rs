use std::time::{Duration, Instant};

use crate::http_request::{
    http_requester::HttpRequester,
    request_response::{RequestOption, ResponseEnum, ResponseType},
};

use super::js_runtime::JsRuntime;

pub fn op_read_file(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut ret: v8::ReturnValue,
) {
    let state = JsRuntime::state_from(scope);
    let state = state.borrow();
    let mut http_requester = HttpRequester::new();
    let file = args.get(0).to_rust_string_lossy(scope);
    let file = state.content_mapping.get(&file);

    if let Some(file_hash) = file {
        let url = format!("{}{file_hash}", state.base_url);
        http_requester.send_request(RequestOption::new(
            0,
            url,
            reqwest::Method::GET,
            ResponseType::AsBytes,
            None,
            None,
        ));

        // wait until the request is done or timeout
        let start_time = Instant::now();
        loop {
            if let Some(response) = http_requester.poll() {
                if let Ok(response) = response {
                    if let Ok(response_data) = response.response_data {
                        match response_data {
                            ResponseEnum::Bytes(bytes) => {
                                let arr = slice_to_uint8array(scope, &bytes);
                                ret.set(arr.into());
                            }
                            _ => {}
                        }
                    }
                }
                break;
            } else {
                std::thread::sleep(Duration::from_millis(10));
            }

            if start_time.elapsed() > Duration::from_secs(10) {
                break;
            }
        }
    }
}

pub fn slice_to_uint8array<'a>(
    scope: &mut v8::HandleScope<'a>,
    buf: &[u8],
) -> v8::Local<'a, v8::Uint8Array> {
    let buffer = if buf.is_empty() {
        v8::ArrayBuffer::new(scope, 0)
    } else {
        let store: v8::UniqueRef<_> = v8::ArrayBuffer::new_backing_store(scope, buf.len());
        // SAFETY: raw memory copy into the v8 ArrayBuffer allocated above
        unsafe {
            std::ptr::copy_nonoverlapping(
                buf.as_ptr(),
                store.data().unwrap().as_ptr() as *mut u8,
                buf.len(),
            )
        }
        v8::ArrayBuffer::with_backing_store(scope, &store.make_shared())
    };
    v8::Uint8Array::new(scope, buffer, 0, buf.len()).expect("Failed to create UintArray8")
}
