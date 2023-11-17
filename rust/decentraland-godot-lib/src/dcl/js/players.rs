use std::{cell::RefCell, rc::Rc};

use deno_core::{
    anyhow::{self},
    error::AnyError,
    op, Op, OpDecl, OpState,
};

use crate::dcl::{
    crdt::{SceneCrdtState, SceneCrdtStateProtoComponents},
    scene_apis::{LocalCall, UserData},
};

pub fn ops() -> Vec<OpDecl> {
    vec![
        op_get_player_data::DECL,
        op_get_connected_players::DECL,
        op_get_players_in_scene::DECL,
    ]
}

#[op]
async fn op_get_player_data(
    op_state: Rc<RefCell<OpState>>,
    user_id: String,
) -> Result<Option<UserData>, AnyError> {
    let (sx, rx) = tokio::sync::oneshot::channel::<Option<UserData>>();

    op_state
        .borrow_mut()
        .borrow_mut::<Vec<LocalCall>>()
        .push(LocalCall::PlayersGetPlayerData {
            user_id,
            response: sx.into(),
        });

    rx.await.map_err(|e| anyhow::anyhow!(e))
}

#[op]
async fn op_get_players_in_scene(op_state: Rc<RefCell<OpState>>) -> Result<Vec<String>, AnyError> {
    let (sx, rx) = tokio::sync::oneshot::channel::<Vec<String>>();

    op_state.borrow_mut().borrow_mut::<Vec<LocalCall>>().push(
        LocalCall::PlayersGetPlayersInScene {
            response: sx.into(),
        },
    );

    rx.await.map_err(|e| anyhow::anyhow!(e))
}

#[op]
async fn op_get_connected_players(op_state: Rc<RefCell<OpState>>) -> Result<Vec<String>, AnyError> {
    let (sx, rx) = tokio::sync::oneshot::channel::<Vec<String>>();

    op_state.borrow_mut().borrow_mut::<Vec<LocalCall>>().push(
        LocalCall::PlayersGetConnectedPlayers {
            response: sx.into(),
        },
    );

    rx.await.map_err(|e| anyhow::anyhow!(e))
}

pub fn get_players(crdt_state: &SceneCrdtState, only_in_scene: bool) -> Vec<String> {
    let player_identity_data_component =
        SceneCrdtStateProtoComponents::get_player_identity_data(crdt_state);
    let transform_component = crdt_state.get_transform();

    player_identity_data_component
        .values
        .iter()
        .filter(|(entity_id, entry)| {
            let Some(_) = entry.value.as_ref() else {
                return false;
            };
            let Some(transform_entry) = transform_component.values.get(entity_id) else {
                return false;
            };
            if only_in_scene {
                let Some(_) = transform_entry.value.as_ref() else {
                    return false;
                };
            }
            true
        })
        .map(|v| {
            v.1.value
                .as_ref()
                .expect("previously acceded to filter")
                .address
                .clone()
        })
        .collect::<Vec<String>>()
}

pub fn get_player_data(user_id: String, crdt_state: &SceneCrdtState) -> Option<UserData> {
    let player_identity_data_component =
        SceneCrdtStateProtoComponents::get_player_identity_data(crdt_state);
    let avatar_base_component = SceneCrdtStateProtoComponents::get_avatar_base(crdt_state);
    let avatar_equipped_data_component =
        SceneCrdtStateProtoComponents::get_avatar_equipped_data(crdt_state);

    let (player_entity_id, player_entry) =
        player_identity_data_component
            .values
            .iter()
            .find(|(_entity_id, entry)| {
                if let Some(data) = entry.value.as_ref() {
                    return data.address == user_id;
                }
                false
            })?;

    let player_identity_data_value = player_entry.value.as_ref()?;
    let avatar_base_value = avatar_base_component
        .values
        .get(player_entity_id)?
        .value
        .as_ref()?;
    let _avatar_equipped_data_value = avatar_equipped_data_component
        .values
        .get(player_entity_id)?
        .value
        .as_ref()?;

    let user_data = UserData {
        display_name: avatar_base_value.name.clone(),
        public_key: if player_identity_data_value.is_guest {
            None
        } else {
            Some(player_identity_data_value.address.clone())
        },
        has_connected_web3: !player_identity_data_value.is_guest,
        user_id: player_identity_data_value.address.clone(),
        version: 0, // TODO: how to get this?
        avatar: None,
        // avatar: Some(AvatarForUserData {
        //     body_shape: avatar_base_value.body_shape_urn.clone(),
        //     skin_color: avatar_base_value.skin_color.clone().unwrap_or(Color3).to_string(),
        //     hair_color: String,
        //     eye_color: String,
        //     wearables: Vec<String>,
        //     snapshots: Option<Snapshots>
        // }),
    };
    Some(user_data)
}
