use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

/// Maps player to their outbound channel
/// Used by upstream components to send messages to players
/// The sender is a channel that directs into the Connection struct
/// The owned sender is called the outbound sender
#[async_trait]
pub trait ConnectionManager: Send + Sync {
    async fn add_connection(&self, uuid: String, sender: mpsc::UnboundedSender<String>);

    async fn remove_connection(&self, uuid: &str);

    async fn send_to_player(&self, uuid: &str, message: &str);

    async fn send_to_players(&self, uuids: &[String], message: &str);
}

pub struct InMemoryConnectionManager {
    // uuid -> sender
    connections: Arc<RwLock<HashMap<String, mpsc::UnboundedSender<String>>>>,
}

impl Default for InMemoryConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryConnectionManager {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl ConnectionManager for InMemoryConnectionManager {
    async fn add_connection(&self, uuid: String, sender: mpsc::UnboundedSender<String>) {
        let mut connections = self.connections.write().await;

        // If there's an existing connection for this username, close it first
        if let Some(existing_sender) = connections.insert(uuid.clone(), sender) {
            // Drop the existing sender to close the connection
            drop(existing_sender);
            tracing::info!(uuid = %uuid, "Replaced existing WebSocket connection");
        } else {
            tracing::info!(uuid = %uuid, "Added new WebSocket connection");
        }
    }

    async fn remove_connection(&self, uuid: &str) {
        let mut connections = self.connections.write().await;
        connections.remove(uuid);
    }

    async fn send_to_player(&self, uuid: &str, message: &str) {
        let connections = self.connections.read().await;
        if let Some(sender) = connections.get(uuid) {
            let _ = sender.send(message.to_string());
        }
    }

    async fn send_to_players(&self, uuids: &[String], message: &str) {
        let connections = self.connections.read().await;
        for uuid in uuids {
            if let Some(sender) = connections.get(uuid) {
                let _ = sender.send(message.to_string());
            }
        }
    }
}
