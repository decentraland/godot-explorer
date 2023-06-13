use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
};

use deno_core::{op, OpDecl, OpState};
use godot::prelude::godot_print;

use crate::dcl::{
    crdt::{
        message::{process_many_messages, put_or_delete_lww_component},
        SceneCrdtState,
    },
    js::{SceneMainCrdtFileContent, ShuttingDown},
    serialization::{reader::DclReader, writer::DclWriter},
    RendererResponse, SceneId, SceneResponse,
};

// list of op declarations
pub fn ops() -> Vec<OpDecl> {
    vec![
        op_crdt_send_to_renderer::decl(),
        op_crdt_recv_from_renderer::decl(),
    ]
}

// receive and process a buffer of crdt messages
#[op(v8)]
fn op_crdt_send_to_renderer(op_state: Rc<RefCell<OpState>>, messages: &[u8]) {
    let mut op_state = op_state.borrow_mut();

    let scene_id = op_state.take::<SceneId>();
    let mutex_scene_crdt_state = op_state.take::<Arc<Mutex<SceneCrdtState>>>();
    let cloned_scene_crdt = mutex_scene_crdt_state.clone();
    let mut stream = DclReader::new(messages);
    let mut scene_crdt_state = cloned_scene_crdt.lock().unwrap();

    process_many_messages(&mut stream, &mut scene_crdt_state);

    let dirty = scene_crdt_state.take_dirty();
    op_state.put(mutex_scene_crdt_state);
    op_state.put(scene_id);

    let sender = op_state.borrow_mut::<std::sync::mpsc::SyncSender<SceneResponse>>();
    sender
        .send(SceneResponse::Ok(scene_id, dirty))
        .expect("error sending scene response!!")
}

#[op(v8)]
async fn op_crdt_recv_from_renderer(op_state: Rc<RefCell<OpState>>) -> Vec<Vec<u8>> {
    let mut ret = Vec::<Vec<u8>>::with_capacity(1);
    if let Some(main_crdt) = op_state.borrow_mut().try_take::<SceneMainCrdtFileContent>() {
        let mut op_state_main_crdt = op_state.borrow_mut();
        let scene_id = op_state_main_crdt.take::<SceneId>();
        let mutex_scene_crdt_state = op_state_main_crdt.take::<Arc<Mutex<SceneCrdtState>>>();
        let cloned_scene_crdt = mutex_scene_crdt_state.clone();
        let mut stream = DclReader::new(&main_crdt.0);
        let mut scene_crdt_state = cloned_scene_crdt.lock().unwrap();

        process_many_messages(&mut stream, &mut scene_crdt_state);

        let dirty = scene_crdt_state.take_dirty();
        op_state_main_crdt.put(mutex_scene_crdt_state);
        op_state_main_crdt.put(scene_id);

        let sender = op_state_main_crdt.borrow_mut::<std::sync::mpsc::SyncSender<SceneResponse>>();
        sender
            .send(SceneResponse::Ok(scene_id, dirty))
            .expect("error sending scene response!!");

        ret.push(main_crdt.0);
    }

    let mut receiver = op_state
        .borrow_mut()
        .take::<tokio::sync::mpsc::Receiver<RendererResponse>>();
    let response = receiver.recv().await;

    let mut op_state = op_state.borrow_mut();
    op_state.put(receiver);

    let mutex_scene_crdt_state = op_state.take::<Arc<Mutex<SceneCrdtState>>>();
    let cloned_scene_crdt = mutex_scene_crdt_state.clone();
    let scene_crdt_state = cloned_scene_crdt.lock().unwrap();

    let data = match response {
        Some(RendererResponse::Ok(data)) => {
            let (_dirty_entities, dirty_components) = data;

            let mut data_buf = Vec::new();
            let mut data_writter = DclWriter::new(&mut data_buf);

            for (component_id, entities) in dirty_components {
                for entity_id in entities {
                    if let Err(err) = put_or_delete_lww_component(
                        &scene_crdt_state,
                        &entity_id,
                        &component_id,
                        &mut data_writter,
                    ) {
                        godot_print!("error writing crdt message: {}", err);
                    }
                }
            }

            data_buf
        }
        _ => {
            // channel has been closed, shutdown gracefully
            godot_print!("{}: shutting down", std::thread::current().name().unwrap());
            op_state.put(ShuttingDown);
            Default::default()
        }
    };

    op_state.put(mutex_scene_crdt_state);
    ret.push(data);
    ret
}
