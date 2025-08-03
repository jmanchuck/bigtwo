use std::sync::Arc;
use tokio::task::JoinHandle;

use bigtwo::{
    event::{EventBus, RoomSubscription},
    game::GameManager,
    room::{
        models::RoomModel,
        repository::{InMemoryRoomRepository, RoomRepository},
    },
    websockets::{WebSocketRoomSubscriber, WebsocketReceiveHandler},
};

use super::mocks::MockConnectionManager;

// ============================================================================
// Test Setup Infrastructure
// ============================================================================

pub struct TestSetup {
    pub event_bus: EventBus,
    pub mock_conn_manager: Arc<MockConnectionManager>,
    pub input_handler: WebsocketReceiveHandler,
    pub game_manager: Arc<GameManager>,
    pub players: Vec<String>,
    pub _subscription_handle: JoinHandle<()>,
}

pub struct TestSetupBuilder {
    players: Vec<String>,
    room_id: String,
}

impl TestSetupBuilder {
    pub fn new() -> Self {
        Self {
            players: vec![],
            room_id: "room-123".to_string(),
        }
    }

    pub fn with_players(mut self, players: Vec<&str>) -> Self {
        self.players = players.into_iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn with_two_players(self) -> Self {
        self.with_players(vec!["alice", "bob"])
    }

    pub fn with_four_players(self) -> Self {
        self.with_players(vec!["alice", "bob", "charlie", "david"])
    }

    pub async fn build(self) -> TestSetup {
        let event_bus = EventBus::new();
        let repo = Arc::new(InMemoryRoomRepository::new());
        let mock_conn_manager = Arc::new(MockConnectionManager::new());
        let game_manager = Arc::new(GameManager::new());

        // Create room
        let room = RoomModel {
            id: self.room_id.clone(),
            host_name: self
                .players
                .first()
                .cloned()
                .unwrap_or_else(|| "host".to_string()),
            status: "ONLINE".to_string(),
            players: self.players.clone(),
        };
        repo.create_room(&room).await.unwrap();

        // Connect players
        for player in &self.players {
            mock_conn_manager.add_connected_player(player).await;
        }

        let input_handler = WebsocketReceiveHandler::new(event_bus.clone());

        let output_subscriber = WebSocketRoomSubscriber::new(
            repo.clone(),
            mock_conn_manager.clone(),
            game_manager.clone(),
            event_bus.clone(),
        );

        let subscription = RoomSubscription::new(
            self.room_id.clone(),
            Arc::new(output_subscriber),
            event_bus.clone(),
        );
        let subscription_handle = subscription.start().await;

        TestSetup {
            event_bus,
            mock_conn_manager,
            input_handler,
            game_manager,
            players: self.players,
            _subscription_handle: subscription_handle,
        }
    }
}
