use crate::{app::ToBackend, env::Env};
use log::{error, info};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ToBackendEnvelope {
    token: String,
    to_backend: ToBackend,
}

pub struct Processor {
    env: Env,
    receiver: tokio::sync::mpsc::Receiver<ToBackendEnvelope>,
}

impl Processor {
    pub fn new(env: Env, receiver: tokio::sync::mpsc::Receiver<ToBackendEnvelope>) -> Self {
        Processor { env, receiver }
    }

    pub fn start_loop(mut self) {
        tokio::spawn(async move {
            while let Some(action) = self.receiver.recv().await {
                info!("Processing action {:?}", action);
                if let Some(client) = self.env.client_broadcaster.get(&action.token).await {
                    let user_by_id = self.env.user_service.find_user(client.user_id).await;
                    match user_by_id {
                        None => error!("Client references missing user {:?}", client.user_id),
                        Some(user) => {
                            let to_clients = self.env.app.update(&user, action.to_backend);
                            for client_message in to_clients.await {
                                self.env
                                    .client_broadcaster
                                    .send_to_user(client_message)
                                    .await;
                            }
                        }
                    }
                } else {
                    error!("Couldn't find client for token {:?}", action.token);
                }
            }
            info!("I'm done here.");
        });
    }
}
