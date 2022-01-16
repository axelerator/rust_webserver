use log::info;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::Duration,
};
use warp::Filter;

#[derive(Clone)]
struct Env {
    users: Arc<RwLock<Vec<String>>>,
    ages: Arc<RwLock<HashMap<String, usize>>>,
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let env = Env {
        users: Arc::new(RwLock::new(Vec::new())),
        ages: Arc::new(RwLock::new(HashMap::new())),
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
    std::thread::spawn(move || ticker(thread_env));

    warp::serve(hello).run(([127, 0, 0, 1], 3030)).await;
}

fn ticker(thread_env: Env) {
    loop {
        std::thread::sleep(Duration::from_secs(3));
        let users = thread_env.users.read().unwrap();
        let mut ages = thread_env.ages.write().unwrap();
        for user in users.iter() {
            let previous_age = ages.get(user);
            let new_age = match previous_age {
                None => 0,
                Some(i) => i + 1,
            };
            info!("tick for user {:?} {:?}", user, new_age);
            ages.insert(user.clone(), new_age);
        }
    }
}

fn with_env(env: Env) -> impl Filter<Extract = (Env,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || env.clone())
}
