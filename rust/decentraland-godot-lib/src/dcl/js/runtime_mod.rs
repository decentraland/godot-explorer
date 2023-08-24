use crate::{
    dcl::{
        crdt::message::{append_gos_component, process_many_messages, put_or_delete_lww_component},
        serialization::{reader::DclReader, writer::DclWriter},
        RendererResponse, SceneResponse,
    },
    http_request::{http_requester::HttpRequester, request_response::RequestOption},
};

use super::js_runtime::JsRuntime;

pub fn op_read_file(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut ret: v8::ReturnValue,
) {
    let state = JsRuntime::state_from(scope);
    let mut state = state.borrow();
    let http_requester = HttpRequester::new();
    let file = args.get(0).to_rust_string_lossy(scope);
    let file = state.content_mapping.get(&file);

    if let Some(file) = file {
        
    }
    // http_requester.send_request(RequestOption::new(
    //     0,
    //     file,
    //     reqwest::Method::GET,
    //     ResponseType::AsBytes,
    //     None,
    //     None,
    // ));

    let receiver = &mut state.thread_receive_from_main;
    let response = receiver.blocking_recv();

    let mutex_scene_crdt_state = &mut state.crdt;
    let cloned_scene_crdt = mutex_scene_crdt_state.clone();
    let scene_crdt_state = cloned_scene_crdt.lock().unwrap();

    let data = match response {
        Some(RendererResponse::Ok(data)) => {
            let (_dirty_entities, dirty_lww_components, dirty_gos_components) = data;

            let mut data_buf = Vec::new();
            let mut data_writter = DclWriter::new(&mut data_buf);

            for (component_id, entities) in dirty_lww_components {
                for entity_id in entities {
                    if let Err(err) = put_or_delete_lww_component(
                        &scene_crdt_state,
                        &entity_id,
                        &component_id,
                        &mut data_writter,
                    ) {
                        println!("error writing crdt message: {err}");
                    }
                }
            }

            for (component_id, entities) in dirty_gos_components {
                for (entity_id, element_count) in entities {
                    if let Err(err) = append_gos_component(
                        &scene_crdt_state,
                        &entity_id,
                        &component_id,
                        element_count,
                        &mut data_writter,
                    ) {
                        println!("error writing crdt message: {err}");
                    }
                }
            }

            data_buf
        }
        _ => {
            // channel has been closed, shutdown gracefully
            println!("{}: shutting down", std::thread::current().name().unwrap());

            // TODO: handle recv from renderer
            state.dying = true;

            Default::default()
        }
    };
    drop(scene_crdt_state);
    drop(cloned_scene_crdt);

    let arr_bytes = if state.main_crdt.is_some() {
        let main_crdt_data = state.main_crdt.take().unwrap();
        vec![main_crdt_data, data]
    } else {
        vec![data]
    };
    // TODO: main.crdt

    let arr = v8::Array::new(scope, arr_bytes.len() as i32);
    for (index, arr_u8) in arr_bytes.into_iter().enumerate() {
        let uint8_array = slice_to_uint8array(scope, &arr_u8);
        arr.set_index(scope, index as u32, uint8_array.into());
    }

    ret.set(arr.into());
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
