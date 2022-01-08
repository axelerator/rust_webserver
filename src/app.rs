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
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ToBackend {
    StartGame,
    ChangeSetting,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum RocketJam {
    InLobby(LobbyState),
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
pub struct LobbyState {
    players_ready: Vec<UserId>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Item {
    label: String,
    state: bool,
    user_id: i32,
}

pub fn init_rocket_jam(user_id: UserId) -> RocketJamRound {
    RocketJamRound {
        id: Uuid::new_v4(),
        players: vec![user_id],
        game: RocketJam::InLobby(LobbyState {
            players_ready: vec![],
        }),
    }
}

pub type GameId = String;

pub struct RocketJamApp {}

impl RocketJamApp {
    pub fn update(user_id: UserId, model: &RwLock<Model>, msg: ToBackend) {
        if let Some(round) = find_game_by_user_id(&user_id, model) {
            println!("yeah");
            let updated_round = update_round(&user_id, &round, &msg);
            let round_id = round.id;
            let mut model = model.write().unwrap();
            model
                .games_by_id
                .insert(round_id.to_string(), updated_round);
            //}
        } else {
            if let ToBackend::StartGame = msg {
                let new_round = init_rocket_jam(user_id);
                let mut model = model.write().unwrap();
                model
                    .game_ids_by_user_id
                    .insert(user_id, new_round.id.to_string());
                model
                    .games_by_id
                    .insert(new_round.id.to_string(), new_round);
            }
        }
    }
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

fn update_round(_user_id: &UserId, round: &RocketJamRound, _msg: &ToBackend) -> RocketJamRound {
    round.clone()
}
