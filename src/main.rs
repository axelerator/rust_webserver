use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgExecutor, PgPool, Pool, Postgres};
use warp::log::Log;
use warp::reply::Json;
use warp::{Filter, Rejection, Reply};
use std::sync::mpsc::{sync_channel, SyncSender};

#[derive(Serialize, Deserialize)]
struct Login {
    username: String,
    password: String,
}

#[derive(Serialize, Deserialize)]
enum LoginResponse {
    Success(String),
    Failure(String),
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

fn with_db(
    pool: PgPool,
) -> impl Filter<Extract = (PgPool,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || pool.clone())
}

#[tokio::main]
async fn main() {
    let (sender, receiver) = sync_channel::<ClientAction>(3); 

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect("postgres://rust@localhost/rust_server")
        .await
        .unwrap();

    let static_files = warp::any().and(warp::fs::dir("client"));

    let login = warp::path("login")
        .and(warp::body::content_length_limit(1024 * 16))
        .and(with_db(pool))
        .and(warp::body::json())
        .and_then(move |local_pool: PgPool, login: Login| auth_handler(local_pool, login));

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

async fn auth_handler(
    ext_pool: PgPool,
    login: Login,
) -> std::result::Result<impl Reply, Rejection> {
    let user = sqlx::query_as::<_, User>(
        "SELECT username, hashed_password FROM users WHERE username = $1",
    )
    .bind(login.username)
    .fetch_one(&ext_pool)
    .await;

    let login_response = match user {
        Ok(user) => {
            if user.hashed_password.eq(&login.password) {
                LoginResponse::Success(user.username)
            } else {
                LoginResponse::Failure("wrong pw".to_string())
            }
        }
        Err(_) => LoginResponse::Failure("not found".to_string()),
    };
    Ok(warp::reply::json(&login_response))
}

async fn action_handler(sender: SyncSender<ClientAction>, action: ClientAction) -> std::result::Result<impl Reply, Rejection> {
    sender.send(action.clone()).unwrap();
    Ok(warp::reply::json(&action))
}

#[derive(Clone, sqlx::FromRow)]
struct User {
    username: String,
    hashed_password: String,
}

fn find_user_by_username_and_password(username: &String, password: &String) -> Option<User> {
    let users: Vec<User> = [
        User {
            username: "at".to_string(),
            hashed_password: "aa".to_string(),
        },
        User {
            username: "rb".to_string(),
            hashed_password: "rb".to_string(),
        },
    ]
    .to_vec();

    let res = match users.iter().find(|u| u.username.eq(username)) {
        Some(user) => {
            if user.hashed_password.eq(password) {
                Some(user.clone())
            } else {
                None
            }
        }
        None => None,
    };
    res
}
