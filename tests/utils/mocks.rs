use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

use bigtwo::websockets::ConnectionManager;

// ============================================================================
// Mock Infrastructure
// ============================================================================

#[derive(Clone)]
pub struct MockConnectionManager {
    sent_messages: Arc<RwLock<HashMap<String, Vec<String>>>>,
    connected_players: Arc<RwLock<Vec<String>>>,
}

impl MockConnectionManager {
    pub fn new() -> Self {
        Self {
            sent_messages: Arc::new(RwLock::new(HashMap::new())),
            connected_players: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn add_connected_player(&self, username: &str) {
        self.connected_players
            .write()
            .await
            .push(username.to_string());
    }

    pub async fn get_messages_for(&self, username: &str) -> Vec<String> {
        self.sent_messages
            .read()
            .await
            .get(username)
            .cloned()
            .unwrap_or_default()
    }

    pub async fn clear_messages(&self) {
        self.sent_messages.write().await.clear();
    }
}

#[async_trait]
impl ConnectionManager for MockConnectionManager {
    async fn add_connection(&self, username: String, _sender: mpsc::UnboundedSender<String>) {
        self.add_connected_player(&username).await;
    }

    async fn remove_connection(&self, username: &str) {
        self.connected_players
            .write()
            .await
            .retain(|p| p != username);
    }

    async fn send_to_player(&self, username: &str, message: &str) {
        self.sent_messages
            .write()
            .await
            .entry(username.to_string())
            .or_default()
            .push(message.to_string());
    }

    async fn send_to_players(&self, usernames: &[String], message: &str) {
        for username in usernames {
            self.send_to_player(username, message).await;
        }
    }
}
