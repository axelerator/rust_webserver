use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::sync::mpsc::{sync_channel, SyncSender};
use warp::{Filter, Rejection, Reply};

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

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
}

#[derive(Serialize, Deserialize)]
struct LoginFailureDetails {
    msg: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
enum ClientAction {
    Ping(u32),
}

#[derive(Serialize, Deserialize)]
enum ActionResponse {
    Success(String),
    Failure(String),
}

struct Client {
    token: String,
    user_id: i32,
}

type ClientsByToken = Arc<Mutex<HashMap<String, Client>>>;

#[derive(Clone)]
struct Env {
    pool: PgPool,
    clientsByToken: ClientsByToken,
}

fn with_env(env: Env) -> impl Filter<Extract = (Env,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || env.clone())
}

#[tokio::main]
async fn main() {
    let (sender, receiver) = sync_channel::<ClientAction>(3);

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect("postgres://rust@localhost/rust_server")
        .await
        .unwrap();

    let env = Env {
        pool: pool.clone(),
        clientsByToken: Arc::new(Mutex::new(HashMap::new())),
    };
    let static_files = warp::any().and(warp::fs::dir("client"));

    let login = warp::path("login")
        .and(warp::body::content_length_limit(1024 * 16))
        .and(with_env(env))
        .and(warp::body::json())
        .and_then(move |env: Env, login: Login| auth_handler(env, login));

    let action = warp::path("action")
        .and(warp::any().map(move || sender.clone()))
        .and(warp::body::json())
        .and_then(action_handler);

    let post_routes = warp::post().and(login.or(action));

    std::thread::spawn(move || {
        for action in receiver.iter() {
            println!("Action {:?}", action);
            std::thread::sleep(std::time::Duration::from_millis(1000));
        }
    });

    warp::serve(post_routes.or(static_files))
        .run(([127, 0, 0, 1], 3030))
        .await;
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
                let mut map = env.clientsByToken.lock().unwrap();
                let token = "aToken".to_string();
                let client = Client {
                    token: token.clone(),
                    user_id: user.id,
                };

                map.insert(token.clone(), client);

                LoginResponse::Success(LoginSuccessDetails { token })
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
    sender: SyncSender<ClientAction>,
    action: ClientAction,
) -> std::result::Result<impl Reply, Rejection> {
    sender.send(action.clone()).unwrap();
    Ok(warp::reply::json(&action))
}

#[derive(Clone, sqlx::FromRow)]
struct User {
    id: i32,
    username: String,
    hashed_password: String,
}
