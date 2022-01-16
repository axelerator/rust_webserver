use log::info;
use std::{
    sync::{Arc, RwLock},
    time::Duration,
};
use warp::Filter;

#[derive(Clone)]
struct Env {
    users: Arc<RwLock<Vec<String>>>,
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let env = Env {
        users: Arc::new(RwLock::new(Vec::new())),
    };
    // GET /hello/warp => 200 OK with body "Hello, warp!"
    let hello = warp::path!("hello" / String)
        .and(with_env(env.clone()))
        .map(|name: String, env: Env| {
            env.users.write().unwrap().push(name.clone());
            info!("User {:?} connected", name);
            format!("Hello, {}!", name)
        });

    let thread_env = env.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(3));
        let users = thread_env.users.read().unwrap();
        for user in users.iter() {
            info!("tick for user {:?}", user);
        }
    });

    warp::serve(hello).run(([127, 0, 0, 1], 3030)).await;
}

fn with_env(env: Env) -> impl Filter<Extract = (Env,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || env.clone())
}
