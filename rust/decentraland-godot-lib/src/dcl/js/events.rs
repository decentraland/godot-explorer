use crate::dcl::{
    components::{
        proto_components::sdk::components::common::{InputAction, PointerEventType, RaycastHit},
        SceneComponentId, SceneEntityId,
    },
    crdt::{
        grow_only_set::GenericGrowOnlySetComponentOperation,
        last_write_wins::LastWriteWinsComponentOperation, DirtyCrdtState, SceneCrdtState,
        SceneCrdtStateProtoComponents,
    },
};
use deno_core::{op, Op, OpDecl, OpState};
use ethers_core::types::H160;
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    marker::PhantomData,
};

pub fn ops() -> Vec<OpDecl> {
    vec![
        op_subscribe::DECL,
        op_unsubscribe::DECL,
        op_send_batch::DECL,
    ]
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EventBodyUserId {
    user_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EventBodyExpressionId {
    expression_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EventBodyProfileChanged {
    eth_address: String,
    version: i32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EventBodyRealmChanged {
    domain: String,
    room: String,
    server_name: String,
    display_name: String,
}

#[derive(Serialize)]
struct EventComms {
    sender: String,
    message: String,
}

#[derive(Serialize)]
struct EventBodyRay {
    origin: EventBodyVector3,
    direction: EventBodyVector3,
    distance: f32,
}

#[derive(Serialize)]
struct EventBodyVector3 {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EventBodyPlayerClicked {
    user_id: String,
    ray: EventBodyRay,
}

impl From<Option<&RaycastHit>> for EventBodyRay {
    fn from(ray: Option<&RaycastHit>) -> Self {
        if let Some(ray) = ray {
            Self {
                origin: EventBodyVector3 {
                    x: ray.global_origin.as_ref().map(|v| v.x).unwrap_or_default(),
                    y: ray.global_origin.as_ref().map(|v| v.y).unwrap_or_default(),
                    z: ray.global_origin.as_ref().map(|v| v.z).unwrap_or_default(),
                },
                direction: EventBodyVector3 {
                    x: ray.direction.as_ref().map(|v| v.x).unwrap_or_default(),
                    y: ray.direction.as_ref().map(|v| v.y).unwrap_or_default(),
                    z: ray.direction.as_ref().map(|v| v.z).unwrap_or_default(),
                },
                distance: ray.length,
            }
        } else {
            Self {
                origin: EventBodyVector3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                direction: EventBodyVector3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                distance: 0.0,
            }
        }
    }
}

trait EventType {
    fn label() -> &'static str;
}

macro_rules! impl_event {
    ($name: ident, $label: expr) => {
        #[derive(Debug)]
        struct $name;
        impl EventType for $name {
            fn label() -> &'static str {
                $label
            }
        }
    };
}

impl_event!(PlayerConnected, "playerConnected");
impl_event!(PlayerDisconnected, "playerDisconnected");
impl_event!(PlayerEnteredScene, "onEnterScene");
impl_event!(PlayerLeftScene, "onLeaveScene");
impl_event!(SceneReady, "sceneStart");
impl_event!(PlayerExpression, "playerExpression");
impl_event!(ProfileChanged, "profileChanged");
impl_event!(RealmChanged, "onRealmChanged");
impl_event!(PlayerClicked, "playerClicked");
impl_event!(MessageBus, "comms");

struct EventReceiver<T: EventType> {
    inner: tokio::sync::mpsc::UnboundedReceiver<String>,
    _p: PhantomData<fn() -> T>,
}

struct EventSender<T: EventType> {
    inner: tokio::sync::mpsc::UnboundedSender<String>,
    _p: PhantomData<fn() -> T>,
}

#[op]
fn op_subscribe(state: &mut OpState, id: &str) {
    macro_rules! register {
        ($id: expr, $state: expr, $marker: ty) => {{
            if id == <$marker as EventType>::label() {
                if $state.has::<EventReceiver<$marker>>() {
                    return;
                }
                let (sx, rx) = tokio::sync::mpsc::unbounded_channel::<String>();

                state.put(EventReceiver::<$marker> {
                    inner: rx,
                    _p: Default::default(),
                });
                state.put(EventSender::<$marker> {
                    inner: sx,
                    _p: Default::default(),
                });

                tracing::debug!("subscribed to {}", <$marker as EventType>::label());
                return;
            }
        }};
    }

    register!(id, state, PlayerConnected);
    register!(id, state, PlayerDisconnected);
    register!(id, state, PlayerEnteredScene);
    register!(id, state, PlayerLeftScene);
    register!(id, state, SceneReady);
    register!(id, state, PlayerExpression);
    register!(id, state, ProfileChanged);
    register!(id, state, RealmChanged);
    register!(id, state, PlayerClicked);
    register!(id, state, MessageBus);

    tracing::warn!("subscribe to unrecognised event {id}");
}

#[op]
fn op_unsubscribe(state: &mut OpState, id: &str) {
    macro_rules! unregister {
        ($id: expr, $state: expr, $marker: ty) => {{
            if id == <$marker as EventType>::label() {
                // removing the receiver will cause the sender to error so it can be cleaned up at the sender side
                state.try_take::<EventReceiver<$marker>>();
                state.try_take::<EventSender<$marker>>();
                return;
            }
        }};
    }

    unregister!(id, state, PlayerConnected);
    unregister!(id, state, PlayerDisconnected);
    unregister!(id, state, PlayerEnteredScene);
    unregister!(id, state, PlayerLeftScene);
    unregister!(id, state, SceneReady);
    unregister!(id, state, PlayerExpression);
    unregister!(id, state, ProfileChanged);
    unregister!(id, state, RealmChanged);
    unregister!(id, state, PlayerClicked);
    unregister!(id, state, MessageBus);

    tracing::warn!("unsubscribe for unrecognised event {id}");
}

#[derive(Serialize)]
struct Event {
    generic: EventGeneric,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EventGeneric {
    event_id: String,
    event_data: String,
}

#[op]
fn op_send_batch(state: &mut OpState) -> Vec<Event> {
    let mut results = Vec::default();

    macro_rules! poll {
        ($state: expr, $marker: ty, $id: expr) => {{
            if let Some(receiver) = state.try_borrow_mut::<EventReceiver<$marker>>() {
                while let Ok(event_data) = receiver.inner.try_recv() {
                    tracing::debug!("received {} event", <$marker as EventType>::label());
                    results.push(Event {
                        generic: EventGeneric {
                            event_id: $id.to_owned(),
                            event_data,
                        },
                    });
                }
            }
        }};
    }

    poll!(state, PlayerConnected, "playerConnected");
    poll!(state, PlayerDisconnected, "playerDisconnected");
    poll!(state, PlayerEnteredScene, "onEnterScene");
    poll!(state, PlayerLeftScene, "onLeaveScene");
    poll!(state, PlayerClicked, "playerClicked");
    poll!(state, PlayerExpression, "playerExpression");
    poll!(state, ProfileChanged, "profileChanged");

    poll!(state, RealmChanged, "onRealmChanged");
    poll!(state, MessageBus, "comms");
    poll!(state, SceneReady, "sceneStart");

    results
}

pub fn process_events(
    op_state: &mut OpState,
    crdt_state: &SceneCrdtState,
    dirty_crdt_state: &DirtyCrdtState,
    comms_string: Vec<(H160, Vec<u8>)>,
) {
    process_events_players_stateful(op_state, crdt_state, dirty_crdt_state);
    process_events_players_stateless(op_state, crdt_state, dirty_crdt_state);

    let messages = comms_string
        .into_iter()
        .map(|(sender_address, data)| {
            let sender = format!("{:#x}", sender_address);
            let message = String::from_utf8_lossy(&data[1..]).to_string();
            EventComms { sender, message }
        })
        .collect::<Vec<_>>();

    if !messages.is_empty() {
        if let Some(message_bus_sender) = op_state.try_take::<EventSender<MessageBus>>() {
            messages.into_iter().for_each(|message| {
                message_bus_sender
                    .inner
                    .send(serde_json::to_string(&message).unwrap())
                    .unwrap();
            });
            op_state.put(message_bus_sender);
        }
    }

    let engine_info_component = SceneCrdtStateProtoComponents::get_engine_info(crdt_state);
    let tick_number = if let Some(entry) = engine_info_component.get(&SceneEntityId::ROOT) {
        if let Some(value) = entry.value.as_ref() {
            value.tick_number
        } else {
            0
        }
    } else {
        0
    };

    if tick_number == 4 {
        if let Some(scene_ready_sender) = op_state.try_take::<EventSender<SceneReady>>() {
            scene_ready_sender.inner.send("{}".to_string()).unwrap();
            op_state.put(scene_ready_sender);
        }
    }

    // TODO: RealmChanged, it needs to add a new component to the crdt (realmInfo or something)
}

struct EventPlayerState {
    current_players: HashMap<SceneEntityId, String>,
    inside_scene: HashSet<SceneEntityId>,
}

impl EventPlayerState {
    fn new(crdt_state: &SceneCrdtState) -> Self {
        let player_identity_data_component =
            SceneCrdtStateProtoComponents::get_player_identity_data(crdt_state);
        let internal_player_data_component = crdt_state.get_internal_player_data();

        let current_players =
            HashMap::from_iter(player_identity_data_component.values.iter().filter_map(
                |(entity_id, value)| {
                    value
                        .value
                        .as_ref()
                        .map(|value| (*entity_id, value.address.clone()))
                },
            ));

        let inside_scene =
            HashSet::from_iter(internal_player_data_component.values.iter().filter_map(
                |(entity_id, value)| {
                    if let Some(internal_player_data) = value.value.as_ref() {
                        if internal_player_data.inside {
                            Some(*entity_id)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                },
            ));
        Self {
            current_players,
            inside_scene,
        }
    }
}

pub fn process_events_players_stateless(
    op_state: &mut OpState,
    crdt_state: &SceneCrdtState,
    dirty_crdt_state: &DirtyCrdtState,
) {
    let player_identity_data_component =
        SceneCrdtStateProtoComponents::get_player_identity_data(crdt_state);

    if let Some(player_clicked_sender) = op_state.try_take::<EventSender<PlayerClicked>>() {
        let pointer_events_result_component =
            SceneCrdtStateProtoComponents::get_pointer_events_result(crdt_state);

        if let Some(pointer_event_results) = dirty_crdt_state
            .gos
            .get(&SceneComponentId::POINTER_EVENTS_RESULT)
        {
            for (entity_id, elements_count) in pointer_event_results {
                let Some(user_id) = player_identity_data_component
                    .values
                    .get(entity_id)
                    .and_then(|value| value.value.as_ref())
                    .map(|value| value.address.clone())
                else {
                    continue;
                };
                let Some(grow_only_set) = pointer_events_result_component.values.get(entity_id)
                else {
                    continue;
                };

                for i in 0..*elements_count {
                    let Some(value) = grow_only_set.get(i) else {
                        continue;
                    };
                    if value.button() == InputAction::IaPointer
                        && value.state() == PointerEventType::PetDown
                    {
                        player_clicked_sender
                            .inner
                            .send(
                                serde_json::to_string(&EventBodyPlayerClicked {
                                    user_id: user_id.clone(),
                                    ray: EventBodyRay::from(value.hit.as_ref()),
                                })
                                .expect("fail json serialize"),
                            )
                            .unwrap();
                    }
                }
            }
        }

        op_state.put(player_clicked_sender);
    }

    // Note: The player expression event is only for the current player, not foreign players
    if let Some(player_expression_sender) = op_state.try_take::<EventSender<PlayerExpression>>() {
        let avatar_emote_command_component =
            SceneCrdtStateProtoComponents::get_avatar_emote_command(crdt_state);

        let new_values_count = {
            if let Some(dirty_crdt_state) = dirty_crdt_state
                .gos
                .get(&SceneComponentId::AVATAR_EMOTE_COMMAND)
            {
                *dirty_crdt_state.get(&SceneEntityId::PLAYER).unwrap_or(&0)
            } else {
                0
            }
        };

        if new_values_count > 0 {
            if let Some(emote_command) = avatar_emote_command_component.get(&SceneEntityId::PLAYER)
            {
                for i in 0..new_values_count {
                    let Some(value) = emote_command.get(i) else {
                        continue;
                    };

                    player_expression_sender
                        .inner
                        .send(
                            serde_json::to_string(&EventBodyExpressionId {
                                expression_id: value.emote_urn.clone(),
                            })
                            .expect("fail json serialize"),
                        )
                        .unwrap();
                }
            }
        }

        op_state.put(player_expression_sender);
    }

    // TODO: implement this when version is in the avatar components
    if let Some(profile_changed_sender) = op_state.try_take::<EventSender<ProfileChanged>>() {
        op_state.put(profile_changed_sender);
    }
}

pub fn process_events_players_stateful(
    op_state: &mut OpState,
    crdt_state: &SceneCrdtState,
    dirty_crdt_state: &DirtyCrdtState,
) {
    let is_subscribed = op_state.has::<EventSender<PlayerConnected>>()
        || op_state.has::<EventSender<PlayerDisconnected>>()
        || op_state.has::<EventSender<PlayerEnteredScene>>()
        || op_state.has::<EventSender<PlayerLeftScene>>();

    if !is_subscribed {
        // When it's not subscribed, clean the state if it'exists
        let _ = op_state.try_take::<EventPlayerState>();
        return;
    }

    let mut events_state = {
        if let Some(events_state) = op_state.try_take::<EventPlayerState>() {
            events_state
        } else {
            // First tick after subscription
            EventPlayerState::new(crdt_state)
        }
    };

    let player_identity_data_component =
        SceneCrdtStateProtoComponents::get_player_identity_data(crdt_state);

    let player_identity_data_component_dirty = dirty_crdt_state
        .lww
        .get(&SceneComponentId::PLAYER_IDENTITY_DATA);

    let player_connected_sender = op_state.try_take::<EventSender<PlayerConnected>>();
    let player_disconnected_sender = op_state.try_take::<EventSender<PlayerDisconnected>>();

    if let Some(player_identity_data_component_dirty) = player_identity_data_component_dirty {
        for entity_id in player_identity_data_component_dirty {
            let existing_value = {
                if let Some(value) = player_identity_data_component
                    .values
                    .get(entity_id)
                    .as_ref()
                {
                    value.value.as_ref()
                } else {
                    None
                }
            };

            if let Some(player_identity_value) = existing_value {
                events_state
                    .current_players
                    .insert(*entity_id, player_identity_value.address.clone());
                if let Some(player_connected_sender) = player_connected_sender.as_ref() {
                    player_connected_sender
                        .inner
                        .send(
                            serde_json::to_string(&EventBodyUserId {
                                user_id: player_identity_value.address.clone(),
                            })
                            .expect("fail json serialize"),
                        )
                        .unwrap();
                }
            } else {
                let address = events_state.current_players.remove(entity_id);

                if let Some(user_id) = address {
                    if let Some(player_disconnected_sender) = player_disconnected_sender.as_ref() {
                        player_disconnected_sender
                            .inner
                            .send(
                                serde_json::to_string(&EventBodyUserId { user_id })
                                    .expect("fail json serialize"),
                            )
                            .unwrap();
                    }
                }
            }
        }
    }

    if let Some(player_connected_sender) = player_connected_sender {
        op_state.put(player_connected_sender);
    }
    if let Some(player_disconnected_sender) = player_disconnected_sender {
        op_state.put(player_disconnected_sender);
    }

    let player_entered_scene_sender = op_state.try_take::<EventSender<PlayerEnteredScene>>();
    let player_left_scene_sender = op_state.try_take::<EventSender<PlayerLeftScene>>();

    let internal_player_data_dirty = dirty_crdt_state
        .lww
        .get(&SceneComponentId::INTERNAL_PLAYER_DATA);
    let internal_player_data_component = crdt_state.get_internal_player_data();

    if let Some(internal_player_data_dirty) = internal_player_data_dirty {
        for entity_id in internal_player_data_dirty {
            let entity_is_notified_as_inside = {
                if let Some(value) = internal_player_data_component
                    .values
                    .get(entity_id)
                    .as_ref()
                {
                    if let Some(internal_player_data_value) = value.value.as_ref() {
                        internal_player_data_value.inside
                    } else {
                        false
                    }
                } else {
                    false
                }
            };
            let entity_is_inside = events_state.inside_scene.contains(entity_id);

            if entity_is_notified_as_inside != entity_is_inside {
                if entity_is_notified_as_inside {
                    events_state.inside_scene.insert(*entity_id);

                    if let Some(user_id) = events_state.current_players.get(entity_id).cloned() {
                        if let Some(player_entered_scene_sender) =
                            player_entered_scene_sender.as_ref()
                        {
                            player_entered_scene_sender
                                .inner
                                .send(
                                    serde_json::to_string(&EventBodyUserId { user_id })
                                        .expect("fail json serialize"),
                                )
                                .unwrap();
                        }
                    }
                } else {
                    events_state.inside_scene.remove(entity_id);

                    if let Some(user_id) = events_state.current_players.get(entity_id).cloned() {
                        if let Some(player_left_scene_sender) = player_left_scene_sender.as_ref() {
                            player_left_scene_sender
                                .inner
                                .send(
                                    serde_json::to_string(&EventBodyUserId { user_id })
                                        .expect("fail json serialize"),
                                )
                                .unwrap();
                        }
                    } else {
                        tracing::error!("entity left scene but no player identity data found");
                    }
                }
            }
        }
    }

    if let Some(player_entered_scene_sender) = player_entered_scene_sender {
        op_state.put(player_entered_scene_sender);
    }
    if let Some(player_left_scene_sender) = player_left_scene_sender {
        op_state.put(player_left_scene_sender);
    }

    op_state.put(events_state);
}
