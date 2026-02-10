use crate::dcl::{
    components::{SceneComponentId, SceneCrdtTimestamp, SceneEntityId},
    crdt::SceneCrdtState,
    serialization::{
        reader::{DclReader, DclReaderError},
        writer::DclWriter,
    },
};

use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::FromPrimitive;

#[derive(FromPrimitive, ToPrimitive, Debug)]
pub enum CrdtMessageType {
    PutComponent = 1,
    DeleteComponent = 2,

    DeleteEntity = 3,
    AppendValue = 4,
}

const CRDT_HEADER_SIZE: usize = 8;

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
            let Some(component_definition) =
                scene_crdt_state.get_lww_component_definition_mut(component)
            else {
                tracing::warn!(
                    "CRDT: unknown LWW component {} for entity {:?} (PutComponent skipped)",
                    component.0,
                    entity
                );
                return Ok(());
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
            let Some(component_definition) =
                scene_crdt_state.get_lww_component_definition_mut(component)
            else {
                tracing::warn!(
                    "CRDT: unknown LWW component {} for entity {:?} (DeleteComponent skipped)",
                    component.0,
                    entity
                );
                return Ok(());
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

            let timestamp: SceneCrdtTimestamp = stream.read()?;
            let _content_len = stream.read_u32()? as usize;

            if !scene_crdt_state.entities.try_init(entity) {
                return Ok(());
            }
            let Some(component_definition) =
                scene_crdt_state.get_gos_component_definition_mut(component)
            else {
                tracing::warn!(
                    "CRDT: unknown GOS component {} for entity {:?} (AppendValue skipped)",
                    component.0,
                    entity
                );
                return Ok(());
            };

            component_definition.append_from_binary(entity, timestamp, stream);
        }
    }

    Ok(())
}

pub fn process_many_messages(stream: &mut DclReader, scene_crdt_state: &mut SceneCrdtState) {
    // collect commands
    while stream.len() > CRDT_HEADER_SIZE {
        let length = stream.read_u32().unwrap() as usize;
        let crdt_type = stream.read_u32().unwrap();
        let mut message_stream = stream.take_reader(length.saturating_sub(8));

        match FromPrimitive::from_u32(crdt_type) {
            Some(crdt_type) => {
                if let Err(e) = process_message(scene_crdt_state, crdt_type, &mut message_stream) {
                    tracing::warn!("CRDT Buffer error: {:?}", e);
                };
            }
            None => tracing::warn!("CRDT Header error: unhandled crdt message type {crdt_type}"),
        }
    }
}

const CRDT_DELETE_ENTITY_HEADER_SIZE: usize = CRDT_HEADER_SIZE + 4;
const CRDT_PUT_COMPONENT_HEADER_SIZE: usize = CRDT_HEADER_SIZE + 20;
const CRDT_DELETE_COMPONENT_HEADER_SIZE: usize = CRDT_HEADER_SIZE + 16;

pub fn put_or_delete_lww_component(
    scene_crdt_state: &SceneCrdtState,
    entity_id: &SceneEntityId,
    component_id: &SceneComponentId,
    writer: &mut DclWriter,
) -> Result<(), String> {
    let Some(component_definition) = scene_crdt_state.get_lww_component_definition(*component_id)
    else {
        return Err("Component not found".into());
    };
    let Some(opaque_value) = component_definition.get_opaque(*entity_id) else {
        return Err("Entity not found".into());
    };

    if opaque_value.value.is_some() {
        // TODO: this can be improved by using the same writer, we don't know the component_data_length in advance to write the right length
        //  but if we have the position written we can overwrite then
        let mut component_buf = Vec::new();
        let mut component_writer = DclWriter::new(&mut component_buf);
        component_definition.to_binary(*entity_id, &mut component_writer)?;

        let content_length = component_buf.len();
        let length = CRDT_DELETE_COMPONENT_HEADER_SIZE + component_buf.len();

        writer.write_u32(length as u32);
        writer.write(&CrdtMessageType::PutComponent);
        writer.write(entity_id);
        writer.write(component_id);
        writer.write(&opaque_value.timestamp);

        writer.write_u32(content_length as u32);
        writer.write_raw(&component_buf)
    } else {
        writer.write_u32(CRDT_PUT_COMPONENT_HEADER_SIZE as u32);
        writer.write(&CrdtMessageType::DeleteComponent);
        writer.write(entity_id);
        writer.write(component_id);
        writer.write(&opaque_value.timestamp);
    }

    Ok(())
}

pub fn append_gos_component(
    scene_crdt_state: &SceneCrdtState,
    entity_id: &SceneEntityId,
    component_id: &SceneComponentId,
    elements_count: &usize,
    writer: &mut DclWriter,
) -> Result<(), String> {
    let Some(component_definition) = scene_crdt_state.get_gos_component_definition(*component_id)
    else {
        return Err("Component not found".into());
    };

    for i in 0..*elements_count {
        // TODO: this can be improved by using the same writer, we don't know the component_data_length in advance to write the right length
        //  but if we have the position written we can overwrite then
        let mut component_buf = Vec::new();
        let mut component_writer = DclWriter::new(&mut component_buf);
        component_definition.to_binary(*entity_id, i, &mut component_writer)?;

        let content_length = component_buf.len();
        let length = CRDT_DELETE_COMPONENT_HEADER_SIZE + component_buf.len();

        writer.write_u32(length as u32);
        writer.write(&CrdtMessageType::AppendValue);
        writer.write(entity_id);
        writer.write(component_id);
        writer.write(&SceneCrdtTimestamp(0));

        writer.write_u32(content_length as u32);
        writer.write_raw(&component_buf)
    }

    Ok(())
}

pub fn delete_entity(entity_id: &SceneEntityId, writer: &mut DclWriter) {
    writer.write_u32(CRDT_DELETE_ENTITY_HEADER_SIZE as u32);
    writer.write(&CrdtMessageType::DeleteEntity);
    writer.write(entity_id);
}

/// Filters raw CRDT bytes, preserving only messages for components known to
/// `scene_crdt_state`. Unknown component messages are dropped at the byte level,
/// which avoids prost re-serialization that can strip unknown proto fields.
pub fn filter_known_crdt_messages(raw: &[u8], scene_crdt_state: &SceneCrdtState) -> Vec<u8> {
    let mut out = Vec::with_capacity(raw.len());
    let mut pos = 0;

    while pos + CRDT_HEADER_SIZE <= raw.len() {
        // Read message length (4 bytes LE) – includes the 8-byte header
        let msg_len = u32::from_le_bytes(raw[pos..pos + 4].try_into().unwrap()) as usize;
        let crdt_type = u32::from_le_bytes(raw[pos + 4..pos + 8].try_into().unwrap());

        // Total bytes this message occupies (length field itself + body)
        let total = 4 + msg_len.saturating_sub(4);
        // Guard: make sure we don't read past the buffer
        if pos + 4 + msg_len.saturating_sub(4) > raw.len() {
            tracing::warn!(
                "CRDT filter: truncated message at pos={}, msg_len={}, remaining={}",
                pos,
                msg_len,
                raw.len() - pos
            );
            break;
        }

        let keep = match FromPrimitive::from_u32(crdt_type) {
            Some(CrdtMessageType::DeleteEntity) => true,
            Some(CrdtMessageType::PutComponent) | Some(CrdtMessageType::DeleteComponent) => {
                // entity(4) + component_id(4) start at offset 8 inside the message
                if msg_len >= 16 {
                    let comp_id_offset = pos + 12; // 4(len) + 4(type) + 4(entity)
                    let comp_id = u32::from_le_bytes(
                        raw[comp_id_offset..comp_id_offset + 4].try_into().unwrap(),
                    );
                    let cid = SceneComponentId(comp_id);
                    scene_crdt_state.get_lww_component_definition(cid).is_some()
                } else {
                    true // malformed but let the normal parser handle it
                }
            }
            Some(CrdtMessageType::AppendValue) => {
                if msg_len >= 16 {
                    let comp_id_offset = pos + 12;
                    let comp_id = u32::from_le_bytes(
                        raw[comp_id_offset..comp_id_offset + 4].try_into().unwrap(),
                    );
                    let cid = SceneComponentId(comp_id);
                    scene_crdt_state.get_gos_component_definition(cid).is_some()
                } else {
                    true
                }
            }
            None => {
                tracing::warn!(
                    "CRDT filter: unknown message type {} at pos={}",
                    crdt_type,
                    pos
                );
                false
            }
        };

        if keep {
            out.extend_from_slice(&raw[pos..pos + total]);
        }

        pos += total;
    }

    out
}
