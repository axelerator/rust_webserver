use std::{collections::HashMap, sync::Arc};

use log::error;
use sqlx::PgPool;
use tokio::sync::RwLock;

pub type UserId = i32;

#[derive(Clone, sqlx::FromRow)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub hashed_password: String,
}

#[derive(Clone)]
pub struct UserServiceImpl {
    pool: PgPool,
    user_cache: Arc<RwLock<HashMap<UserId, User>>>,
}

impl UserServiceImpl {
    pub fn new(pool: &PgPool) -> UserServiceImpl {
        UserServiceImpl {
            pool: pool.clone(),
            user_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn find_user(&self, user_id: UserId) -> Option<User> {
        let users_by_id_lock = self.user_cache.read();
        let users_by_id = users_by_id_lock.await;
        let maybe_user: Option<&User> = users_by_id.get(&user_id);
        match maybe_user {
            Some(user) => Some(user.clone()),
            None => {
                let user_query_result = sqlx::query_as::<_, User>(
                    "SELECT id, username, hashed_password FROM users WHERE id = $1",
                )
                .bind(user_id)
                .fetch_one(&self.pool)
                .await;
                match user_query_result {
                    Ok(user) => {
                        // THIS PRODUCES A DEADLOCK
                        //self.user_cache.write().await.insert(user_id, user.clone());
                        Some(user)
                    }
                    _ => {
                        error!("User not found");
                        None
                    }
                }
            }
        }
    }

    pub async fn find_user_by_name_and_password(
        &self,
        username: &String,
        password: &String,
    ) -> Option<User> {
        let user_query_result = sqlx::query_as::<_, User>(
            "SELECT id, username, hashed_password FROM users WHERE username = $1",
        )
        .bind(username)
        .fetch_one(&self.pool)
        .await;
        match user_query_result {
            Ok(user) => {
                if user.hashed_password.eq(password) {
                    Some(user)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}
/*
pub async fn auth_handler(env: &Env, login: Login)  {
    let user = sqlx::query_as::<_, User>(
        "SELECT id, username, hashed_password FROM users WHERE username = $1",
    )
    .bind(login.username)
    .fetch_one(&env.pool)
    .await;
}
*/
