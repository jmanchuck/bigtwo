use std::sync::Arc;
use tracing::{error, info, instrument};

use crate::event::{RoomEvent, RoomEventHandler};

use super::activity_tracker::ActivityTracker;

/// Event subscriber that tracks room activity based on room events
pub struct ActivityRoomSubscriber {
    activity_tracker: Arc<ActivityTracker>,
}

impl ActivityRoomSubscriber {
    /// Creates a new activity room subscriber
    pub fn new(activity_tracker: Arc<ActivityTracker>) -> Self {
        Self { activity_tracker }
    }
}

#[async_trait::async_trait]
impl RoomEventHandler for ActivityRoomSubscriber {
    #[instrument(skip(self, event))]
    async fn handle_room_event(
        &self,
        room_id: &str,
        event: RoomEvent,
    ) -> Result<(), crate::event::RoomEventError> {
        // Only track meaningful user interactions, not connection status or heartbeats
        let should_track = matches!(
            &event,
            RoomEvent::PlayerJoined { .. }
                | RoomEvent::PlayerLeft { .. }
                | RoomEvent::ChatMessage { .. }
                | RoomEvent::TryStartGame { .. }
                | RoomEvent::TryPlayMove { .. }
                | RoomEvent::PlayerReadyToggled { .. }
                | RoomEvent::BotAdded { .. }
                | RoomEvent::BotRemoved { .. }
        );

        if should_track {
            info!(
                room_id = %room_id,
                event = ?event,
                "Recording activity for room event"
            );

            if let Err(e) = self.activity_tracker.record_activity(room_id).await {
                error!(
                    room_id = %room_id,
                    error = %e,
                    "Failed to record room activity"
                );
            }
        }

        Ok(())
    }

    fn handler_name(&self) -> &'static str {
        "ActivityRoomSubscriber"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::room::models::RoomModel;
    use crate::room::{
        activity_tracker::ActivityTracker, repository::InMemoryRoomRepository,
        repository::RoomRepository,
    };

    async fn setup_subscriber() -> (Arc<InMemoryRoomRepository>, ActivityRoomSubscriber, String) {
        let repo = Arc::new(InMemoryRoomRepository::new());
        let tracker = Arc::new(ActivityTracker::new(repo.clone()));
        let subscriber = ActivityRoomSubscriber::new(tracker);

        // Create a test room
        let room = RoomModel::new("test-host".to_string());
        let room_id = room.id.clone();
        repo.create_room(&room).await.unwrap();

        (repo, subscriber, room_id)
    }

    #[tokio::test]
    async fn test_player_joined_updates_activity() {
        let (repo, subscriber, room_id) = setup_subscriber().await;

        let initial_activity = repo
            .get_room(&room_id)
            .await
            .unwrap()
            .unwrap()
            .last_activity_at;

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Simulate PlayerJoined event
        let event = RoomEvent::PlayerJoined {
            player: "player1".to_string(),
        };
        subscriber.handle_room_event(&room_id, event).await.unwrap();

        let updated_activity = repo
            .get_room(&room_id)
            .await
            .unwrap()
            .unwrap()
            .last_activity_at;

        assert!(updated_activity > initial_activity);
    }

    #[tokio::test]
    async fn test_chat_message_updates_activity() {
        let (repo, subscriber, room_id) = setup_subscriber().await;

        let initial_activity = repo
            .get_room(&room_id)
            .await
            .unwrap()
            .unwrap()
            .last_activity_at;

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let event = RoomEvent::ChatMessage {
            sender: "player1".to_string(),
            content: "Hello!".to_string(),
        };
        subscriber.handle_room_event(&room_id, event).await.unwrap();

        let updated_activity = repo
            .get_room(&room_id)
            .await
            .unwrap()
            .unwrap()
            .last_activity_at;

        assert!(updated_activity > initial_activity);
    }

    #[tokio::test]
    async fn test_try_play_move_updates_activity() {
        let (repo, subscriber, room_id) = setup_subscriber().await;

        let initial_activity = repo
            .get_room(&room_id)
            .await
            .unwrap()
            .unwrap()
            .last_activity_at;

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let event = RoomEvent::TryPlayMove {
            player: "player1".to_string(),
            cards: vec![],
        };
        subscriber.handle_room_event(&room_id, event).await.unwrap();

        let updated_activity = repo
            .get_room(&room_id)
            .await
            .unwrap()
            .unwrap()
            .last_activity_at;

        assert!(updated_activity > initial_activity);
    }

    #[tokio::test]
    async fn test_heartbeat_does_not_update_activity() {
        let (repo, subscriber, room_id) = setup_subscriber().await;

        let initial_activity = repo
            .get_room(&room_id)
            .await
            .unwrap()
            .unwrap()
            .last_activity_at;

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Heartbeat should NOT update activity
        let event = RoomEvent::HeartbeatReceived {
            player: "player1".to_string(),
        };
        subscriber.handle_room_event(&room_id, event).await.unwrap();

        let updated_activity = repo
            .get_room(&room_id)
            .await
            .unwrap()
            .unwrap()
            .last_activity_at;

        // Activity should not change
        assert_eq!(updated_activity, initial_activity);
    }

    #[tokio::test]
    async fn test_player_connected_does_not_update_activity() {
        let (repo, subscriber, room_id) = setup_subscriber().await;

        let initial_activity = repo
            .get_room(&room_id)
            .await
            .unwrap()
            .unwrap()
            .last_activity_at;

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Connection events should NOT update activity
        let event = RoomEvent::PlayerConnected {
            player: "player1".to_string(),
        };
        subscriber.handle_room_event(&room_id, event).await.unwrap();

        let updated_activity = repo
            .get_room(&room_id)
            .await
            .unwrap()
            .unwrap()
            .last_activity_at;

        assert_eq!(updated_activity, initial_activity);
    }

    #[tokio::test]
    async fn test_player_disconnected_does_not_update_activity() {
        let (repo, subscriber, room_id) = setup_subscriber().await;

        let initial_activity = repo
            .get_room(&room_id)
            .await
            .unwrap()
            .unwrap()
            .last_activity_at;

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let event = RoomEvent::PlayerDisconnected {
            player: "player1".to_string(),
        };
        subscriber.handle_room_event(&room_id, event).await.unwrap();

        let updated_activity = repo
            .get_room(&room_id)
            .await
            .unwrap()
            .unwrap()
            .last_activity_at;

        assert_eq!(updated_activity, initial_activity);
    }

    #[tokio::test]
    async fn test_bot_added_updates_activity() {
        let (repo, subscriber, room_id) = setup_subscriber().await;

        let initial_activity = repo
            .get_room(&room_id)
            .await
            .unwrap()
            .unwrap()
            .last_activity_at;

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let event = RoomEvent::BotAdded {
            bot_uuid: "bot-123".to_string(),
            bot_name: "Bot".to_string(),
        };
        subscriber.handle_room_event(&room_id, event).await.unwrap();

        let updated_activity = repo
            .get_room(&room_id)
            .await
            .unwrap()
            .unwrap()
            .last_activity_at;

        assert!(updated_activity > initial_activity);
    }

    #[tokio::test]
    async fn test_ready_toggle_updates_activity() {
        let (repo, subscriber, room_id) = setup_subscriber().await;

        let initial_activity = repo
            .get_room(&room_id)
            .await
            .unwrap()
            .unwrap()
            .last_activity_at;

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let event = RoomEvent::PlayerReadyToggled {
            player: "player1".to_string(),
            is_ready: true,
        };
        subscriber.handle_room_event(&room_id, event).await.unwrap();

        let updated_activity = repo
            .get_room(&room_id)
            .await
            .unwrap()
            .unwrap()
            .last_activity_at;

        assert!(updated_activity > initial_activity);
    }
}
