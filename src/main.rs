mod app;
mod env;
mod user;

use env::{Client, Env, ToClientEnvelope};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::postgres::PgPoolOptions;
use std::sync::mpsc::{sync_channel, SyncSender};
use std::time::Duration;
use user::User;
use warp::{Filter, Rejection, Reply};

use std::collections::HashMap;
use std::sync::{Arc, RwLock, RwLockReadGuard};

use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::sse::Event;

use uuid::Uuid;

use app::{init_model, ClientMessage, RocketJamApp, ToBackend};
use log::{info, warn};

#[derive(Serialize, Deserialize)]
struct Login {
    username: String,
    password: String,
}

#[derive(Serialize, Deserialize)]
enum LoginResponse {
    Success(LoginSuccessDetails),
    Failure(LoginFailureDetails),
}

#[derive(Serialize, Deserialize)]
struct LoginSuccessDetails {
    token: String,
    username: String,
}

#[derive(Serialize, Deserialize)]
struct LoginFailureDetails {
    msg: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct ToBackendEnvelope {
    token: String,
    to_backend: ToBackend,
}

#[derive(Serialize, Deserialize)]
enum ActionResponse {
    Success(String),
    Failure(String),
}

#[derive(Serialize, Deserialize)]
struct ConnectRequest {
    token: String,
}

fn with_env(env: Env) -> impl Filter<Extract = (Env,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || env.clone())
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let be = ToBackendEnvelope {
        token: "aToken".to_string(),
        to_backend: ToBackend::JoinGame {
            round_id: "foo".to_string(),
        },
    };
    let v = serde_json::to_string(&be);
    println!("json {:?}", v);

    let (sender, receiver) = sync_channel::<ToBackendEnvelope>(3);

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect("postgres://rust@localhost/rust_server")
        .await
        .unwrap();

    let env = Env {
        pool: pool.clone(),
        clients_by_token: Arc::new(RwLock::new(HashMap::new())),
        model: Arc::new(RwLock::new(init_model())),
    };
    let model2 = env.model.clone();
    let clients_by_token2 = env.clients_by_token.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(1));
        let msgs = RocketJamApp::tick(&model2);
        let clients_by_token = clients_by_token2.read().unwrap();
        for client_message in msgs {
            send_to_user(client_message, &clients_by_token);
        }
    });

    let static_files = warp::any().and(warp::fs::dir("client"));

    let login = warp::path("login")
        .and(warp::body::content_length_limit(1024 * 16))
        .and(with_env(env.clone()))
        .and(warp::body::json())
        .and_then(move |env: Env, login: Login| auth_handler(env, login));

    let action = warp::path("action")
        .and(warp::any().map(move || sender.clone()))
        .and(warp::body::json())
        .and_then(action_handler);

    let event_route = warp::path!("events" / String)
        .and(with_env(env.clone()))
        .and_then(event_handler);

    let post_routes = warp::post().and(login.or(action));
    let get_routes = warp::get().and(event_route);
    std::thread::spawn(move || {
        for action in receiver.iter() {
            let clients_by_token = env.clients_by_token.read().unwrap();
            if let Some(client) = clients_by_token.get(&action.token) {
                let to_clients =
                    RocketJamApp::update(client.user_id, &env.model, action.to_backend);
                for client_message in to_clients {
                    send_to_user(client_message, &clients_by_token);
                }
            }
        }
    });

    warp::serve(post_routes.or(static_files).or(get_routes))
        .run(([127, 0, 0, 1], 3030))
        .await;
}

fn send_to_user(
    (user_id, to_client): ClientMessage,
    clients_by_token: &RwLockReadGuard<HashMap<String, Client>>,
) {
    let senders_for_user = clients_by_token
        .values()
        .filter(|c| c.user_id == user_id)
        .map(|c| match &c.sender {
            Some(sender) => Some((&c.token, sender)),
            None => None,
        })
        .flatten();
    if senders_for_user.clone().count().eq(&0) {
        warn!("No clients for user {:?} to send response to", &user_id);
    }

    senders_for_user.for_each(|(token, sender)| {
        info!(
            "Sending to client {:?} ({:?}: {:?})",
            &token, &user_id, &to_client
        );
        sender
            .send(ToClientEnvelope::AppMsg(to_client.clone()))
            .expect("Could not send msg to client");
    });
}

async fn auth_handler(env: Env, login: Login) -> std::result::Result<impl Reply, Rejection> {
    let user = sqlx::query_as::<_, User>(
        "SELECT id, username, hashed_password FROM users WHERE username = $1",
    )
    .bind(login.username)
    .fetch_one(&env.pool)
    .await;

    let login_response = match user {
        Ok(user) => {
            if user.hashed_password.eq(&login.password) {
                //let mut map = env.clients_by_token.lock().unwrap();
                let mut map = env.clients_by_token.write().expect("RwLock poisoned");
                let token = Uuid::new_v4();

                let client = Client {
                    token: token.to_string(),
                    user_id: user.id,
                    sender: None,
                };

                map.insert(token.to_string(), client);

                LoginResponse::Success(LoginSuccessDetails {
                    token: token.to_string(),
                    username: user.username,
                })
            } else {
                LoginResponse::Failure(LoginFailureDetails {
                    msg: "wrong pw".to_string(),
                })
            }
        }
        Err(_) => LoginResponse::Failure(LoginFailureDetails {
            msg: "not found".to_string(),
        }),
    };
    Ok(warp::reply::json(&login_response))
}

async fn action_handler(
    sender: SyncSender<ToBackendEnvelope>,
    action: ToBackendEnvelope,
) -> std::result::Result<impl Reply, Rejection> {
    info!("Received action {:?}", action);
    sender.send(action.clone()).unwrap();
    Ok(warp::reply::json(&action))
}

async fn event_handler(token: String, env: Env) -> std::result::Result<impl Reply, Rejection> {
    let mut clients_by_token = env.clients_by_token.write().unwrap();
    if let Some(client) = clients_by_token.get(&token) {
        // logout previously registered client
        if let Some(sender) = &client.sender {
            if let Err(some_error) = sender.send(ToClientEnvelope::SuperSeeded()) {
                println!(
                    "Can't send SuperSeed but it doesn't matter really {:?}",
                    some_error
                );
            }
        }
        //
        // Use an unbounded channel to handle buffering and flushing of messages
        // to the event source...
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let rx: UnboundedReceiverStream<ToClientEnvelope> = UnboundedReceiverStream::new(rx);

        let updated_client = Client {
            token: client.token.clone(),
            user_id: client.user_id,
            sender: Some(tx),
        };
        clients_by_token.insert(token, updated_client);
        let event_stream = rx.map(|to_client| {
            let r: Result<Event, warp::Error> = Ok(Event::default().json_data(to_client).unwrap());
            r
        });
        Ok(warp::sse::reply(event_stream))
    } else {
        Err(warp::reject::not_found())
    }
}
