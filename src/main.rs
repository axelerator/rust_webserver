mod app;
mod env;
mod user;

use env::{Client, Env, ToClientEnvelope};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
//use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::time::Duration;
use tokio::{
    sync::{mpsc::Sender, RwLock, RwLockReadGuard},
    time::sleep,
};
use warp::{Filter, Rejection, Reply};

use std::collections::HashMap;
use std::sync::Arc;

use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::sse::Event;

use uuid::Uuid;

use app::{init_model, ClientMessage, RocketJamApp, ToBackend};
use log::{error, info, warn};

use crate::user::UserServiceImpl;

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
    let (sender, mut receiver) = tokio::sync::mpsc::channel::<ToBackendEnvelope>(32);

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect("postgres://rust@localhost/rust_server")
        .await
        .unwrap();

    let env = Env {
        pool: pool.clone(),
        clients_by_token: Arc::new(RwLock::new(HashMap::new())),
        model: Arc::new(RwLock::new(init_model())),
        user_service: UserServiceImpl::new(&pool),
    };

    let model2 = env.model.clone();
    let clients_by_token2 = env.clients_by_token.clone();
    tokio::spawn(async move {
        loop {
            sleep(Duration::from_secs(1)).await;
            let msgs = RocketJamApp::tick(&model2).await;
            let clients_by_token = clients_by_token2.read().await;
            for client_message in msgs {
                send_to_user(client_message, &clients_by_token);
            }
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
    tokio::spawn(async move {
        while let Some(action) = receiver.recv().await {
            info!("Processing action {:?}", action);
            let clients_by_token = env.clients_by_token.read().await;
            if let Some(client) = clients_by_token.get(&action.token) {
                let user_by_id = env.user_service.find_user(client.user_id).await;
                match user_by_id {
                    None => error!("Client references missing user {:?}", client.user_id),
                    Some(user) => {
                        let to_clients = RocketJamApp::update(&user, &env.model, action.to_backend);
                        for client_message in to_clients.await {
                            send_to_user(client_message, &clients_by_token);
                        }
                    }
                }
            } else {
                error!("Couldn't find client for token {:?}", action.token);
            }
        }
        info!("I'm done here.");
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
        let send_result = sender.send(ToClientEnvelope::AppMsg(to_client.clone()));
        if let Err(e) = send_result {
            warn!("Cannot send {:?}", e);
        }
    });
}

async fn auth_handler(env: Env, login: Login) -> std::result::Result<impl Reply, Rejection> {
    let user = env
        .user_service
        .find_user_by_name_and_password(&login.username, &login.password)
        .await;
    let login_response = match user {
        Some(user) => {
            if user.hashed_password.eq(&login.password) {
                //let mut map = env.clients_by_token.lock().unwrap();
                let map = env.clients_by_token.write();
                let token = Uuid::new_v4();

                let client = Client {
                    token: token.to_string(),
                    user_id: user.id,
                    sender: None,
                };

                map.await.insert(token.to_string(), client);

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
        _ => LoginResponse::Failure(LoginFailureDetails {
            msg: "not found".to_string(),
        }),
    };
    Ok(warp::reply::json(&login_response))
}

async fn action_handler(
    sender: Sender<ToBackendEnvelope>,
    action: ToBackendEnvelope,
) -> std::result::Result<impl Reply, Rejection> {
    info!("Received action {:?}", action);
    // should probably do auth & resolution to user already here?
    sender.send(action.clone()).await.unwrap();
    Ok(warp::reply::json(&action))
}

async fn event_handler(token: String, env: Env) -> std::result::Result<impl Reply, Rejection> {
    let mut clients_by_token = env.clients_by_token.write().await;
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
            info!("Sending event to client {:?}", to_client);
            let r: Result<Event, warp::Error> = Ok(Event::default().json_data(to_client).unwrap());
            r
        });
        Ok(warp::sse::reply(event_stream))
    } else {
        Err(warp::reject::not_found())
    }
}
