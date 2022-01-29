use std::{collections::HashMap, sync::Arc};

use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tokio::sync::{mpsc::UnboundedSender, RwLock};

use crate::{
    app::{ClientMessage, Model, ToClient},
    user::UserServiceImpl,
};

use log::{warn};

#[derive(Clone)]
pub struct Env {
    pub pool: PgPool,
    pub client_broadcaster: ClientBroadcaster,
    pub model: Arc<RwLock<Model>>,
    pub user_service: UserServiceImpl,
}

#[derive(Debug, Clone)]
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

#[derive(Clone)]
pub struct ClientBroadcaster {
    clients_by_token: std::sync::Arc<RwLock<HashMap<String, Client>>>,
}

impl ClientBroadcaster {
    pub fn new() -> Self {
        ClientBroadcaster {
            clients_by_token: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get(&self, token: &String) -> Option<Client> {
        let map = self.clients_by_token.read().await;
        map.get(token).map(|client| client.clone())
    }

    pub async fn update_client(&self, token: String, client: Client) {
        let mut registry = self.clients_by_token.write().await;
        registry.insert(token, client);
    }

    pub async fn send_to_user(&self, (user_id, to_client): ClientMessage) {
        let clients_by_token = self.clients_by_token.read().await;
        let senders_for_user = clients_by_token
            .values()
            .filter(|c| c.user_id == user_id)
            .map(|c| match &c.sender {
                Some(sender) => Some((&c.token, sender)),
                None => None,
            })
            .flatten();
        if senders_for_user.clone().count() == 0 {
            warn!("No clients for user {:?} to send response to", &user_id);
        }

        senders_for_user.for_each(|(_, sender)| {
            let send_result = sender.send(ToClientEnvelope::AppMsg(to_client.clone()));
            if let Err(e) = send_result {
                warn!("Cannot send {:?}", e);
            }
        });
    }
}
