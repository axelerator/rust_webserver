use std::{collections::HashMap, sync::Arc};

use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tokio::sync::{mpsc::UnboundedSender, RwLock};

use crate::{app::ToClient, user::UserServiceImpl};

#[derive(Clone)]
pub struct Env {
    pub pool: PgPool,
    pub user_service: UserServiceImpl,
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

