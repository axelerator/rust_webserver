use std::{collections::HashMap, sync::RwLock};

use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

type UserId = i32;

pub struct Model {
    pub games_by_id: HashMap<RoundId, RocketJamRound>,
    pub game_ids_by_user_id: HashMap<UserId, RoundId>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ToClient {
    HelloClient,
    UpdateGameState { client_state: ClientState },
    AvailableRounds { round_ids: Vec<RoundId> },
    EnterRound { client_state: ClientState },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ToBackend {
    StartGame,
    ToggleReady,
    ChangeSetting,
    GetAvailableRounds,
    JoinGame { round_id: RoundId },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum RocketJam {
    InLobby { players_ready: Vec<UserId> },
    InLevel(LevelState),
}

#[derive(Clone, Debug, PartialEq)]
pub struct RocketJamRound {
    id: Uuid,
    players: Vec<UserId>,
    game: RocketJam,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LevelState {
    items: Vec<Item>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Item {
    label: String,
    state: bool,
    user_id: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ClientState {
    Lobby {
        player_count: usize,
        player_ready_count: usize,
    },
    InGame {
        current_instruction: String,
        ui_items: Vec<ClientUiItem>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ClientUiItem {
    label: String,
    state: bool,
}

fn client_state_for_user(user_id: UserId, round: &RocketJamRound) -> Option<ClientState> {
    match &round.game {
        RocketJam::InLobby { players_ready } => Some(ClientState::Lobby {
            player_count: round.players.len(),
            player_ready_count: players_ready.len(),
        }),
        RocketJam::InLevel(details) => level_for_user(user_id, details),
    }
}

fn level_for_user(user_id: UserId, level: &LevelState) -> Option<ClientState> {
    let ui_items: Vec<ClientUiItem> = level
        .items
        .iter()
        .filter(|i| i.user_id.eq(&user_id))
        .map(|i| ClientUiItem {
            label: i.label.clone(),
            state: i.state,
        })
        .collect();
    Some(ClientState::InGame {
        current_instruction: "Brew coffee".to_string(),
        ui_items,
    })
}

pub fn init_rocket_jam(user_id: UserId) -> RocketJamRound {
    let players_ready: Vec<UserId> = vec![];
    RocketJamRound {
        id: Uuid::new_v4(),
        players: vec![user_id],
        game: RocketJam::InLobby { players_ready },
    }
}

pub type RoundId = String;

pub struct RocketJamApp {}

type ClientMessage = (UserId, ToClient);

impl RocketJamApp {
    pub fn update(user_id: UserId, model: &RwLock<Model>, msg: ToBackend) -> Vec<ClientMessage> {
        if let Some(round) = find_game_by_user_id(&user_id, model) {
            let updated_round = update_round(user_id, &round, &msg);
            let round_id = round.id;
            let mut model = model.write().unwrap();
            model
                .games_by_id
                .insert(round_id.to_string(), updated_round.clone());
            round
                .players
                .iter()
                .map(
                    |user_id| match client_state_for_user(user_id.clone(), &updated_round) {
                        Some(client_state) => {
                            Some((*user_id, ToClient::UpdateGameState { client_state }))
                        }
                        None => None,
                    },
                )
                .flatten()
                .collect()
        } else {
            return match msg {
                ToBackend::StartGame => start_game(user_id, model),
                ToBackend::GetAvailableRounds => get_available_rounds(user_id, model),
                ToBackend::JoinGame { round_id } => join_game(user_id, &round_id, model),
                _ => vec![],
            };
        }
    }
}

fn join_game(user_id: UserId, round_id: &RoundId, model: &RwLock<Model>) -> Vec<ClientMessage> {
    let round = find_round_by_id(round_id, model);
    match round {
        Some(round) => {
            let other_players = round.players.iter();

            let mut players = round.players.to_vec();
            players.push(user_id);
            let round_with_user = RocketJamRound {
                players,
                ..round.clone()
            };

            let client_state = client_state_for_user(user_id, &round_with_user);
            let mut model = model.write().unwrap();
            let round_id = &round_with_user.id;
            model
                .games_by_id
                .insert(round_id.to_string(), round_with_user.clone());
            model
                .game_ids_by_user_id
                .insert(user_id, round_id.to_string());
            drop(model); // release lock asap
            match client_state {
                Some(client_state) => other_players
                    .into_iter()
                    .map(|user_id| {
                        (
                            user_id,
                            client_state_for_user(user_id.clone(), &round_with_user),
                        )
                    })
                    .map(|(user_id, client_state)| match client_state {
                        Some(client_state) => {
                            Some((*user_id, ToClient::UpdateGameState { client_state }))
                        }
                        None => None,
                    })
                    .flatten()
                    .chain(vec![(user_id, ToClient::EnterRound { client_state })])
                    .collect(),
                None => {
                    error!(
                        "couldn't generate client state for user {:?} when joining game {:?}",
                        &user_id, &round_id
                    );
                    vec![]
                }
            }
        }
        None => {
            warn!("round {:?} not found", &round_id);
            vec![]
        }
    }
}

fn find_round_by_id(round_id: &RoundId, model: &RwLock<Model>) -> Option<RocketJamRound> {
    match model.read().unwrap().games_by_id.get(round_id) {
        Some(round) => Some(round.clone()),
        None => None,
    }
}

fn get_available_rounds(user_id: UserId, model: &RwLock<Model>) -> Vec<ClientMessage> {
    let model = model.read().unwrap();
    let round_ids: Vec<String> = model
        .games_by_id
        .values()
        .filter(|round| {
            if let RocketJam::InLobby { .. } = round.game {
                true
            } else {
                false
            }
        })
        .map(|round| round.id.to_string())
        .collect();
    vec![(user_id, ToClient::AvailableRounds { round_ids })]
}

fn start_game(user_id: UserId, model: &RwLock<Model>) -> Vec<ClientMessage> {
    let new_round = init_rocket_jam(user_id);
    let mut model = model.write().unwrap();
    model
        .game_ids_by_user_id
        .insert(user_id, new_round.id.to_string());
    model
        .games_by_id
        .insert(new_round.id.to_string(), new_round.clone());
    if let Some(client_state) = client_state_for_user(user_id, &new_round) {
        return vec![(user_id, ToClient::EnterRound { client_state })];
    }
    vec![]
}

fn find_game_by_user_id(user_id: &UserId, model: &RwLock<Model>) -> Option<RocketJamRound> {
    let model = model.read().unwrap();
    if let Some(game_id) = model.game_ids_by_user_id.get(&user_id) {
        if let Some(round) = model.games_by_id.get(game_id) {
            return Some(round.clone());
        }
    }
    None
}

pub fn init_model() -> Model {
    Model {
        games_by_id: HashMap::new(),
        game_ids_by_user_id: HashMap::new(),
    }
}

fn update_round(user_id: UserId, round: &RocketJamRound, msg: &ToBackend) -> RocketJamRound {
    match (msg, &round.game) {
        (ToBackend::ToggleReady, RocketJam::InLobby { players_ready }) => {
            let mut new_round = round.clone();
            if players_ready.contains(&user_id) {
                let mut players_ready: Vec<UserId> = players_ready.to_vec();
                players_ready.retain(|player_id| *player_id != user_id);
                info!("User {:?} was ready, turning off", &user_id);
                new_round.game = RocketJam::InLobby { players_ready };
                new_round
            } else {
                let mut players_ready = players_ready.clone();
                info!("User {:?} wasn't ready, turning on", &user_id);
                players_ready.push(user_id);
                new_round.game = RocketJam::InLobby { players_ready };
                new_round
            }
        }
        _ => round.clone(),
    }
}
