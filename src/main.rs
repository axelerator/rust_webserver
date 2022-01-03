use serde::{Serialize, Deserialize};
use warp::Filter;
use warp::reply::Json;

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


#[tokio::main]
async fn main() {
    // GET /hello/warp => 200 OK with body "Hello, warp!"
    let hello = warp::path!("hello" / String)
        .map(|name| format!("Hello, {}!", name));

    let login = warp::path("login")
        .and(warp::body::content_length_limit(1024 * 16))
        .and(warp::body::json())
        .map(|login: Login| auth(&login));

    let get_routes = warp::get().and(hello);
    let post_routes = warp::post().and(login);


    warp::serve(get_routes.or(post_routes))
        .run(([127, 0, 0, 1], 3030))
        .await;
}

fn auth(login: &Login) -> Json {
    let login_response =
        match find_user_by_username(&login.username, &login.password) {
            Some(user) => LoginResponse::Success(user.username),
            None => LoginResponse::Failure("not found".to_string()),
        };
    warp::reply::json(&login_response)
}

#[derive(Clone)]
struct User {
    username: String,
    hashed_password: String,
}

fn find_user_by_username(username: &String, password: &String) -> Option<User> {
    let users:Vec<User> =
     [ User { username: "at".to_string(), hashed_password: "aa".to_string() }
     , User { username: "rb".to_string(), hashed_password: "rb".to_string() } ].to_vec();

    match users.iter().find(|u| u.username.eq(username)) {
       Some(user) => if user.hashed_password.eq(password) { Some(user.clone()) } else { None },
       None => None
    }
}
