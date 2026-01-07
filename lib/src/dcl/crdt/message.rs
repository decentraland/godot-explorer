use crate::dcl::{
    components::{SceneComponentId, SceneCrdtTimestamp, SceneEntityId},
    crdt::SceneCrdtState,
    serialization::{
        reader::{DclReader, DclReaderError},
        writer::DclWriter,
    },
};

#[cfg(feature = "scene_logging")]
use crate::dcl::components::component_id_to_name;

use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::FromPrimitive;

#[cfg(feature = "scene_logging")]
use crate::tools::scene_logging::{
    CrdtDirection, CrdtLogEntry, CrdtOperation, SceneLogEntry, SceneLoggerSender,
};

/// Context for logging CRDT messages (only used when scene_logging feature is enabled).
#[cfg(feature = "scene_logging")]
pub struct CrdtLoggingContext {
    pub sender: SceneLoggerSender,
    pub tick: u32,
    pub direction: CrdtDirection,
}

#[cfg(feature = "scene_logging")]
impl CrdtLoggingContext {
    pub fn new(sender: SceneLoggerSender, tick: u32, direction: CrdtDirection) -> Self {
        Self {
            sender,
            tick,
            direction,
        }
    }
}

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
        (1079, "PBLightSource"),
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

/// Process multiple CRDT messages from a stream.
#[cfg(not(feature = "scene_logging"))]
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

/// Process multiple CRDT messages from a stream with optional logging context.
#[cfg(feature = "scene_logging")]
pub fn process_many_messages(stream: &mut DclReader, scene_crdt_state: &mut SceneCrdtState) {
    process_many_messages_with_logging(stream, scene_crdt_state, None);
}

/// Process multiple CRDT messages from a stream with logging context.
#[cfg(feature = "scene_logging")]
pub fn process_many_messages_with_logging(
    stream: &mut DclReader,
    scene_crdt_state: &mut SceneCrdtState,
    logging_ctx: Option<&CrdtLoggingContext>,
) {
    // collect commands
    while stream.len() > CRDT_HEADER_SIZE {
        let length = stream.read_u32().unwrap() as usize;
        let crdt_type_raw = stream.read_u32().unwrap();
        let message_size = length.saturating_sub(8);
        let mut message_stream = stream.take_reader(message_size);

        match FromPrimitive::from_u32(crdt_type_raw) {
            Some(crdt_type) => {
                // Log the message if logging is enabled
                if let Some(ctx) = logging_ctx {
                    log_crdt_message(
                        ctx,
                        crdt_type_raw,
                        &crdt_type,
                        &message_stream,
                        message_size,
                    );
                }

                if let Err(e) = process_message(scene_crdt_state, crdt_type, &mut message_stream) {
                    tracing::info!("CRDT Buffer error: {:?}", e);
                };
            }
            None => {
                tracing::info!("CRDT Header error: unhandled crdt message type {crdt_type_raw}")
            }
        }
    }
}

#[cfg(feature = "scene_logging")]
fn log_crdt_message(
    ctx: &CrdtLoggingContext,
    _crdt_type_raw: u32,
    crdt_type: &CrdtMessageType,
    message_stream: &DclReader,
    message_size: usize,
) {
    use crate::dcl::components::proto_components::deserialize_component_to_json;
    use crate::tools::scene_logging::current_timestamp_ms;

    // Parse minimal info from message for logging without consuming the stream
    let data = message_stream.as_slice();
    if data.len() < 4 {
        return;
    }

    // Entity ID is first 4 bytes (u16 number + u16 version)
    let entity_number = u16::from_le_bytes([data[0], data[1]]);
    let entity_version = u16::from_le_bytes([data[2], data[3]]);
    let entity_id = ((entity_version as u32) << 16) | (entity_number as u32);

    // Component ID, timestamp, and payload depend on message type
    // Format for PUT: entity(4) + component(4) + timestamp(4) + content_length(4) + payload
    // Format for DELETE: entity(4) + component(4) + timestamp(4)
    // Format for DELETE_ENTITY: entity(4)
    let (component_id, crdt_timestamp, payload, bin_payload) =
        if matches!(crdt_type, CrdtMessageType::DeleteEntity) {
            (0, 0, None, None)
        } else if data.len() >= 12 {
            let comp_id = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
            let timestamp = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);

            // For PutComponent and AppendValue, payload starts after content_length field (offset 16)
            let (payload, bin_payload) = if matches!(
                crdt_type,
                CrdtMessageType::PutComponent | CrdtMessageType::AppendValue
            ) && data.len() > 16
            {
                let payload_data = &data[16..];
                let json_payload = deserialize_component_to_json(comp_id, payload_data);
                // Encode as hex string
                let hex_payload: String = payload_data.iter().map(|b| format!("{:02x}", b)).collect();
                (json_payload, Some(hex_payload))
            } else {
                (None, None)
            };

            (comp_id, timestamp, payload, bin_payload)
        } else {
            (0, 0, None, None)
        };

    let operation = match crdt_type {
        CrdtMessageType::PutComponent => CrdtOperation::Put,
        CrdtMessageType::DeleteComponent => CrdtOperation::Delete,
        CrdtMessageType::DeleteEntity => CrdtOperation::DeleteEntity,
        CrdtMessageType::AppendValue => CrdtOperation::Append,
    };

    let entry = CrdtLogEntry {
        tick: ctx.tick,
        timestamp_ms: current_timestamp_ms(),
        direction: ctx.direction,
        entity_id,
        component_name: component_id_to_name(component_id).to_string(),
        operation,
        crdt_timestamp,
        payload,
        bin_payload,
        raw_size_bytes: message_size,
    };

    let _ = ctx.sender.try_send(SceneLogEntry::CrdtMessage(entry));
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
