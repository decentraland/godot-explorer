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

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
fn debug_check_component(component: SceneComponentId, entity: SceneEntityId, operation: &str) {
    // List of component IDs to debug with their names
    const DEBUG_COMPONENTS: &[(u32, &str)] = &[
        (1078, "PBInputModifier"),
        (1099, "PBGltfNodeModifiers"),
        // Add more components here as needed
    ];

    if let Some((_, name)) = DEBUG_COMPONENTS.iter().find(|(id, _)| *id == component.0) {
        tracing::warn!(
            "{} ({}) detected in {} for entity {:?}",
            name,
            component.0,
            operation,
            entity
        );
    }
}

fn process_message(
    scene_crdt_state: &mut SceneCrdtState,
    crdt_type: CrdtMessageType,
    stream: &mut DclReader,
) -> Result<(), DclReaderError> {
    match crdt_type {
        CrdtMessageType::PutComponent => {
            let entity = stream.read()?;
            let component: SceneComponentId = stream.read()?;

            #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
            debug_check_component(component, entity, "PutComponent");

            let timestamp: SceneCrdtTimestamp = stream.read()?;
            let _content_len = stream.read_u32()? as usize;

            if !scene_crdt_state.entities.try_init(entity) {
                return Ok(());
            }
            let Some(component_definition) =
                scene_crdt_state.get_lww_component_definition_mut(component)
            else {
                return Ok(());
            };

            component_definition.set_from_binary(entity, timestamp, stream);
        }
        CrdtMessageType::DeleteComponent => {
            let entity = stream.read()?;
            let component: SceneComponentId = stream.read()?;

            #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
            debug_check_component(component, entity, "DeleteComponent");

            let timestamp: SceneCrdtTimestamp = stream.read()?;

            if !scene_crdt_state.entities.try_init(entity) {
                return Ok(());
            }
            let Some(component_definition) =
                scene_crdt_state.get_lww_component_definition_mut(component)
            else {
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

            #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
            debug_check_component(component, entity, "AppendValue");

            let timestamp: SceneCrdtTimestamp = stream.read()?;
            let _content_len = stream.read_u32()? as usize;

            if !scene_crdt_state.entities.try_init(entity) {
                return Ok(());
            }
            let Some(component_definition) =
                scene_crdt_state.get_gos_component_definition_mut(component)
            else {
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
                    tracing::info!("CRDT Buffer error: {:?}", e);
                };
            }
            None => tracing::info!("CRDT Header error: unhandled crdt message type {crdt_type}"),
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
