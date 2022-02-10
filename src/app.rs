use std::collections::HashMap;

use log::{error, info, warn};
use rand::{prelude::SliceRandom, thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::user::{User, UserId};

const ITEMS: &'static [&'static str] = &[
    "Chemex Coffeemaker",
    "Sound system",
    "Pizza oven",
    "Foot massager",
    "Heating",
    "Radio",
    "Windshield wiper",
    "Flux compensator",
    "Warp Core",
    "Fridge",
    "Fireplace",
    "Ventilation",
];

pub struct Model {
    pub games_by_id: HashMap<RoundId, RocketJamRound>,
    pub game_ids_by_user_id: HashMap<UserId, RoundId>,
    pub tick: i32,
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
    Init,
    StartGame,
    ToggleReady,
    ChangeSetting { item_id: ItemId, value: u8 },
    GetAvailableRounds,
    JoinGame { round_id: RoundId },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum RocketJam {
    InLobby { players_ready: Vec<UserId> },
    InLevel(RoundState),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RoundState {
    available_items: Vec<(ItemId, String, u8)>,
    items: Vec<Item>,
    instructions: Vec<Instruction>,
    instructions_executed: usize,
    instructions_missed: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Instruction {
    user_id: UserId,
    item_id: ItemId,
    state: u8,
    eol_tick: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RocketJamRound {
    id: Uuid,
    players: Vec<UserId>,
    game: RocketJam,
}

type ItemId = usize;
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Item {
    id: ItemId,
    label: String,
    state: u8,
    user_id: i32,
    max_value: u8,
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
        instructions_executed: usize,
        instructions_missed: usize,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ClientUiItem {
    id: ItemId,
    label: String,
    state: u8,
    max_value: u8,
}

fn client_state_for_user(user_id: UserId, round: &RocketJamRound) -> Option<ClientState> {
    match &round.game {
        RocketJam::InLobby { players_ready } => Some(ClientState::Lobby {
            player_count: round.players.len(),
            player_ready_count: players_ready.len(),
        }),

        RocketJam::InLevel(round_state) => level_for_user(user_id, &round_state),
    }
}

fn level_for_user(user_id: UserId, round_state: &RoundState) -> Option<ClientState> {
    let ui_items: Vec<ClientUiItem> = round_state
        .items
        .iter()
        .filter(|i| i.user_id.eq(&user_id))
        .map(|i| ClientUiItem {
            id: i.id,
            label: i.label.clone(),
            state: i.state,
            max_value: i.max_value,
        })
        .collect();
    let current_instruction = if let Some(instruction) = round_state
        .instructions
        .iter()
        .find(|i| i.user_id == user_id)
    {
        let item = round_state
            .items
            .iter()
            .find(|i| i.id == instruction.item_id)
            .unwrap();
        format!("Turn {} to {}", item.label, instruction.state)
    } else {
        error!("Got no instruction for user {:?}", user_id);
        "".to_string()
    };

    Some(ClientState::InGame {
        current_instruction,
        ui_items,
        instructions_executed: round_state.instructions_executed,
        instructions_missed: round_state.instructions_missed,
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

#[derive(Clone)]
pub struct RocketJamApp {
    model: Arc<RwLock<Model>>,
}

pub type ClientMessage = (UserId, ToClient);

impl RocketJamApp {
    pub fn new() -> Self {
        RocketJamApp {
            model: Arc::new(RwLock::new(init_model())),
        }
    }

    pub async fn tick(&self) -> Vec<ClientMessage> {
        let mut model = self.model.write().await;
        let mut updated_rounds = HashMap::new();
        let mut msgs = Vec::new();
        for (key, round) in model.games_by_id.iter() {
            let (updated_round, client_messages) = tick_round(round, model.tick);
            updated_rounds.insert(key.clone(), updated_round);
            let mut ccc = client_messages.clone();
            msgs.append(&mut ccc);
        }
        model.games_by_id = updated_rounds;
        model.tick += 1;
        msgs
    }

    pub async fn update(&self, user: &User, msg: ToBackend) -> Vec<ClientMessage> {
        info!("app update with msg {:?}", msg);
        if let Some(round) = find_game_by_user_id(&user.id, &self.model).await {
            let mut model = self.model.write().await;
            let updated_round = update_round(user.id, &round, &msg, model.tick);
            let round_id = round.id;
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
                ToBackend::Init => get_available_rounds(user.id, &self.model).await,
                ToBackend::StartGame => start_game(user.id, &self.model).await,
                ToBackend::GetAvailableRounds => get_available_rounds(user.id, &self.model).await,
                ToBackend::JoinGame { round_id } => {
                    join_game(user.id, &round_id, &self.model).await
                }
                _ => vec![],
            };
        }
    }
}

fn tick_round(round: &RocketJamRound, current_tick: i32) -> (RocketJamRound, Vec<ClientMessage>) {
    match &round.game {
        RocketJam::InLevel(round_state) => {
            let mut instructions: Vec<Instruction> = Vec::new();
            let mut instructions_missed = round_state.instructions_missed;
            let mut updated = false;
            for instruction in &round_state.instructions {
                if instruction.eol_tick == current_tick {
                    instructions_missed += 1;
                    if let Some(instruction) =
                        mk_instructions(instruction.user_id, &round_state.items, current_tick)
                    {
                        instructions.push(instruction);
                        updated = true;
                    } else {
                        error!("Got no instruction");
                    }
                } else {
                    instructions.push(instruction.clone());
                }
            }
            let updated_round_state = RoundState {
                instructions,
                instructions_missed,
                ..round_state.clone()
            };

            let rocket_jam = RocketJam::InLevel(updated_round_state);
            let updated_round = RocketJamRound {
                game: rocket_jam,
                ..round.clone()
            };
            if !updated {
                (updated_round, vec![])
            } else {
                let msgs: Vec<ClientMessage> = updated_round
                    .players
                    .iter()
                    .map(
                        |user_id| match client_state_for_user(*user_id, &updated_round) {
                            Some(client_state) => {
                                Some((*user_id, ToClient::UpdateGameState { client_state }))
                            }
                            None => None,
                        },
                    )
                    .flatten()
                    .collect();
                (updated_round, msgs)
            }
        }
        RocketJam::InLobby { .. } => (round.clone(), vec![]),
    }
}

async fn join_game(
    user_id: UserId,
    round_id: &RoundId,
    model: &RwLock<Model>,
) -> Vec<ClientMessage> {
    let round = find_round_by_id(round_id, model).await;
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
            let mut model = model.write().await;
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

async fn find_round_by_id(round_id: &RoundId, model: &RwLock<Model>) -> Option<RocketJamRound> {
    match model.read().await.games_by_id.get(round_id) {
        Some(round) => Some(round.clone()),
        None => None,
    }
}

async fn get_available_rounds(user_id: UserId, model: &RwLock<Model>) -> Vec<ClientMessage> {
    info!("get_availble_rounds for {:?}", user_id);
    let model = model.read().await;
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

async fn start_game(user_id: UserId, model: &RwLock<Model>) -> Vec<ClientMessage> {
    info!("Starting new game");
    let new_round = init_rocket_jam(user_id);
    let mut model = model.write().await;
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

async fn find_game_by_user_id(user_id: &UserId, model: &RwLock<Model>) -> Option<RocketJamRound> {
    let model = model.read().await;
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
        tick: 0,
    }
}

fn update_round(
    user_id: UserId,
    round: &RocketJamRound,
    msg: &ToBackend,
    current_tick: i32,
) -> RocketJamRound {
    match (msg, &round.game) {
        (ToBackend::ToggleReady, RocketJam::InLobby { players_ready }) => {
            toggle_ready(user_id, players_ready, round, current_tick)
        }
        (ToBackend::ChangeSetting { item_id, value }, RocketJam::InLevel(game_state)) => {
            change_setting(user_id, *item_id, *value, game_state, round, current_tick)
        }

        _ => round.clone(),
    }
}

fn change_setting(
    user_id: UserId,
    item_id: ItemId,
    value: u8,
    round_state: &RoundState,
    round: &RocketJamRound,
    current_tick: i32,
) -> RocketJamRound {
    let mut new_state = 0;
    let mut instructions_executed = round_state.instructions_executed;
    let items = round_state
        .items
        .iter()
        .map(|item| {
            if item.id == item_id && item.user_id == user_id {
                new_state = value;
                Item {
                    state: new_state,
                    ..item.clone()
                }
            } else {
                item.clone()
            }
        })
        .collect();
    let mut instructions: Vec<Instruction> = Vec::new();
    for instruction in &round_state.instructions {
        if instruction.item_id == item_id && new_state == instruction.state {
            instructions_executed += 1;
            if let Some(instruction) = mk_instructions(instruction.user_id, &items, current_tick) {
                instructions.push(instruction);
            } else {
                error!("Got no instruction");
            }
        } else {
            instructions.push(instruction.clone());
        }
    }

    let new_round_state = RoundState {
        instructions,
        instructions_executed,
        items,
        ..round_state.clone()
    };

    let game = RocketJam::InLevel(new_round_state);
    RocketJamRound {
        game,
        ..round.clone()
    }
}

fn toggle_ready(
    user_id: i32,
    players_ready: &Vec<UserId>,
    round: &RocketJamRound,
    current_tick: i32,
) -> RocketJamRound {
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
        if players_ready.len() == (round.players.len() - 1) {
            // everybody is ready
            let mut rng = thread_rng();
            let mut available_items: Vec<(ItemId, String, u8)> = ITEMS
                .to_vec()
                .iter()
                .enumerate()
                .map(|(item_id, label)| (item_id, label.to_string(), rng.gen_range(2, 10)))
                .collect();

            available_items.shuffle(&mut rng);

            let items: Vec<Item> = round
                .players
                .clone()
                .into_iter()
                .map(|user_id| mk_items(user_id, 4, &mut available_items))
                .flatten()
                .collect();
            let instructions: Vec<Instruction> = round
                .players
                .iter()
                .map(|user_id| mk_instructions(user_id.clone(), &items, current_tick))
                .flatten()
                .collect();
            let game = RocketJam::InLevel(RoundState {
                items,
                available_items,
                instructions,
                instructions_executed: 0,
                instructions_missed: 0,
            });
            RocketJamRound {
                game,
                ..round.clone()
            }
        } else {
            players_ready.push(user_id);
            new_round.game = RocketJam::InLobby { players_ready };
            new_round
        }
    }
}

fn mk_instructions(user_id: UserId, items: &Vec<Item>, current_tick: i32) -> Option<Instruction> {
    let mut rng = thread_rng();
    let mut items: Vec<Item> = items.clone();
    items.retain(|i| i.user_id != user_id);
    items.shuffle(&mut rng);

    items
        .into_iter()
        .take(1)
        .map(|i| {
            let new_state = if i.state == 0 { 1 } else { 0 };
            Instruction {
                item_id: i.id,
                user_id,
                state: new_state,
                eol_tick: current_tick + 5,
            }
        })
        .next()
}

fn mk_items(
    user_id: UserId,
    count: usize,
    available_items: &mut Vec<(ItemId, String, u8)>,
) -> Vec<Item> {
    let mut items = Vec::new();
    for _i in 0..count {
        if let Some((item_id, label, max_value)) = available_items.pop() {
            let item = Item {
                id: item_id,
                label,
                state: 0,
                user_id,
                max_value,
            };
            items.push(item);
        } else {
            warn!("Ran out of available items");
        }
    }
    items
}
