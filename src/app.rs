use std::{collections::HashMap, sync::RwLock};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

type UserId = i32;

pub struct Model {
    pub games_by_id: HashMap<GameId, RocketJamRound>,
    pub game_ids_by_user_id: HashMap<UserId, GameId>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ToClient {
    HelloClient,
    UpdateGameState { client_state: ClientState },
    AvailableRounds { round_ids: Vec<GameId> },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ToBackend {
    StartGame,
    Ready,
    ChangeSetting,
    GetAvailableRounds,
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

pub type GameId = String;

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
                .insert(round_id.to_string(), updated_round);
        } else {
            return match msg {
                ToBackend::StartGame => start_game(user_id, model),
                ToBackend::GetAvailableRounds => get_available_rounds(user_id, model),
                _ => vec![],
            };
        }
        vec![]
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
        return vec![(user_id, ToClient::UpdateGameState { client_state })];
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
        (ToBackend::Ready, RocketJam::InLobby { players_ready }) => {
            let mut new_round = round.clone();
            if !players_ready.contains(&user_id) {
                let mut players_ready = players_ready.clone();
                players_ready.push(user_id);
                new_round.game = RocketJam::InLobby { players_ready };
                return new_round;
            }
            new_round
        }
        _ => round.clone(),
    }
}
