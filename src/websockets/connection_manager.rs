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
    async fn add_connection(&self, username: String, sender: mpsc::UnboundedSender<String>);

    async fn remove_connection(&self, username: &str);

    async fn send_to_player(&self, username: &str, message: &str);

    async fn send_to_players(&self, usernames: &[String], message: &str);
}

pub struct InMemoryConnectionManager {
    // username -> sender
    connections: Arc<RwLock<HashMap<String, mpsc::UnboundedSender<String>>>>,
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
    async fn add_connection(&self, username: String, sender: mpsc::UnboundedSender<String>) {
        let mut connections = self.connections.write().await;
        connections.insert(username, sender);
    }

    async fn remove_connection(&self, username: &str) {
        let mut connections = self.connections.write().await;
        connections.remove(username);
    }

    async fn send_to_player(&self, username: &str, message: &str) {
        let connections = self.connections.read().await;
        if let Some(sender) = connections.get(username) {
            let _ = sender.send(message.to_string());
        }
    }

    async fn send_to_players(&self, usernames: &[String], message: &str) {
        let connections = self.connections.read().await;
        for username in usernames {
            if let Some(sender) = connections.get(username) {
                let _ = sender.send(message.to_string());
            }
        }
    }
}
