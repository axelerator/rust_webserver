use log::info;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::RwLock;
use tokio::time::sleep;
use warp::{Filter, Future};

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
        .then(|name: String, env: Env| async move {
            env.users.write().await.push(name.clone());
            info!("User {:?} connected", name);
            Ok(name)
        });

    let thread_env = env.clone();

    info!("Starting thread");
    tokio::spawn(async { ticker(thread_env).await });

    warp::serve(hello).run(([127, 0, 0, 1], 3030)).await;
}

async fn ticker(thread_env: Env) {
    loop {
        sleep(Duration::from_secs(3)).await;
        let users = thread_env.users.read().await;
        let mut ages = thread_env.ages.write().await;
        for user in users.iter() {
            let previous_age = ages.get(user);
            let x = bar().await;
            let new_age = match previous_age {
                None => 0,
                Some(i) => i + x,
            };
            info!("tick for user {:?} {:?}", user, new_age);
            ages.insert(user.clone(), new_age);
        }
    }
}
//
// `foo()` returns a type that implements `Future<Output = usize>`.
// `foo().await` will result in a value of type `usize`.
async fn foo() -> usize {
    5
}

fn bar() -> impl Future<Output = usize> {
    // This `async` block results in a type that implements
    // `Future<Output = usize>`.
    async {
        let x: usize = foo().await;
        x + 5
    }
}

fn with_env(env: Env) -> impl Filter<Extract = (Env,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || env.clone())
}
