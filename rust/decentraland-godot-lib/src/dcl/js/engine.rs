// Engine module

use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
};

use deno_core::{op, OpDecl, OpState};
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::FromPrimitive;

use crate::dcl::{
    components::{SceneComponentId, SceneCrdtTimestamp, SceneEntityId},
    crdt::SceneCrdtState,
    serialization::reader::{DclReader, DclReaderError},
    RendererResponse, SceneId, SceneResponse,
};

const CRDT_HEADER_SIZE: usize = 8;

#[derive(FromPrimitive, ToPrimitive, Debug)]
pub enum CrdtMessageType {
    PutComponent = 1,
    DeleteComponent = 2,

    DeleteEntity = 3,
    AppendValue = 4,
}

// list of op declarations
pub fn ops() -> Vec<OpDecl> {
    vec![
        op_crdt_send_to_renderer::decl(),
        op_crdt_recv_from_renderer::decl(),
    ]
}

// handles a single message from the buffer
fn process_message(
    scene_crdt_state: &mut SceneCrdtState,
    crdt_type: CrdtMessageType,
    stream: &mut DclReader,
) -> Result<(), DclReaderError> {
    match crdt_type {
        CrdtMessageType::PutComponent => {
            let entity = stream.read()?;
            let component: SceneComponentId = stream.read()?;
            let timestamp: SceneCrdtTimestamp = stream.read()?;
            let _content_len = stream.read_u32()? as usize;

            if !scene_crdt_state.entities.try_init(entity) {
                return Ok(());
            }

            let Some(component_definition) = scene_crdt_state.get_lww_component_definition(component) else {
                return Ok(())
            };

            component_definition.set_from_binary(entity, timestamp, stream);
        }
        CrdtMessageType::DeleteComponent => {
            let entity = stream.read()?;
            let component: SceneComponentId = stream.read()?;
            let timestamp: SceneCrdtTimestamp = stream.read()?;

            if !scene_crdt_state.entities.try_init(entity) {
                return Ok(());
            }

            // check for a writer
            let Some(component_definition) = scene_crdt_state.get_lww_component_definition(component) else {
                return Ok(())
            };

            component_definition.set_none(entity, timestamp);
        }
        CrdtMessageType::DeleteEntity => {
            let entity: SceneEntityId = stream.read()?;
            scene_crdt_state.entities.kill(entity);
        }
        CrdtMessageType::AppendValue => {
            let entity = stream.read()?;
            let component: SceneComponentId = stream.read()?;

            if !scene_crdt_state.entities.try_init(entity) {
                return Ok(());
            }

            // check for a writer
            let Some(component_definition) = scene_crdt_state.get_gos_component_definition(component) else {
                return Ok(())
            };

            component_definition.append_from_binary(entity, stream);
        }
    }

    Ok(())
}

// receive and process a buffer of crdt messages
#[op(v8)]
fn op_crdt_send_to_renderer(op_state: Rc<RefCell<OpState>>, messages: &[u8]) {
    let mut op_state = op_state.borrow_mut();

    let mutex_scene_crdt_state = op_state.take::<Arc<Mutex<SceneCrdtState>>>();
    let cloned_scene_crdt = mutex_scene_crdt_state.clone();
    let mut stream = DclReader::new(messages);

    let mut scene_crdt_state = cloned_scene_crdt.lock().unwrap();

    // collect commands
    while stream.len() > CRDT_HEADER_SIZE {
        let length = stream.read_u32().unwrap() as usize;
        let crdt_type = stream.read_u32().unwrap();
        let mut message_stream = stream.take_reader(length.saturating_sub(8));

        match FromPrimitive::from_u32(crdt_type) {
            Some(crdt_type) => {
                if let Err(e) =
                    process_message(&mut scene_crdt_state, crdt_type, &mut message_stream)
                {
                    println!("CRDT Buffer error: {:?}", e);
                };
            }
            None => println!("CRDT Header error: unhandled crdt message type {crdt_type}"),
        }
    }

    let dirty = scene_crdt_state.take_dirty();
    op_state.put(mutex_scene_crdt_state);

    let sender = op_state.borrow_mut::<std::sync::mpsc::SyncSender<SceneResponse>>();
    sender
        .send(SceneResponse::Ok(SceneId(0), dirty))
        .expect("error sending scene response!!")
}

// fn put_component(
//     entity_id: &SceneEntityId,
//     component_id: &SceneComponentId,
//     data: &LWWEntry,
// ) -> Vec<u8> {
//     let content_len = data.data.len();
//     let length = content_len + 12 + if data.is_some { 4 } else { 0 } + 8;

//     let mut buf = Vec::with_capacity(length);
//     let mut writer = DclWriter::new(&mut buf);
//     writer.write_u32(length as u32);

//     if data.is_some {
//         writer.write(&CrdtMessageType::PutComponent);
//     } else {
//         writer.write(&CrdtMessageType::DeleteComponent);
//     }

//     writer.write(entity_id);
//     writer.write(component_id);
//     writer.write(&data.timestamp);

//     if data.is_some {
//         writer.write_u32(content_len as u32);
//         writer.write_raw(&data.data)
//     }

//     buf
// }

#[op(v8)]
async fn op_crdt_recv_from_renderer(op_state: Rc<RefCell<OpState>>) -> Vec<Vec<u8>> {
    // println!("op_crdt_recv_from_renderer is called!");

    let mut receiver = op_state
        .borrow_mut()
        .take::<tokio::sync::mpsc::Receiver<RendererResponse>>();
    let _response = receiver.recv().await;
    op_state.borrow_mut().put(receiver);

    // let results = match response {
    //     Some(RendererResponse::Ok) => {
    //         let mut results = Vec::new();
    //         // // TODO: consider writing directly into a v8 buffer
    //         // for (component_id, lww) in updates.lww.iter() {
    //         //     for (entity_id, data) in lww.last_write.iter() {
    //         //         results.push(put_component(entity_id, component_id, data));
    //         //     }
    //         // }
    //         results
    //     }
    //     None => {
    //         // channel has been closed, shutdown gracefully
    //         println!("{}: shutting down", std::thread::current().name().unwrap());
    //         op_state.borrow_mut().put(ShuttingDown);
    //         Default::default()
    //     }
    // };

    // results
    vec![vec![0]]
}
