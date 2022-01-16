use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tokio::sync::mpsc::UnboundedSender;

use crate::app::{Model, ToClient};

#[derive(Clone)]
pub struct Env {
    pub pool: PgPool,
    pub clients_by_token: ClientsByToken,
    pub model: Arc<RwLock<Model>>,
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

#[derive(Debug)]
pub struct Client {
    pub token: String,
    pub user_id: i32,
    pub sender: Option<UnboundedSender<ToClientEnvelope>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ToClientEnvelope {
    SuperSeeded(),
    AppMsg(ToClient),
}

pub type ClientsByToken = Arc<RwLock<HashMap<String, Client>>>;
