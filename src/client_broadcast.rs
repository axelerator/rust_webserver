use std::sync::Arc;
use tokio::sync::{mpsc::UnboundedSender, RwLock};
use serde::{Deserialize, Serialize};
use crate::{app::ToClient};
use crate::user::{UserId};
use std::{collections::HashMap};
use log::{info, warn};

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
pub type ClientMessage = (UserId, ToClient);

pub type ClientsByToken = Arc<RwLock<HashMap<String, Client>>>;

#[derive(Clone)]
pub struct ClientBroadcaster {
    clients_by_token: Arc<RwLock<HashMap<String, Client>>>,
}

impl ClientBroadcaster {
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

        if senders_for_user.clone().count().eq(&0) {
            warn!("No clients for user {:?} to send response to", &user_id);
        }

        senders_for_user.for_each(|(token, sender)| {
            info!(
                "Sending to client {:?} ({:?}: {:?})",
                &token, &user_id, &to_client
            );
            let send_result = sender.send(ToClientEnvelope::AppMsg(to_client.clone()));
            if let Err(e) = send_result {
                warn!("Cannot send {:?}", e);
            }
        });
    }

    pub async fn get_user(&self, token: &String) -> Option<Client> {
        let clients_by_token = self.clients_by_token.read().await;
        let opt_client = clients_by_token.get(token);
        if let Some(client) = opt_client {
            Some(client.clone())
        } else {
            None
        }
    }

    pub async fn register_user(&self, token: &String, client: Client) {
        let mut clients_by_token = self.clients_by_token.write().await;
        clients_by_token.insert(token.to_string(), client);
    }
}
